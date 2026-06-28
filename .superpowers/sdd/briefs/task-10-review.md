# Task 10 Review — AppState and Tauri commands wiring

## 1. Spec conformance

### 16 Tauri commands present and registered
All 16 commands exist in `commands.rs` and are wired in `lib.rs` `invoke_handler`:

| # | Command | Defined | Registered |
|---|---------|---------|------------|
| 1 | `get_state` | commands.rs:17 | lib.rs:69 ✅ |
| 2 | `start_process` | commands.rs:33 | lib.rs:70 ✅ |
| 3 | `stop_process` | commands.rs:51 | lib.rs:71 ✅ |
| 4 | `restart_process` | commands.rs:57 | lib.rs:72 ✅ |
| 5 | `start_all` | commands.rs:89 | lib.rs:73 ✅ |
| 6 | `stop_all` | commands.rs:92 | lib.rs:74 ✅ |
| 7 | `restart_all` | commands.rs:95 | lib.rs:75 ✅ |
| 8 | `get_config` | commands.rs:98 | lib.rs:76 ✅ |
| 9 | `save_config` | commands.rs:103 | lib.rs:77 ✅ |
| 10 | `check_bridge_update` | commands.rs:108 | lib.rs:78 ✅ |
| 11 | `update_bridge` | commands.rs:115 | lib.rs:79 ✅ |
| 12 | `reinstall_bridge` | commands.rs:122 | lib.rs:80 ✅ |
| 13 | `get_log_history` | commands.rs:129 | lib.rs:81 ✅ |
| 14 | `clear_logs` | commands.rs:140 | lib.rs:82 ✅ |
| 15 | `export_logs` | commands.rs:147 | lib.rs:83 ✅ |
| 16 | `check_deps` | commands.rs:164 | lib.rs:84 ✅ |

### Free functions for Task 11 tray menu
- `do_start_all` — commands.rs:64 ✅ (takes `&AppState`, no `State<'_>` wrapper)
- `do_stop_all` — commands.rs:77 ✅
- `do_restart_all` — commands.rs:83 ✅ (delegates to stop then start)

These are correctly separated from the `#[tauri::command]` wrappers (`start_all`/`stop_all`/`restart_all`) which call `do_*_all(state.inner())`. This is the right shape: tray handlers can call `do_*_all(&app_state)` directly without a Tauri `State<'_>`.

### AppState shape
`state.rs:6-10`:
```rust
pub struct AppState {
    pub config_store: ConfigStore,
    pub process_manager: ProcessManager,
    pub log_buffer: Arc<Mutex<LogBuffer>>,
}
```
All three required fields present ✅. Convenience helpers `load_config`/`save_config` are thin wrappers over `config_store` — fine.

### lib.rs setup
- Creates `log_buffer: Arc<Mutex<LogBuffer>>` with capacity 5000 (lib.rs:18) ✅ matches ring-buffer constraint.
- Three callbacks constructed with cloned `AppHandle`:
  - `on_state: StateCallback` emits `state://update` (lib.rs:21-27) ✅
  - `on_log: LogCallback` pushes to buffer then emits `log://entry` (lib.rs:29-37) ✅
  - `on_qr: QrCallback` emits `wechat://qrcode` (lib.rs:39-44) ✅
- `ProcessManager::new(on_state, on_log, on_qr)` (lib.rs:46) ✅
- `AppState::new_with_buffer(pm, log_buffer)` then `app.manage(...)` (lib.rs:47-48) ✅
- Health check loop spawned via `tauri::async_runtime::spawn` (lib.rs:50-64) ✅

### Events emitted
- `state://update` — lib.rs:25 ✅
- `log://entry` — lib.rs:35 ✅
- `wechat://qrcode` — lib.rs:42 ✅
- `health://update` — lib.rs:61 ✅

All four required events present.

**Spec conformance: ✅**

## 2. Code quality

### `check_deps` naming conflict resolution
- `bridge/env_check.rs:26` defines `pub fn check_deps() -> DepStatus`.
- `commands.rs:164` defines `pub fn check_deps() -> AppResult<DepStatus>` (the Tauri command).
- Import at commands.rs:2: `use crate::bridge::{check_deps as bridge_check_deps, ...}`.
- Call sites use `bridge_check_deps()` (commands.rs:50, 66, 72, 171).

Correct. The alias resolves the name collision cleanly; the public command keeps the spec-required name `check_deps`, and internal calls go through the alias. No ambiguity, no `use ... as` at call sites. ✅

### `process/mod.rs` re-export change
Before: `pub use manager::{ProcessManager, ProcessState, ProcessTarget, ProcessStateKind};`
After:  `pub use manager::{ProcessManager, ProcessState, ProcessTarget, ProcessStateKind, StateCallback, LogCallback, QrCallback};`

