pub mod error;
pub mod config;
pub mod process;
pub mod bridge;
pub mod monitor;
pub mod state;
pub mod commands;
pub mod env_path;

use std::sync::{Arc, Mutex};
use tauri::{Manager, Emitter};

pub fn run() {
    env_path::augment_path();
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();

            let log_buffer: Arc<Mutex<monitor::LogBuffer>> = Arc::new(Mutex::new(monitor::LogBuffer::new(5000)));
            let log_buffer_for_cb = log_buffer.clone();

            let on_state: process::StateCallback = Arc::new({
                let handle = handle.clone();
                move |target, server_id, state| {
                    let target_str = if target == process::ProcessTarget::Server { "server" } else { "bridge" };
                    let _ = handle.emit("state://update", serde_json::json!({
                        "target": target_str,
                        "serverId": server_id,
                        "state": state
                    }));
                }
            });

            let on_log: process::LogCallback = Arc::new({
                let handle = handle.clone();
                move |entry: monitor::LogEntry| {
                    let mut buf = crate::process::lock_or_recover(&log_buffer_for_cb);
                    buf.push(entry.clone());
                    drop(buf);
                    if entry.source == "bridge" && entry.level == "info" {
                        let lower = entry.line.to_lowercase();
                        if lower.contains("logged in") || lower.contains("login success") || entry.line.contains("登录成功") {
                            let _ = handle.emit("wechat://logined", ());
                        }
                    }
                    let _ = handle.emit("log://entry", entry);
                }
            });

            let on_qr: process::QrCallback = Arc::new({
                let handle = handle.clone();
                move |ev: monitor::stdout_parser::WechatQrEvent| {
                    let _ = handle.emit("wechat://qrcode", ev);
                }
            });

            let pm = process::ProcessManager::new(on_state, on_log, on_qr);
            let app_state = state::AppState::new_with_buffer(pm, log_buffer);
            let config_version = app_state.config_version();
            app.manage(app_state);

            let handle2 = handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut last_version: u64 = 0;
                let mut checkers: std::collections::HashMap<String, monitor::health::HealthChecker> = std::collections::HashMap::new();
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    let state = handle2.state::<state::AppState>();
                    let server_states = state.process_manager.get_all_server_states();
                    let v = config_version.load(std::sync::atomic::Ordering::Relaxed);
                    if v != last_version {
                        last_version = v;
                        let cfg = state.load_config().unwrap_or_else(|_| config::ConfigStore::default_config());
                        checkers.clear();
                        for s in &cfg.servers {
                            checkers.insert(s.id.clone(), monitor::health::HealthChecker::new(&s.url));
                        }
                    }
                    for (id, ps) in &server_states {
                        if ps.state != process::ProcessStateKind::Running {
                            continue;
                        }
                        let healthy = match checkers.get(id) {
                            Some(c) => c.check_once().await,
                            None => false,
                        };
                        state.process_manager.set_health(process::ProcessTarget::Server, Some(id.clone()), healthy);
                        let _ = handle2.emit("health://update", serde_json::json!({
                            "target": "server",
                            "serverId": id,
                            "healthy": healthy
                        }));
                    }
                }
            });

            use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};
            use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};

            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&show_item, &sep, &quit_item])?;

            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png")).unwrap();
            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(cfg!(target_os = "macos"))
                .menu(&menu)
                .tooltip("OpenCodeDeck")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = handle.state::<state::AppState>();
                            state.process_manager.stop_all_servers().await;
                            let _ = state.process_manager.stop_async(process::ProcessTarget::Bridge).await;
                            handle.exit(0);
                        });
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            let main_window = app.get_webview_window("main").unwrap();
            let hide_handle = main_window.clone();
            main_window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = hide_handle.hide();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::start_process,
            commands::stop_process,
            commands::restart_process,
            commands::get_config,
            commands::save_config,
            commands::bind_bridge,
            commands::check_bridge_update,
            commands::update_bridge,
            commands::reinstall_bridge,
            commands::get_log_history,
            commands::clear_logs,
            commands::export_logs,
            commands::check_deps,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
