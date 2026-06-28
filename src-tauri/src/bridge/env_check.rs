use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepStatus {
    pub opencode: bool,
    pub bun: bool,
    pub node: bool,
    pub npm: bool,
    pub git: bool,
}

fn which(cmd: &str) -> bool {
    let check = if cfg!(target_os = "windows") {
        Command::new("where").arg(cmd).output()
    } else {
        Command::new("which").arg(cmd).output()
    };
    match check {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

pub fn check_deps() -> DepStatus {
    DepStatus {
        opencode: which("opencode"),
        bun: which("bun"),
        node: which("node"),
        npm: which("npm"),
        git: which("git"),
    }
}