Necessary — `lib.rs` refers to `process::StateCallback`, `process::LogCallback`, `process::QrCallback` (lib.rs:21, 29, 39). Without the re-export these would have to be referenced as `process::manager::StateCallback` etc. The re-export is the idiomatic Tauri/Rust pattern and keeps `lib.rs` clean. The three type aliases do exist in `manager.rs:55-57`. ✅

### Health check loop
lib.rs:50-64:
```rust
tauri::async_runtime::spawn(async move {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let state = handle2.state::<state::AppState>();
        let server_state = state.process_manager.get_state(process::ProcessTarget::Server);
        if server_state.state == process::ProcessStateKind::Running {
            let cfg = state.load_config().unwrap_or_else(|_| config::ConfigStore::default_config());
            let checker = monitor::health::HealthChecker::new(&cfg.server.opencode_server_url);
            let healthy = checker.check_once();
            state.process_manager.set_health(process::ProcessTarget::Server, healthy);
            let _ = handle2.emit("health://update", serde_json::json!({ "target": "server", "healthy": healthy }));
        }
    }
});
```
- Polls every 5s via `tokio::time::sleep(Duration::from_secs(5))` ✅
- Only acts when `server_state.state == ProcessStateKind::Running` ✅
- URL sourced from `cfg.server.opencode_server_url` ✅
- `HealthChecker::new` appends `/session/status` (health.rs:10) — matches spec `GET {opencodeServerUrl}/session/status` ✅
- `check_once` performs `client.get(&self.url).send()` (health.rs:19) ✅
- Graceful config fallback via `unwrap_or_else` so a corrupt config won't panic the loop ✅
- `set_health` updates manager state and (per manager.rs:263-269) triggers the state callback, so a health change also surfaces through `state://update`. Good side-effect hygiene.

One observation (not a defect): `check_once` uses `reqwest::blocking::Client` inside an async task. This blocks the async worker thread for up to the 3s timeout. Given Tauri's `async_runtime` uses a multi-threaded tokio runtime and the call is short-lived, this is acceptable for now but worth noting for Task 11/12 if latency becomes an issue. Not blocking.

### No code comments
- `commands.rs`: no `//` or `/*` comments ✅
- `lib.rs`: no comments ✅
- `state.rs`: no comments ✅
- `process/mod.rs`: no comments ✅

### No unused imports
- `commands.rs` imports: `State`, `bridge_check_deps`/`DepStatus`/`BridgeInstaller`, `AppConfig`/`renderer`, `AppError`/`AppResult`, `LogEntry`, `ProcessState`/`ProcessTarget`, `AppState` — all used. `AppError` used in `parse_target` (commands.rs:28). ✅
- `lib.rs` imports: `Arc`/`Mutex` (lib.rs:18), `Manager` (`.state::<>()`, `.manage()`), `Emitter` (`.emit()`) — all used. ✅
- `state.rs` imports: `Arc`/`Mutex`, `ConfigStore`/`AppConfig`, `ProcessManager`, `LogBuffer` — all used. ✅

`cargo check` passes clean (no warnings beyond an unrelated deprecated `~/.cargo/config` notice). ✅

### Additional observations (non-blocking)
- `export_logs` (commands.rs:147-161) writes to `dirs::download_dir().or_else(dirs::home_dir()).unwrap_or_default()`. The `unwrap_or_default()` yields an empty `PathBuf` on platforms without a download/home dir, which would write to CWD — acceptable fallback, and realistically never hit on macOS/Windows/Linux.
- `get_log_history`/`clear_logs` use `.lock().unwrap()` — panics on poison. Consistent with the rest of the codebase's locking style (e.g. `manager.rs`), so this is an accepted project convention, not a regression.
- `do_stop_all` stops Bridge before Server (commands.rs:77-81). Correct ordering — bridge depends on the opencode server, so tear down the dependent first.
- `start_process` for Bridge re-checks install and re-writes bridge files each call (commands.rs:38-47). Idempotent and safe; `do_start_all` mirrors this. Slight duplication between `start_process`'s Bridge branch and `do_start_all`, but extracting a shared helper would add indirection for two call sites — current form is readable.
- `FullState` uses `#[serde(rename_all = "camelCase")]` (commands.rs:9-10), consistent with `LogEntry`, `DepStatus`, and config structs. ✅

**Code quality: PASS**

## Verdicts

- **Spec conformance: ✅** — all 16 commands, 3 free functions, AppState shape, setup wiring, 5s health loop, and 4 events are present and correct.
- **Code quality: PASS** — `cargo check` clean, no comments, no unused imports, naming-conflict alias is correct, re-export is necessary and idiomatic, health loop logic is sound.
