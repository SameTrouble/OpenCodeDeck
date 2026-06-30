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

#[cfg(windows)]
#[repr(C)]
struct MIB_TCPROW_OWNER_PID {
    dw_state: u32,
    dw_local_addr: u32,
    dw_local_port: u32,
    dw_remote_addr: u32,
    dw_remote_port: u32,
    dw_owning_pid: u32,
}

#[cfg(windows)]
pub fn pids_on_port(port: u16) -> AppResult<Vec<u32>> {
    use windows::Win32::NetworkManagement::IpHelper::{GetExtendedTcpTable, TCP_TABLE_OWNER_PID_LISTENER};
    use windows::Win32::Networking::WinSock::htons;

    let mut size: u32 = 0;
    unsafe {
        GetExtendedTcpTable(None, &mut size, false, 2, TCP_TABLE_OWNER_PID_LISTENER, 0);
    }
    if size == 0 {
        return Ok(Vec::new());
    }
    let mut buf = vec![0u8; size as usize];
    let result = unsafe {
        GetExtendedTcpTable(Some(buf.as_mut_ptr() as *mut _), &mut size, false, 2, TCP_TABLE_OWNER_PID_LISTENER, 0)
    };
    if result != 0 {
        return Ok(Vec::new());
    }

    let entries_count = unsafe {
        (buf.as_ptr() as *const u32).read_unaligned()
    };

    let row_size = std::mem::size_of::<MIB_TCPROW_OWNER_PID>();
    let header_size = std::mem::size_of::<u32>();
    let mut pids = Vec::new();
        let target_port = unsafe { htons(port) } as u32;
        for i in 0..entries_count as usize {
            let offset = header_size + i * row_size;
            if offset + row_size > buf.len() {
                break;
            }
            let row = unsafe {
                &*(buf.as_ptr().add(offset) as *const MIB_TCPROW_OWNER_PID)
            };
            if row.dw_local_port == target_port && row.dw_state == 2 {
            pids.push(row.dw_owning_pid);
        }
    }
    Ok(pids)
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

#[cfg(windows)]
pub fn kill_pid(pid: u32) -> AppResult<()> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, false, pid)
            .map_err(|e| AppError::Process(format!("OpenProcess failed for pid {}: {}", pid, e)))?;
        TerminateProcess(handle, 1)
            .map_err(|e| AppError::Process(format!("TerminateProcess failed for pid {}: {}", pid, e)))?;
        let _ = CloseHandle(handle);
    }
    Ok(())
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

    #[test]
    #[cfg(windows)]
    fn pids_on_port_finds_self() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let pids = pids_on_port(port).unwrap();
        let self_pid = std::process::id();
        assert!(pids.contains(&self_pid), "expected pids {:?} to contain {}", pids, self_pid);
    }
}
