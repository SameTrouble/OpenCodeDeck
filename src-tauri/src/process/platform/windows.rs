use std::mem::size_of_val;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
    JobObjectExtendedLimitInformation, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

pub struct ProcessTracker {
    job: HANDLE,
}

unsafe impl Send for ProcessTracker {}
unsafe impl Sync for ProcessTracker {}

impl ProcessTracker {
    pub fn new_for_child(child: &tokio::process::Child) -> Option<Self> {
        let pid = child.id()?;
        unsafe {
            let job = CreateJobObjectW(None, None).ok()?;

            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as _,
                size_of_val(&info) as u32,
            )
            .ok()?;

            let proc = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid).ok()?;
            AssignProcessToJobObject(job, proc).ok()?;
            let _ = CloseHandle(proc);

            Some(Self { job })
        }
    }
}

impl Drop for ProcessTracker {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.job);
        }
    }
}

pub fn graceful_terminate(_pid: u32) {
}
