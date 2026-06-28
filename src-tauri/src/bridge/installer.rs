use std::path::{Path, PathBuf};
use crate::error::{AppError, AppResult};
use crate::process::resolve_command;

const BRIDGE_REPO: &str = "https://github.com/ET06731/opencode-im-bridge";

pub struct BridgeInstaller {
    path: PathBuf,
}

impl BridgeInstaller {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path { &self.path }

    pub fn is_installed(&self) -> bool {
        self.path.join(".git").exists()
    }

    pub async fn install(&self) -> AppResult<()> {
        if self.is_installed() {
            return Ok(());
        }
        std::fs::create_dir_all(self.path.parent().unwrap_or(Path::new(".")))?;
        let mut cmd = resolve_command("git")?;
        cmd.arg("clone").arg(BRIDGE_REPO).arg(&self.path);
        let status = cmd.status()
            .map_err(|e| AppError::BridgeInstall(format!("git clone failed: {}", e)))?;
        if !status.success() {
            return Err(AppError::BridgeInstall("git clone returned non-zero".into()));
        }
        Ok(())
    }

    pub async fn check_update(&self) -> AppResult<bool> {
        if !self.is_installed() {
            return Err(AppError::BridgeInstall("bridge not installed".into()));
        }
        let mut fetch = resolve_command("git")?;
        fetch.arg("fetch").current_dir(&self.path);
        fetch.status()
            .map_err(|e| AppError::BridgeInstall(format!("git fetch failed: {}", e)))?;

        let mut local_cmd = resolve_command("git")?;
        local_cmd.args(["rev-parse", "HEAD"]).current_dir(&self.path);
        let local = local_cmd.output()
            .map_err(|e| AppError::BridgeInstall(format!("git rev-parse failed: {}", e)))?;

        let mut remote_cmd = resolve_command("git")?;
        remote_cmd.args(["rev-parse", "origin/main"]).current_dir(&self.path);
        let remote = remote_cmd.output()
            .map_err(|e| AppError::BridgeInstall(format!("git rev-parse failed: {}", e)))?;

        let local_sha = String::from_utf8_lossy(&local.stdout).trim().to_string();
        let remote_sha = String::from_utf8_lossy(&remote.stdout).trim().to_string();
        Ok(local_sha == remote_sha)
    }

    pub async fn update(&self) -> AppResult<()> {
        if !self.is_installed() {
            return Err(AppError::BridgeInstall("bridge not installed".into()));
        }
        let mut cmd = resolve_command("git")?;
        cmd.args(["pull", "--ff-only"]).current_dir(&self.path);
        let status = cmd.status()
            .map_err(|e| AppError::BridgeInstall(format!("git pull failed: {}", e)))?;
        if !status.success() {
            return Err(AppError::BridgeInstall("git pull returned non-zero".into()));
        }
        Ok(())
    }

    pub async fn reinstall(&self) -> AppResult<()> {
        if self.path.exists() {
            std::fs::remove_dir_all(&self.path)
                .map_err(|e| AppError::BridgeInstall(format!("remove dir failed: {}", e)))?;
        }
        self.install().await
    }
}
