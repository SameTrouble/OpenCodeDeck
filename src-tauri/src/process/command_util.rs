use std::process::Command;
use crate::error::AppResult;

/// Resolve `name` to an absolute path via PATH (cross-platform, including
/// Windows `.cmd`/`.bat`/`.exe` extensions) and return a `Command` for it.
pub fn resolve_command(name: &str) -> AppResult<Command> {
    let path = which::which(name)?;
    Ok(Command::new(path))
}
