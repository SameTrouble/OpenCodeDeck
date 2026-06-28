use tauri::State;
use tauri_plugin_dialog::DialogExt;
use crate::bridge::{check_deps as bridge_check_deps, DepStatus, BridgeInstaller};
use crate::config::{AppConfig, renderer};
use crate::error::{AppError, AppResult};
use crate::monitor::LogEntry;
use crate::process::{ProcessState, ProcessTarget};
use crate::state::AppState;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FullState {
    pub server: ProcessState,
    pub bridge: ProcessState,
}

#[tauri::command]
pub fn get_state(state: State<'_, AppState>) -> AppResult<FullState> {
    Ok(FullState {
        server: state.process_manager.get_state(ProcessTarget::Server),
        bridge: state.process_manager.get_state(ProcessTarget::Bridge),
    })
}

fn parse_target(target: &str) -> AppResult<ProcessTarget> {
    match target {
        "server" => Ok(ProcessTarget::Server),
        "bridge" => Ok(ProcessTarget::Bridge),
        _ => Err(AppError::Process(format!("unknown target: {}", target))),
    }
}

#[tauri::command]
pub fn start_process(target: String, state: State<'_, AppState>) -> AppResult<ProcessState> {
    let target = parse_target(&target)?;
    let cfg = state.load_config()?;
    match target {
        ProcessTarget::Server => state.process_manager.start_server(cfg.server.port, &cfg.server.cwd, &cfg.server.extra_env),
        ProcessTarget::Bridge => {
            let installer = BridgeInstaller::new(state.config_store.bridge_install_path(&cfg));
            if !installer.is_installed() {
                installer.install()?;
            }
            renderer::write_bridge_files(&cfg, installer.path())?;
            let deps = bridge_check_deps();
            state.process_manager.start_bridge(installer.path(), deps.bun)
        }
    }
}

#[tauri::command]
pub async fn stop_process(target: String, state: State<'_, AppState>) -> AppResult<()> {
    let target = parse_target(&target)?;
    state.process_manager.stop_async(target).await
}

#[tauri::command]
pub async fn restart_process(target: String, state: State<'_, AppState>) -> AppResult<ProcessState> {
    let target = parse_target(&target)?;
    let cfg = state.load_config()?;
    let deps = bridge_check_deps();
    state.process_manager.restart_async(target, &cfg, deps.bun).await
}

pub fn do_start_all(state: &AppState) -> AppResult<()> {
    let cfg = state.load_config()?;
    state.process_manager.start_server(cfg.server.port, &cfg.server.cwd, &cfg.server.extra_env)?;
    let installer = BridgeInstaller::new(state.config_store.bridge_install_path(&cfg));
    if !installer.is_installed() {
        installer.install()?;
    }
    renderer::write_bridge_files(&cfg, installer.path())?;
    let deps = bridge_check_deps();
    state.process_manager.start_bridge(installer.path(), deps.bun)?;
    Ok(())
}

pub async fn do_stop_all(state: &AppState) -> AppResult<()> {
    state.process_manager.stop_async(ProcessTarget::Bridge).await?;
    state.process_manager.stop_async(ProcessTarget::Server).await?;
    Ok(())
}

pub async fn do_restart_all(state: &AppState) -> AppResult<()> {
    do_stop_all(state).await?;
    do_start_all(state)
}

#[tauri::command]
pub fn start_all(state: State<'_, AppState>) -> AppResult<()> { do_start_all(state.inner()) }

#[tauri::command]
pub async fn stop_all(state: State<'_, AppState>) -> AppResult<()> { do_stop_all(state.inner()).await }

#[tauri::command]
pub async fn restart_all(state: State<'_, AppState>) -> AppResult<()> { do_restart_all(state.inner()).await }

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> AppResult<AppConfig> {
    state.load_config()
}

#[tauri::command]
pub fn save_config(config: AppConfig, state: State<'_, AppState>) -> AppResult<()> {
    state.save_config(&config)
}

#[tauri::command]
pub fn check_bridge_update(state: State<'_, AppState>) -> AppResult<bool> {
    let cfg = state.load_config()?;
    let installer = BridgeInstaller::new(state.config_store.bridge_install_path(&cfg));
    installer.check_update()
}

#[tauri::command]
pub fn update_bridge(state: State<'_, AppState>) -> AppResult<()> {
    let cfg = state.load_config()?;
    let installer = BridgeInstaller::new(state.config_store.bridge_install_path(&cfg));
    installer.update()
}

#[tauri::command]
pub fn reinstall_bridge(state: State<'_, AppState>) -> AppResult<()> {
    let cfg = state.load_config()?;
    let installer = BridgeInstaller::new(state.config_store.bridge_install_path(&cfg));
    installer.reinstall()
}

#[tauri::command]
pub fn get_log_history(source: String, limit: usize, state: State<'_, AppState>) -> AppResult<Vec<LogEntry>> {
    let buf = state.log_buffer.lock().unwrap();
    let entries = if source == "all" {
        buf.recent_all(limit)
    } else {
        buf.recent(&source, limit)
    };
    Ok(entries)
}

#[tauri::command]
pub fn clear_logs(source: String, state: State<'_, AppState>) -> AppResult<()> {
    let mut buf = state.log_buffer.lock().unwrap();
    buf.clear(&source);
    Ok(())
}

#[tauri::command]
pub fn export_logs(source: String, state: State<'_, AppState>, app: tauri::AppHandle) -> AppResult<String> {
    let buf = state.log_buffer.lock().unwrap();
    let entries = if source == "all" { buf.recent_all(100000) } else { buf.recent(&source, 100000) };
    drop(buf);
    let content = entries.iter()
        .map(|e| format!("[{}] [{}] [{}] {}", e.ts, e.source, e.level, e.line))
        .collect::<Vec<_>>()
        .join("\n");
    let default_path = dirs::download_dir()
        .or_else(|| dirs::home_dir())
        .unwrap_or_default();
    let path = app.dialog().file()
        .set_directory(&default_path)
        .set_file_name(format!("opencodedeck-logs-{}.txt", source))
        .blocking_save_file();
    let path = match path {
        Some(p) => p.into_path().unwrap_or_else(|_| default_path.join(format!("opencodedeck-logs-{}.txt", source))),
        None => return Err(AppError::Process("export cancelled".into())),
    };
    std::fs::write(&path, content)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn check_deps() -> AppResult<DepStatus> {
    Ok(bridge_check_deps())
}
