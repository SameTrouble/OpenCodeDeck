# Final Code Review Fixes

## C1 (Critical): `wechat://logined` event never emitted
Fixed in `src-tauri/src/lib.rs` `on_log` closure. When a bridge info log contains
"logged in"/"login success" (case-insensitive) or "登录成功", emit `wechat://logined`.

## I1 (Important): `stop()` uses SIGKILL instead of SIGTERM
Fixed in `src-tauri/src/process/manager.rs` `stop()`. Added `nix` crate; on Unix send
SIGTERM then wait up to 5s before SIGKILL. Windows keeps `start_kill()`.

## I2 (Important): Blocking `reqwest` in async context
Fixed in `src-tauri/src/lib.rs`. Health check loop moved from
`tauri::async_runtime::spawn` to `std::thread::spawn` with a sleep loop.

## I3 (Important): `export_logs` writes without dialog
Fixed in `src-tauri/src/commands.rs`. Added `tauri-plugin-dialog`; `export_logs` now
shows a save dialog via `blocking_save_file` with a default filename, falling back to
the downloads/home dir. Returns error if cancelled.

## Verification
- `cargo check --manifest-path src-tauri/Cargo.toml` — pass
- `cargo test --manifest-path src-tauri/Cargo.toml` — 11 passed
- `npx tsc --noEmit` — pass
- `npm run build` — pass
