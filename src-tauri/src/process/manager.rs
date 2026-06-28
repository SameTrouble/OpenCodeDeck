use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use tokio::process::Child;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ProcessStateKind {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessState {
    pub state: ProcessStateKind,
    pub pid: Option<u32>,
    pub started_at: Option<i64>,
    pub uptime_sec: Option<u64>,
    pub exit_code: Option<i32>,
    pub healthy: Option<bool>,
}

impl Default for ProcessState {
    fn default() -> Self {
        Self {
            state: ProcessStateKind::Stopped,
            pid: None, started_at: None, uptime_sec: None, exit_code: None, healthy: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessTarget {
    Server,
    Bridge,
}

pub(crate) struct ManagedProcess {
    pub state: ProcessState,
    pub child: Option<Child>,
    pub started_at_instant: Option<Instant>,
}

impl ManagedProcess {
    fn new() -> Self {
        Self { state: ProcessState::default(), child: None, started_at_instant: None }
    }
}

pub type StateCallback = Arc<dyn Fn(ProcessTarget, ProcessState) + Send + Sync>;
pub type LogCallback = Arc<dyn Fn(crate::monitor::LogEntry) + Send + Sync>;
pub type QrCallback = Arc<dyn Fn(crate::monitor::stdout_parser::WechatQrEvent) + Send + Sync>;

pub struct ProcessManager {
    server: Arc<Mutex<ManagedProcess>>,
    bridge: Arc<Mutex<ManagedProcess>>,
    on_state: StateCallback,
    on_log: LogCallback,
    on_qr: QrCallback,
    runtime: tokio::runtime::Runtime,
}

impl ProcessManager {
    pub fn new(on_state: StateCallback, on_log: LogCallback, on_qr: QrCallback) -> Self {
        Self {
            server: Arc::new(Mutex::new(ManagedProcess::new())),
            bridge: Arc::new(Mutex::new(ManagedProcess::new())),
            on_state,
            on_log,
            on_qr,
            runtime: tokio::runtime::Runtime::new().expect("failed to create tokio runtime"),
        }
    }

    fn target_ref(&self, target: ProcessTarget) -> &Arc<Mutex<ManagedProcess>> {
        match target {
            ProcessTarget::Server => &self.server,
            ProcessTarget::Bridge => &self.bridge,
        }
    }

    fn emit_state(&self, target: ProcessTarget) {
        let state = self.target_ref(target).lock().unwrap().state.clone();
        (self.on_state)(target, state);
    }

    fn now_ts() -> i64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
    }

    pub fn start_server(&self, port: u16, cwd: &str, extra_env: &std::collections::HashMap<String, String>) -> AppResult<ProcessState> {
        {
            let mut mp = self.server.lock().unwrap();
            if mp.state.state == ProcessStateKind::Running || mp.state.state == ProcessStateKind::Starting {
                return Err(AppError::Process("server already running".into()));
            }
            mp.state = ProcessState { state: ProcessStateKind::Starting, ..Default::default() };
        }
        self.emit_state(ProcessTarget::Server);

        let mut cmd = tokio::process::Command::new("opencode");
        cmd.arg("serve").arg("--port").arg(port.to_string());
        cmd.current_dir(if cwd.is_empty() { "." } else { cwd });
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.envs(extra_env.iter());
        cmd.kill_on_drop(true);

        let child = cmd.spawn().map_err(|e| AppError::Process(format!("failed to spawn opencode: {}", e)))?;
        let pid = child.id();
        let now = Self::now_ts();
        let instant = Instant::now();

        {
            let mut mp = self.server.lock().unwrap();
            mp.child = Some(child);
            mp.started_at_instant = Some(instant);
            mp.state = ProcessState {
                state: ProcessStateKind::Running,
                pid,
                started_at: Some(now),
                uptime_sec: Some(0),
                exit_code: None,
                healthy: None,
            };
        }
        self.emit_state(ProcessTarget::Server);

        let on_log = self.on_log.clone();
        let on_state = self.on_state.clone();
        let server_ref = self.server.clone();
        self.runtime.spawn(async move {
            super::supervisor::supervise(server_ref, ProcessTarget::Server, on_log, on_state).await;
        });

        Ok(self.server.lock().unwrap().state.clone())
    }

    pub fn start_bridge(&self, bridge_dir: &std::path::Path, use_bun: bool) -> AppResult<ProcessState> {
        {
            let mut mp = self.bridge.lock().unwrap();
            if mp.state.state == ProcessStateKind::Running || mp.state.state == ProcessStateKind::Starting {
                return Err(AppError::Process("bridge already running".into()));
            }
            mp.state = ProcessState { state: ProcessStateKind::Starting, ..Default::default() };
        }
        self.emit_state(ProcessTarget::Bridge);

        let mut cmd = if use_bun {
            let mut c = tokio::process::Command::new("bun");
            c.arg("run").arg("src/index.ts");
            c
        } else {
            let mut c = tokio::process::Command::new("npx");
            c.arg("tsx").arg("src/index.ts");
            c
        };
        cmd.current_dir(bridge_dir);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.kill_on_drop(true);

        let child = cmd.spawn().map_err(|e| AppError::Process(format!("failed to spawn bridge: {}", e)))?;
        let pid = child.id();
        let now = Self::now_ts();
        let instant = Instant::now();

        {
            let mut mp = self.bridge.lock().unwrap();
            mp.child = Some(child);
            mp.started_at_instant = Some(instant);
            mp.state = ProcessState {
                state: ProcessStateKind::Running,
                pid,
                started_at: Some(now),
                uptime_sec: Some(0),
                exit_code: None,
                healthy: None,
            };
        }
        self.emit_state(ProcessTarget::Bridge);

        let on_log = self.on_log.clone();
        let on_state = self.on_state.clone();
        let on_qr = self.on_qr.clone();
        let bridge_ref = self.bridge.clone();
        self.runtime.spawn(async move {
            super::supervisor::supervise_with_qr(bridge_ref, ProcessTarget::Bridge, on_log, on_state, on_qr).await;
        });

        Ok(self.bridge.lock().unwrap().state.clone())
    }

    pub fn stop(&self, target: ProcessTarget) -> AppResult<()> {
        let mp_ref = self.target_ref(target).clone();
        let _pid;
        {
            let mut mp = mp_ref.lock().unwrap();
            match mp.state.state {
                ProcessStateKind::Running | ProcessStateKind::Starting => {
                    mp.state.state = ProcessStateKind::Stopping;
                    _pid = mp.state.pid;
                }
                _ => return Ok(()),
            }
        }
        self.emit_state(target);

        if let Some(mut child) = mp_ref.lock().unwrap().child.take() {
            let _ = child.start_kill();
            let rt = &self.runtime;
            let exit_code = rt.block_on(async {
                tokio::select! {
                    res = child.wait() => res.ok().and_then(|s| s.code()),
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                        let _ = child.start_kill();
                        child.wait().await.ok().and_then(|s| s.code())
                    }
                }
            });
            {
                let mut mp = mp_ref.lock().unwrap();
                mp.state = ProcessState {
                    state: ProcessStateKind::Stopped,
                    pid: None, started_at: None, uptime_sec: None,
                    exit_code, healthy: None,
                };
                mp.started_at_instant = None;
            }
            self.emit_state(target);
        }
        Ok(())
    }

    pub fn restart(&self, target: ProcessTarget, cfg: &crate::config::AppConfig, use_bun: bool) -> AppResult<ProcessState> {
        self.stop(target)?;
        match target {
            ProcessTarget::Server => self.start_server(cfg.server.port, &cfg.server.cwd, &cfg.server.extra_env),
            ProcessTarget::Bridge => {
                let store = crate::config::ConfigStore::new();
                let bridge_dir = store.bridge_install_path(cfg);
                self.start_bridge(&bridge_dir, use_bun)
            }
        }
    }

    pub fn get_state(&self, target: ProcessTarget) -> ProcessState {
        let mp = self.target_ref(target).lock().unwrap();
        let mut state = mp.state.clone();
        if state.state == ProcessStateKind::Running {
            if let Some(instant) = mp.started_at_instant {
                state.uptime_sec = Some(instant.elapsed().as_secs());
            }
        }
        state
    }

    pub fn set_health(&self, target: ProcessTarget, healthy: bool) {
        let mp_ref = self.target_ref(target).clone();
        let mut mp = mp_ref.lock().unwrap();
        if mp.state.state == ProcessStateKind::Running {
            mp.state.healthy = Some(healthy);
            let state = mp.state.clone();
            drop(mp);
            (self.on_state)(target, state);
        }
    }
}
