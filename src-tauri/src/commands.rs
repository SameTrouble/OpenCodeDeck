use tauri::State;
use tauri_plugin_dialog::DialogExt;
use crate::bridge::{check_deps as bridge_check_deps, DepStatus, BridgeInstaller};
use crate::config::{AppConfig, renderer};
use crate::error::{AppError, AppResult};
use crate::monitor::LogEntry;
use crate::process::{ProcessState, ProcessTarget, ServerStateItem};
use crate::state::AppState;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FullState {
    pub servers: Vec<ServerStateItem>,
    pub bridge: ProcessState,
}

#[tauri::command]
pub fn get_state(state: State<'_, AppState>) -> AppResult<FullState> {
    let cfg = state.load_config()?;
    let servers = cfg.servers.iter().map(|s| ServerStateItem {
        id: s.id.clone(),
        name: s.name.clone(),
        state: state.process_manager.get_server_state(&s.id),
    }).collect();
    Ok(FullState {
        servers,
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

pub async fn do_start_bridge(state: &AppState) -> AppResult<()> {
    let cfg = state.load_config()?;
    let installer = BridgeInstaller::new(state.config_store.bridge_install_path(&cfg));
    if !installer.is_installed() {
        installer.install().await?;
    }
    renderer::write_bridge_files(&cfg, installer.path())?;
    let deps = bridge_check_deps();
    state.process_manager.start_bridge(installer.path(), deps.bun)?;
    Ok(())
}

#[tauri::command]
pub async fn start_process(target: String, server_id: Option<String>, state: State<'_, AppState>) -> AppResult<ProcessState> {
    let target = parse_target(&target)?;
    let cfg = state.load_config()?;
    match target {
        ProcessTarget::Server => {
            let id = server_id.ok_or_else(|| AppError::Process("server_id required for server target".into()))?;
            state.process_manager.start_server(&id, &cfg)
        }
        ProcessTarget::Bridge => {
            do_start_bridge(state.inner()).await?;
            Ok(state.process_manager.get_state(ProcessTarget::Bridge))
        }
    }
}

#[tauri::command]
pub async fn stop_process(target: String, server_id: Option<String>, state: State<'_, AppState>) -> AppResult<()> {
    let target = parse_target(&target)?;
    match target {
        ProcessTarget::Server => {
            let id = server_id.ok_or_else(|| AppError::Process("server_id required for server target".into()))?;
            state.process_manager.stop_server(&id).await
        }
        ProcessTarget::Bridge => state.process_manager.stop_async(target).await,
    }
}

#[tauri::command]
pub async fn restart_process(target: String, server_id: Option<String>, state: State<'_, AppState>) -> AppResult<ProcessState> {
    let target = parse_target(&target)?;
    let cfg = state.load_config()?;
    match target {
        ProcessTarget::Server => {
            let id = server_id.ok_or_else(|| AppError::Process("server_id required for server target".into()))?;
            state.process_manager.restart_server(&id, &cfg).await
        }
        ProcessTarget::Bridge => {
            let bridge_dir = state.config_store.bridge_install_path(&cfg);
            let deps = bridge_check_deps();
            state.process_manager.restart_async(target, &cfg, &bridge_dir, deps.bun).await
        }
    }
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> AppResult<AppConfig> {
    state.load_config()
}

#[tauri::command]
pub fn save_config(config: AppConfig, state: State<'_, AppState>) -> AppResult<()> {
    state.save_config(&config)
}

#[tauri::command]
pub async fn bind_bridge(server_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let mut cfg = state.load_config()?;
    if !cfg.servers.iter().any(|s| s.id == server_id) {
        return Err(AppError::Config(format!("server not found: {}", server_id)));
    }
    cfg.bridge.bound_server_id = server_id;
    state.save_config(&cfg)?;
    let bridge_state = state.process_manager.get_state(ProcessTarget::Bridge);
    if bridge_state.state == crate::process::ProcessStateKind::Running {
        let bridge_dir = state.config_store.bridge_install_path(&cfg);
        let deps = bridge_check_deps();
        state.process_manager.restart_async(ProcessTarget::Bridge, &cfg, &bridge_dir, deps.bun).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn check_bridge_update(state: State<'_, AppState>) -> AppResult<bool> {
    let cfg = state.load_config()?;
    let installer = BridgeInstaller::new(state.config_store.bridge_install_path(&cfg));
    installer.check_update().await
}

#[tauri::command]
pub async fn update_bridge(state: State<'_, AppState>) -> AppResult<()> {
    let cfg = state.load_config()?;
    let installer = BridgeInstaller::new(state.config_store.bridge_install_path(&cfg));
    installer.update().await
}

#[tauri::command]
pub async fn reinstall_bridge(state: State<'_, AppState>) -> AppResult<()> {
    let cfg = state.load_config()?;
    let installer = BridgeInstaller::new(state.config_store.bridge_install_path(&cfg));
    installer.reinstall().await
}

#[tauri::command]
pub fn get_log_history(source: String, limit: usize, state: State<'_, AppState>) -> AppResult<Vec<LogEntry>> {
    let buf = crate::process::lock_or_recover(&state.log_buffer);
    let entries = if source == "all" {
        buf.recent_all(limit)
    } else {
        buf.recent(&source, limit)
    };
    Ok(entries)
}

#[tauri::command]
pub fn clear_logs(source: String, state: State<'_, AppState>) -> AppResult<()> {
    let mut buf = crate::process::lock_or_recover(&state.log_buffer);
    buf.clear(&source);
    Ok(())
}

#[tauri::command]
pub fn export_logs(source: String, state: State<'_, AppState>, app: tauri::AppHandle) -> AppResult<String> {
    let buf = crate::process::lock_or_recover(&state.log_buffer);
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
