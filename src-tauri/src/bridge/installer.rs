use std::path::{Path, PathBuf};
use std::process::Command;
use crate::error::{AppError, AppResult};

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

    pub fn install(&self) -> AppResult<()> {
        if self.is_installed() {
            return Ok(());
        }
        std::fs::create_dir_all(self.path.parent().unwrap_or(Path::new(".")))?;
        let status = Command::new("git")
            .arg("clone")
            .arg(BRIDGE_REPO)
            .arg(&self.path)
            .status()
            .map_err(|e| AppError::BridgeInstall(format!("git clone failed: {}", e)))?;
        if !status.success() {
            return Err(AppError::BridgeInstall("git clone returned non-zero".into()));
        }
        Ok(())
    }

    pub fn check_update(&self) -> AppResult<bool> {
        if !self.is_installed() {
            return Err(AppError::BridgeInstall("bridge not installed".into()));
        }
        Command::new("git").arg("fetch")
            .current_dir(&self.path)
            .status()
            .map_err(|e| AppError::BridgeInstall(format!("git fetch failed: {}", e)))?;
        let local = Command::new("git").args(["rev-parse", "HEAD"])
            .current_dir(&self.path).output()
            .map_err(|e| AppError::BridgeInstall(format!("git rev-parse failed: {}", e)))?;
        let remote = Command::new("git").args(["rev-parse", "origin/main"])
            .current_dir(&self.path).output()
            .map_err(|e| AppError::BridgeInstall(format!("git rev-parse failed: {}", e)))?;
        let local_sha = String::from_utf8_lossy(&local.stdout).trim().to_string();
        let remote_sha = String::from_utf8_lossy(&remote.stdout).trim().to_string();
        Ok(local_sha == remote_sha)
    }

    pub fn update(&self) -> AppResult<()> {
        if !self.is_installed() {
            return Err(AppError::BridgeInstall("bridge not installed".into()));
        }
        let status = Command::new("git").args(["pull", "--ff-only"])
            .current_dir(&self.path)
            .status()
            .map_err(|e| AppError::BridgeInstall(format!("git pull failed: {}", e)))?;
        if !status.success() {
            return Err(AppError::BridgeInstall("git pull returned non-zero".into()));
        }
        Ok(())
    }

    pub fn reinstall(&self) -> AppResult<()> {
        if self.path.exists() {
            std::fs::remove_dir_all(&self.path)
                .map_err(|e| AppError::BridgeInstall(format!("remove dir failed: {}", e)))?;
        }
        self.install()
    }
}
