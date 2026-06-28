pub mod manager;
pub mod supervisor;
pub mod command_util;

pub use manager::{ProcessManager, ProcessState, ProcessTarget, ProcessStateKind, StateCallback, LogCallback, QrCallback};
pub use command_util::resolve_command;

use std::sync::{Mutex, MutexGuard};

pub(crate) fn lock_or_recover<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}
