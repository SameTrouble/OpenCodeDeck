use crate::error::{AppError, AppResult};

/// 探测 (hostname, port) 是否被占用。
/// TcpListener::bind 成功 = 空闲（立即 drop），失败 = 占用。
pub fn is_port_in_use(hostname: &str, port: u16) -> bool {
    std::net::TcpListener::bind((hostname, port)).is_err()
}

#[cfg(unix)]
pub fn pids_on_port(port: u16) -> AppResult<Vec<u32>> {
    let output = std::process::Command::new("lsof")
        .arg("-i")
        .arg(format!(":{}", port))
        .arg("-t")
        .output()
        .map_err(|e| AppError::Process(format!("failed to run lsof: {}", e)))?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let pids: Vec<u32> = stdout.lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .collect();
    Ok(pids)
}

#[cfg(not(unix))]
pub fn pids_on_port(_port: u16) -> AppResult<Vec<u32>> {
    Err(AppError::Process("pids_on_port only supported on unix".into()))
}

#[cfg(unix)]
pub fn kill_pid(pid: u32) -> AppResult<()> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    let nix_pid = Pid::from_raw(pid as i32);
    match kill(nix_pid, Signal::SIGTERM) {
        Ok(()) => {}
        Err(nix::errno::Errno::ESRCH) => return Ok(()),
        Err(e) => return Err(AppError::Process(format!("failed to SIGTERM pid {}: {}", pid, e))),
    }
    std::thread::sleep(std::time::Duration::from_millis(2000));
    match kill(nix_pid, Signal::SIGKILL) {
        Ok(()) => {
            std::thread::sleep(std::time::Duration::from_millis(200));
            Ok(())
        }
        Err(nix::errno::Errno::ESRCH) => Ok(()),
        Err(e) => Err(AppError::Process(format!("failed to SIGKILL pid {}: {}", pid, e))),
    }
}

#[cfg(not(unix))]
pub fn kill_pid(_pid: u32) -> AppResult<()> {
    Err(AppError::Process("kill_pid only supported on unix".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_port_in_use_returns_true_when_bound() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(is_port_in_use("127.0.0.1", port));
    }

    #[test]
    fn is_port_in_use_returns_false_when_free() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(!is_port_in_use("127.0.0.1", port));
    }

    #[test]
    #[cfg(unix)]
    fn pids_on_port_finds_self() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let pids = pids_on_port(port).unwrap();
        let self_pid = std::process::id();
        assert!(pids.contains(&self_pid), "expected pids {:?} to contain {}", pids, self_pid);
    }
}
