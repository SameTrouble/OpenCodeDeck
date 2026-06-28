use serde::{Deserialize, Serialize};

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
    which::which(cmd).is_ok()
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
