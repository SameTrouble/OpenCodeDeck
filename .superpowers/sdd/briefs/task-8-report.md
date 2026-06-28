# Task 8 Report — ProcessManager & Supervisor

## Created files
- `src-tauri/src/process/mod.rs` — module root re-exporting `ProcessManager`, `ProcessState`, `ProcessTarget`, `ProcessStateKind`.
- `src-tauri/src/process/manager.rs` — `ProcessManager` with `start_server`, `start_bridge`, `stop`, `restart`, `get_state`, `set_health`; owns an internal tokio runtime; holds server/bridge `ManagedProcess` behind `Arc<Mutex<>>`.
- `src-tauri/src/process/supervisor.rs` — `supervise` (server, stdout/stderr → log callbacks) and `supervise_with_qr` (bridge, additionally feeds stdout through `StdoutParser` and emits `WechatQrEvent`).

## Modified files
- `src-tauri/src/lib.rs` — added `pub mod process;` after `pub mod monitor;`.

## Compile fixes applied (vs. plan code)
1. **`stop()` duplicate match block** — removed the erroneous first `match mp.state.state { ProcessTarget::Server => ... }` that matched `ProcessStateKind` against `ProcessTarget` (would not compile). Kept only the correct `ProcessStateKind::Running | Starting` arm. Renamed unused `pid` → `_pid`.
2. **`stop()` `.await` outside async** — the plan called `child.kill().await` then a second `rt.block_on(child.wait())` at sync level (`E0728`). Consolidated into a single `rt.block_on` block using `tokio::select!`: wait up to 5s after `start_kill()`, then force `start_kill()` again + `wait().await` inside the same async block, returning the exit code.
3. **`MutexGuard` held across `.await` in supervisor** (`future not Send`) — `mp.child.as_mut().expect(...)` kept the guard alive across `child.wait().await`. Refactored both `supervise` and `supervise_with_qr` to take the child out of the mutex inside a nested block before awaiting:
   ```rust
   let child = { let mut mp = child_ref.lock().unwrap(); mp.child.take() };
   match child { Some(mut c) => c.wait().await..., None => None }
   ```
   (`drop(mp)` alone was insufficient — the borrow checker still considered the guard live across the await.)
4. **Private-interface warnings** — `supervise`/`supervise_with_qr` were `pub` but take `pub(crate) ManagedProcess`. Lowered both to `pub(crate)` (they are only called from `manager.rs` via `super::supervisor::`).

## Verification
`cargo check --manifest-path src-tauri/Cargo.toml` → finished with no errors and no warnings.

## Commit
`<hash>` — `feat(process): add ProcessManager and supervisor with stdout/stderr streaming`
