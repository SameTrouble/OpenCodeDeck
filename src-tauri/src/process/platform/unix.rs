pub struct ProcessTracker;

impl ProcessTracker {
    pub fn new_for_child(_child: &tokio::process::Child) -> Option<Self> {
        Some(ProcessTracker)
    }
}

pub fn graceful_terminate(pid: u32) {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
}
