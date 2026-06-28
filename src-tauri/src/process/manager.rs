use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use tokio::process::Child;
use crate::error::{AppError, AppResult};
use std::path::Path;

fn parse_port_from_url(url: &str) -> crate::error::AppResult<u16> {
    let after_scheme = url.split("://").nth(1).unwrap_or(url);
    let host_port = after_scheme.split('/').next().unwrap_or(after_scheme);
    let port_str = host_port.rsplit(':').next().ok_or_else(|| {
        crate::error::AppError::Config(format!("url has no port: {}", url))
    })?;
    port_str.parse::<u16>().map_err(|_| {
        crate::error::AppError::Config(format!("invalid port in url: {}", url))
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStateItem {
    pub id: String,
    pub name: String,
    pub state: ProcessState,
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
    pub stopping: bool,
}

impl ManagedProcess {
    fn new() -> Self {
        Self { state: ProcessState::default(), child: None, started_at_instant: None, stopping: false }
    }
}

pub type StateCallback = Arc<dyn Fn(ProcessTarget, Option<String>, ProcessState) + Send + Sync>;
pub type LogCallback = Arc<dyn Fn(crate::monitor::LogEntry) + Send + Sync>;
pub type QrCallback = Arc<dyn Fn(crate::monitor::stdout_parser::WechatQrEvent) + Send + Sync>;

pub struct ProcessManager {
    servers: Arc<Mutex<std::collections::HashMap<String, ManagedProcess>>>,
    bridge: Arc<Mutex<ManagedProcess>>,
    on_state: StateCallback,
    on_log: LogCallback,
    on_qr: QrCallback,
    runtime: tokio::runtime::Runtime,
}

impl ProcessManager {
    pub fn new(on_state: StateCallback, on_log: LogCallback, on_qr: QrCallback) -> Self {
        Self {
            servers: Arc::new(Mutex::new(std::collections::HashMap::new())),
            bridge: Arc::new(Mutex::new(ManagedProcess::new())),
            on_state,
            on_log,
            on_qr,
            runtime: tokio::runtime::Runtime::new().expect("failed to create tokio runtime"),
        }
    }

    fn emit_state(&self, target: ProcessTarget, server_id: Option<String>) {
        let state = match target {
            ProcessTarget::Server => {
                let id = match &server_id {
                    Some(id) => id.clone(),
                    None => return,
                };
                match crate::process::lock_or_recover(&self.servers).get(&id) {
                    Some(mp) => mp.state.clone(),
                    None => return,
                }
            }
            ProcessTarget::Bridge => crate::process::lock_or_recover(&self.bridge).state.clone(),
        };
        (self.on_state)(target, server_id, state);
    }

    fn now_ts() -> i64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
    }

    pub fn start_server(&self, server_id: &str, cfg: &crate::config::AppConfig) -> crate::error::AppResult<ProcessState> {
        let server_cfg = cfg.servers.iter().find(|s| s.id == server_id)
            .ok_or_else(|| crate::error::AppError::Config(format!("server not found: {}", server_id)))?;
        let port = parse_port_from_url(&server_cfg.url)?;
        {
            let mut servers = crate::process::lock_or_recover(&self.servers);
            if let Some(mp) = servers.get(server_id) {
                if mp.state.state == ProcessStateKind::Running || mp.state.state == ProcessStateKind::Starting {
                    return Err(crate::error::AppError::Process(format!("server already running: {}", server_id)));
                }
            }
            servers.insert(server_id.to_string(), ManagedProcess {
                state: ProcessState { state: ProcessStateKind::Starting, ..Default::default() },
                child: None,
                started_at_instant: None,
                stopping: false,
            });
        }
        self.emit_state(ProcessTarget::Server, Some(server_id.to_string()));

        let mut cmd = tokio::process::Command::from(crate::process::resolve_command("opencode")?);
        cmd.arg("serve").arg("--port").arg(port.to_string());
        cmd.current_dir(if server_cfg.cwd.is_empty() { "." } else { &server_cfg.cwd });
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.envs(server_cfg.extra_env.iter());
        cmd.kill_on_drop(true);

        let _enter_guard = self.runtime.enter();
        let child = cmd.spawn().map_err(|e| crate::error::AppError::Process(format!("failed to spawn opencode: {}", e)))?;
        let pid = child.id();
        let now = Self::now_ts();
        let instant = Instant::now();

        {
            let mut servers = crate::process::lock_or_recover(&self.servers);
            if let Some(mp) = servers.get_mut(server_id) {
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
        }
        self.emit_state(ProcessTarget::Server, Some(server_id.to_string()));

        let on_log = self.on_log.clone();
        let on_state = self.on_state.clone();
        let servers_ref = self.servers.clone();
        let id_owned = server_id.to_string();
        self.runtime.spawn(async move {
            super::supervisor::supervise_with_id(servers_ref, id_owned, on_log, on_state).await;
        });

        let servers = crate::process::lock_or_recover(&self.servers);
        servers.get(server_id).map(|mp| mp.state.clone()).ok_or_else(|| crate::error::AppError::Process("server disappeared".into()))
    }

    pub async fn stop_server(&self, server_id: &str) -> crate::error::AppResult<()> {
        let mp_ref = {
            let mut servers = crate::process::lock_or_recover(&self.servers);
            let mp = match servers.get_mut(server_id) {
                Some(mp) => mp,
                None => return Ok(()),
            };
            match mp.state.state {
                ProcessStateKind::Running | ProcessStateKind::Starting => {
                    mp.state.state = ProcessStateKind::Stopping;
                    mp.stopping = true;
                    self.servers.clone()
                }
                _ => return Ok(()),
            }
        };
        self.emit_state(ProcessTarget::Server, Some(server_id.to_string()));

        let child_opt = {
            let mut servers = crate::process::lock_or_recover(&mp_ref);
            servers.get_mut(server_id).and_then(|mp| mp.child.take())
        };
        if let Some(mut child) = child_opt {
            let pid = child.id();
            #[cfg(unix)]
            if let Some(pid) = pid {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
            }
            #[cfg(not(unix))]
            {
                let _ = child.start_kill();
            }
            let exit_code = tokio::select! {
                res = child.wait() => res.ok().and_then(|s| s.code()),
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    let _ = child.start_kill();
                    child.wait().await.ok().and_then(|s| s.code())
                }
            };
            {
                let mut servers = crate::process::lock_or_recover(&mp_ref);
                if let Some(mp) = servers.get_mut(server_id) {
                    mp.state = ProcessState {
                        state: ProcessStateKind::Stopped,
                        pid: None, started_at: None, uptime_sec: None,
                        exit_code, healthy: None,
                    };
                    mp.started_at_instant = None;
                    mp.stopping = false;
                }
            }
            self.emit_state(ProcessTarget::Server, Some(server_id.to_string()));
        } else {
            let mut servers = crate::process::lock_or_recover(&mp_ref);
            if let Some(mp) = servers.get_mut(server_id) {
                mp.state = ProcessState {
                    state: ProcessStateKind::Stopped,
                    pid: None, started_at: None, uptime_sec: None,
                    exit_code: None, healthy: None,
                };
                mp.started_at_instant = None;
                mp.stopping = false;
            }
            self.emit_state(ProcessTarget::Server, Some(server_id.to_string()));
        }
        Ok(())
    }

    pub async fn restart_server(&self, server_id: &str, cfg: &crate::config::AppConfig) -> crate::error::AppResult<ProcessState> {
        self.stop_server(server_id).await?;
        self.start_server(server_id, cfg)
    }

    pub fn get_server_state(&self, server_id: &str) -> ProcessState {
        let servers = crate::process::lock_or_recover(&self.servers);
        let mut state = match servers.get(server_id) {
            Some(mp) => mp.state.clone(),
            None => ProcessState::default(),
        };
        if state.state == ProcessStateKind::Running {
            if let Some(mp) = servers.get(server_id) {
                if let Some(instant) = mp.started_at_instant {
                    state.uptime_sec = Some(instant.elapsed().as_secs());
                }
            }
        }
        state
    }

    pub fn get_all_server_states(&self) -> Vec<(String, ProcessState)> {
        let servers = crate::process::lock_or_recover(&self.servers);
        let mut result = Vec::new();
        for (id, mp) in servers.iter() {
            let mut state = mp.state.clone();
            if state.state == ProcessStateKind::Running {
                if let Some(instant) = mp.started_at_instant {
                    state.uptime_sec = Some(instant.elapsed().as_secs());
                }
            }
            result.push((id.clone(), state));
        }
        result
    }

    pub async fn stop_all_servers(&self) {
        let ids: Vec<String> = {
            let servers = crate::process::lock_or_recover(&self.servers);
            servers.keys().cloned().collect()
        };
        for id in ids {
            let _ = self.stop_server(&id).await;
        }
    }

    pub fn start_bridge(&self, bridge_dir: &std::path::Path, use_bun: bool) -> AppResult<ProcessState> {
        {
            let mut mp = crate::process::lock_or_recover(&self.bridge);
            if mp.state.state == ProcessStateKind::Running || mp.state.state == ProcessStateKind::Starting {
                return Err(AppError::Process("bridge already running".into()));
            }
            mp.state = ProcessState { state: ProcessStateKind::Starting, ..Default::default() };
        }
        self.emit_state(ProcessTarget::Bridge, None);

        let mut cmd = if use_bun {
            let mut c = tokio::process::Command::from(crate::process::resolve_command("bun")?);
            c.arg("run").arg("src/index.ts");
            c
        } else {
            let mut c = tokio::process::Command::from(crate::process::resolve_command("npx")?);
            c.arg("tsx").arg("src/index.ts");
            c
        };
        cmd.current_dir(bridge_dir);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.kill_on_drop(true);

        let _enter_guard = self.runtime.enter();
        let child = cmd.spawn().map_err(|e| AppError::Process(format!("failed to spawn bridge: {}", e)))?;
        let pid = child.id();
        let now = Self::now_ts();
        let instant = Instant::now();

        {
            let mut mp = crate::process::lock_or_recover(&self.bridge);
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
        self.emit_state(ProcessTarget::Bridge, None);

        let on_log = self.on_log.clone();
        let on_state = self.on_state.clone();
        let on_qr = self.on_qr.clone();
        let bridge_ref = self.bridge.clone();
        self.runtime.spawn(async move {
            super::supervisor::supervise_with_qr(bridge_ref, ProcessTarget::Bridge, on_log, on_state, on_qr).await;
        });

        Ok(crate::process::lock_or_recover(&self.bridge).state.clone())
    }

    pub async fn stop_async(&self, target: ProcessTarget) -> crate::error::AppResult<()> {
        let mp_ref = match target {
            ProcessTarget::Server => return Err(crate::error::AppError::Process("use stop_server for server".into())),
            ProcessTarget::Bridge => self.bridge.clone(),
        };
        let _pid;
        {
            let mut mp = crate::process::lock_or_recover(&mp_ref);
            match mp.state.state {
                ProcessStateKind::Running | ProcessStateKind::Starting => {
                    mp.state.state = ProcessStateKind::Stopping;
                    mp.stopping = true;
                    _pid = mp.state.pid;
                }
                _ => return Ok(()),
            }
        }
        self.emit_state(target, None);

        let child_opt = crate::process::lock_or_recover(&mp_ref).child.take();
        if let Some(mut child) = child_opt {
            let pid = child.id();
            #[cfg(unix)]
            if let Some(pid) = pid {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
            }
            #[cfg(not(unix))]
            {
                let _ = child.start_kill();
            }
            let exit_code = tokio::select! {
                res = child.wait() => res.ok().and_then(|s| s.code()),
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    let _ = child.start_kill();
                    child.wait().await.ok().and_then(|s| s.code())
                }
            };
            {
                let mut mp = crate::process::lock_or_recover(&mp_ref);
                mp.state = ProcessState {
                    state: ProcessStateKind::Stopped,
                    pid: None, started_at: None, uptime_sec: None,
                    exit_code, healthy: None,
                };
                mp.started_at_instant = None;
                mp.stopping = false;
            }
            self.emit_state(target, None);
        } else {
            {
                let mut mp = crate::process::lock_or_recover(&mp_ref);
                mp.state = ProcessState {
                    state: ProcessStateKind::Stopped,
                    pid: None, started_at: None, uptime_sec: None,
                    exit_code: None, healthy: None,
                };
                mp.started_at_instant = None;
                mp.stopping = false;
            }
            self.emit_state(target, None);
        }
        Ok(())
    }

    pub fn stop(&self, target: ProcessTarget) -> AppResult<()> {
        let rt = &self.runtime;
        rt.block_on(self.stop_async(target))
    }

    pub async fn restart_async(&self, target: ProcessTarget, cfg: &crate::config::AppConfig, bridge_dir: &Path, use_bun: bool) -> crate::error::AppResult<ProcessState> {
        match target {
            ProcessTarget::Server => return Err(crate::error::AppError::Process("use restart_server for server".into())),
            ProcessTarget::Bridge => {
                self.stop_async(target).await?;
                crate::config::renderer::write_bridge_files(cfg, bridge_dir)?;
                self.start_bridge(bridge_dir, use_bun)
            }
        }
    }

    pub fn get_state(&self, target: ProcessTarget) -> ProcessState {
        match target {
            ProcessTarget::Server => ProcessState::default(),
            ProcessTarget::Bridge => {
                let mp = crate::process::lock_or_recover(&self.bridge);
                let mut state = mp.state.clone();
                if state.state == ProcessStateKind::Running {
                    if let Some(instant) = mp.started_at_instant {
                        state.uptime_sec = Some(instant.elapsed().as_secs());
                    }
                }
                state
            }
        }
    }

    pub fn set_health(&self, target: ProcessTarget, server_id: Option<String>, healthy: bool) {
        match target {
            ProcessTarget::Server => {
                if let Some(id) = &server_id {
                    let mp_ref = self.servers.clone();
                    let mut servers = crate::process::lock_or_recover(&mp_ref);
                    if let Some(mp) = servers.get_mut(id) {
                        if mp.state.state == ProcessStateKind::Running {
                            mp.state.healthy = Some(healthy);
                            let state = mp.state.clone();
                            drop(servers);
                            (self.on_state)(target, server_id, state);
                        }
                    }
                }
            }
            ProcessTarget::Bridge => {
                let mut mp = crate::process::lock_or_recover(&self.bridge);
                if mp.state.state == ProcessStateKind::Running {
                    mp.state.healthy = Some(healthy);
                    let state = mp.state.clone();
                    drop(mp);
                    (self.on_state)(target, None, state);
                }
            }
        }
    }
}

#[cfg(test)]
mod serde_tests {
    use super::*;
    #[test]
    fn test_process_state_kind_serialization() {
        let cases = [
            (ProcessStateKind::Stopped, "Stopped"),
            (ProcessStateKind::Starting, "Starting"),
            (ProcessStateKind::Running, "Running"),
            (ProcessStateKind::Stopping, "Stopping"),
            (ProcessStateKind::Failed, "Failed"),
        ];
        for (kind, expected) in cases {
            let s = serde_json::to_string(&kind).unwrap();
            assert_eq!(s, format!("\"{}\"", expected));
            let de: ProcessStateKind = serde_json::from_str(&s).unwrap();
            assert_eq!(de, kind);
        }
    }
}

#[cfg(test)]
mod stopping_flag_tests {
    use super::*;
    use std::sync::Arc;

    fn noop_callbacks() -> (StateCallback, LogCallback, QrCallback) {
        let on_state: StateCallback = Arc::new(|_, _, _| {});
        let on_log: LogCallback = Arc::new(|_| {});
        let on_qr: QrCallback = Arc::new(|_| {});
        (on_state, on_log, on_qr)
    }

    #[tokio::test]
    async fn stop_async_on_unstarted_process_is_noop() {
        let (on_state, on_log, on_qr) = noop_callbacks();
        let pm = ProcessManager::new(on_state, on_log, on_qr);
        pm.stop_async(ProcessTarget::Bridge).await.unwrap();
        let s = pm.get_state(ProcessTarget::Bridge);
        assert_eq!(s.state, ProcessStateKind::Stopped);
        {
            let mp = crate::process::lock_or_recover(&pm.bridge);
            assert!(!mp.stopping, "stopping flag must be false when process was never Running");
        }
        std::mem::forget(pm);
    }

    #[tokio::test]
    async fn stop_async_on_running_process_sets_stopping_flag() {
        let (on_state, on_log, on_qr) = noop_callbacks();
        let pm = ProcessManager::new(on_state, on_log, on_qr);
        {
            let mut mp = crate::process::lock_or_recover(&pm.bridge);
            mp.state.state = ProcessStateKind::Running;
            mp.stopping = true;
        }
        pm.stop_async(ProcessTarget::Bridge).await.unwrap();
        let s = pm.get_state(ProcessTarget::Bridge);
        assert_eq!(s.state, ProcessStateKind::Stopped, "state should be Stopped after stop_async");
        {
            let mp = crate::process::lock_or_recover(&pm.bridge);
            assert!(!mp.stopping, "stopping flag must be cleared after stop_async completes");
        }
        std::mem::forget(pm);
    }

    #[tokio::test]
    async fn restart_async_bridge_writes_config_files() {
        use crate::config::ConfigStore;
        let (on_state, on_log, on_qr) = noop_callbacks();
        let pm = ProcessManager::new(on_state, on_log, on_qr);
        let cfg = ConfigStore::default_config();
        let tmp = tempfile::tempdir().unwrap();
        let bridge_dir = tmp.path().to_path_buf();
        let _ = pm.restart_async(ProcessTarget::Bridge, &cfg, &bridge_dir, false).await;
        assert!(bridge_dir.join(".env").exists(), "restart_async must write .env before starting bridge");
        assert!(bridge_dir.join("opencode-im.jsonc").exists(), "restart_async must write opencode-im.jsonc before starting bridge");
        std::mem::forget(pm);
    }
}

#[cfg(test)]
mod multi_server_tests {
    use super::*;
    use std::sync::Arc;
    use crate::config::ConfigStore;

    fn noop_callbacks() -> (StateCallback, LogCallback, QrCallback) {
        let on_state: StateCallback = Arc::new(|_, _, _| {});
        let on_log: LogCallback = Arc::new(|_| {});
        let on_qr: QrCallback = Arc::new(|_| {});
        (on_state, on_log, on_qr)
    }

    #[test]
    fn get_all_server_states_empty_when_none_started() {
        let (on_state, on_log, on_qr) = noop_callbacks();
        let pm = ProcessManager::new(on_state, on_log, on_qr);
        let states = pm.get_all_server_states();
        assert!(states.is_empty());
        std::mem::forget(pm);
    }

    #[test]
    fn get_server_state_for_unknown_id_is_stopped() {
        let (on_state, on_log, on_qr) = noop_callbacks();
        let pm = ProcessManager::new(on_state, on_log, on_qr);
        let s = pm.get_server_state("nonexistent");
        assert_eq!(s.state, ProcessStateKind::Stopped);
        std::mem::forget(pm);
    }

    #[test]
    fn start_server_rejects_unknown_id() {
        let (on_state, on_log, on_qr) = noop_callbacks();
        let pm = ProcessManager::new(on_state, on_log, on_qr);
        let cfg = ConfigStore::default_config();
        let result = pm.start_server("nonexistent", &cfg);
        assert!(result.is_err(), "starting unknown server id should error");
        std::mem::forget(pm);
    }
}
