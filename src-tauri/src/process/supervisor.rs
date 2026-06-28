use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use super::manager::{ManagedProcess, ProcessTarget, ProcessState, ProcessStateKind, StateCallback, LogCallback, QrCallback};
use crate::monitor::{LogEntry, stdout_parser::StdoutParser};

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn source_str(target: ProcessTarget) -> &'static str {
    match target { ProcessTarget::Server => "server", ProcessTarget::Bridge => "bridge" }
}

async fn read_stream<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    source: String,
    level: String,
    on_log: LogCallback,
) {
    let mut lines = BufReader::new(reader).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => on_log(LogEntry { ts: now_ts(), source: source.clone(), level: level.clone(), line }),
            Ok(None) => break,
            Err(e) => {
                on_log(LogEntry {
                    ts: now_ts(),
                    source: source.clone(),
                    level: "error".to_string(),
                    line: format!("stream read error: {}", e),
                });
                break;
            }
        }
    }
}

async fn read_stream_with_qr<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    source: String,
    level: String,
    on_log: LogCallback,
    on_qr: QrCallback,
    parser: Arc<Mutex<StdoutParser>>,
) {
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if level == "info" {
            if let Some(ev) = crate::process::lock_or_recover(&parser).feed_line(&line) {
                on_qr(ev);
            }
        }
        on_log(LogEntry { ts: now_ts(), source: source.clone(), level: level.clone(), line });
    }
}

pub(crate) async fn supervise(
    process: Arc<Mutex<ManagedProcess>>,
    target: ProcessTarget,
    on_log: LogCallback,
    on_state: StateCallback,
) {
    let (stdout, stderr, child_ref) = {
        let mut mp = crate::process::lock_or_recover(&process);
        let child = match mp.child.as_mut() {
            Some(c) => c,
            None => return,
        };
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        (stdout, stderr, process.clone())
    };
    let source = source_str(target).to_string();
    let log_clone = on_log.clone();
    let stdout_task = if let Some(out) = stdout {
        Some(tokio::spawn(read_stream(out, source.clone(), "info".into(), on_log)))
    } else { None };
    let stderr_task = if let Some(err) = stderr {
        Some(tokio::spawn(read_stream(err, source.clone(), "error".into(), log_clone)))
    } else { None };

    if let Some(t) = stdout_task { let _ = t.await; }
    if let Some(t) = stderr_task { let _ = t.await; }

    let exit_code = {
        let child = {
            let mut mp = crate::process::lock_or_recover(&child_ref);
            mp.child.take()
        };
        match child {
            Some(mut c) => c.wait().await.ok().and_then(|s| s.code()),
            None => None,
        }
    };
    {
        let mut mp = crate::process::lock_or_recover(&child_ref);
        let next_state = if mp.stopping {
            mp.stopping = false;
            ProcessStateKind::Stopped
        } else {
            ProcessStateKind::Failed
        };
        mp.state = ProcessState {
            state: next_state,
            pid: None, started_at: None, uptime_sec: None,
            exit_code, healthy: None,
        };
        mp.started_at_instant = None;
        let state = mp.state.clone();
        drop(mp);
        on_state(target, state);
    }
}

pub(crate) async fn supervise_with_qr(
    process: Arc<Mutex<ManagedProcess>>,
    target: ProcessTarget,
    on_log: LogCallback,
    on_state: StateCallback,
    on_qr: QrCallback,
) {
    let (stdout, stderr, child_ref) = {
        let mut mp = crate::process::lock_or_recover(&process);
        let child = match mp.child.as_mut() {
            Some(c) => c,
            None => return,
        };
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        (stdout, stderr, process.clone())
    };
    let source = source_str(target).to_string();
    let parser = Arc::new(Mutex::new(StdoutParser::new()));
    let log_clone = on_log.clone();
    let qr_clone = on_qr.clone();
    let parser_clone = parser.clone();
    let stdout_task = if let Some(out) = stdout {
        Some(tokio::spawn(read_stream_with_qr(out, source.clone(), "info".into(), on_log, qr_clone, parser_clone)))
    } else { None };
    let stderr_task = if let Some(err) = stderr {
        Some(tokio::spawn(read_stream(err, source.clone(), "error".into(), log_clone)))
    } else { None };

    if let Some(t) = stdout_task { let _ = t.await; }
    if let Some(t) = stderr_task { let _ = t.await; }

    let exit_code = {
        let child = {
            let mut mp = crate::process::lock_or_recover(&child_ref);
            mp.child.take()
        };
        match child {
            Some(mut c) => c.wait().await.ok().and_then(|s| s.code()),
            None => None,
        }
    };
    {
        let mut mp = crate::process::lock_or_recover(&child_ref);
        let next_state = if mp.stopping {
            mp.stopping = false;
            ProcessStateKind::Stopped
        } else {
            ProcessStateKind::Failed
        };
        mp.state = ProcessState {
            state: next_state,
            pid: None, started_at: None, uptime_sec: None,
            exit_code, healthy: None,
        };
        mp.started_at_instant = None;
        let state = mp.state.clone();
        drop(mp);
        on_state(target, state);
    }
}
