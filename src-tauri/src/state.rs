use std::sync::{Arc, Mutex};
use crate::config::{ConfigStore, AppConfig};
use crate::process::ProcessManager;
use crate::monitor::LogBuffer;

pub struct AppState {
    pub config_store: ConfigStore,
    pub process_manager: ProcessManager,
    pub log_buffer: Arc<Mutex<LogBuffer>>,
}

impl AppState {
    pub fn new_with_buffer(process_manager: ProcessManager, log_buffer: Arc<Mutex<LogBuffer>>) -> Self {
        Self {
            config_store: ConfigStore::new(),
            process_manager,
            log_buffer,
        }
    }

    pub fn load_config(&self) -> crate::error::AppResult<AppConfig> {
        self.config_store.load()
    }

    pub fn save_config(&self, config: &AppConfig) -> crate::error::AppResult<()> {
        self.config_store.save(config)
    }
}
