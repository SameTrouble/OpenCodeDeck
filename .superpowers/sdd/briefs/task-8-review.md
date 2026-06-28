# Task 8 Review — ProcessManager & Supervisor

## Verdicts

- **Spec conformance:** ✅ PASS — all required interfaces present with correct signatures.
- **Code quality:** PASS — all 4 compile fixes are correct and well-justified; no issues introduced by the implementer.

## Spec conformance

| Required interface | Location | Status |
|---|---|---|
| `ProcessTarget` enum (Server/Bridge) | `manager.rs:37-41` | ✅ |
| `ProcessState` struct (state/pid/started_at/uptime_sec/exit_code/healthy) | `manager.rs:17-26` | ✅ all 6 fields |
| `ProcessStateKind` enum (Stopped/Starting/Running/Stopping/Failed) | `manager.rs:7-15` | ✅ all 5 variants |
| `ProcessManager::new(state_cb, log_cb, qr_cb)` | `manager.rs:69` | ✅ |
| `start_server` | `manager.rs:96` | ✅ |
| `start_bridge` | `manager.rs:144` | ✅ |
| `stop` | `manager.rs:199` | ✅ |
| `restart` | `manager.rs:240` | ✅ |
| `get_state` | `manager.rs:252` | ✅ (also computes live `uptime_sec`) |
| `set_health` | `manager.rs:263` | ✅ |
| `supervisor::supervise` | `supervisor.rs:48` | ✅ (`pub(crate)`) |
| `supervisor::supervise_with_qr` | `supervisor.rs:97` | ✅ (`pub(crate)`) |

`mod.rs` re-exports `ProcessManager`, `ProcessState`, `ProcessTarget`, `ProcessStateKind`. `lib.rs` adds `pub mod process;`. All interfaces satisfied.

## Code quality — the 4 compile fixes

### Fix 1: Removed duplicate match block in `stop()` — ✅ Correct
The plan reportedly had `match mp.state.state { ProcessTarget::Server => ... }`, a type error (matching `ProcessStateKind` against `ProcessTarget` variants). The surviving single match (`manager.rs:204-210`) on `ProcessStateKind::Running | Starting` is correct. `_pid` is assigned but unused — harmless, correctly silenced with the underscore prefix.

### Fix 2: Consolidated `.await` in `stop()` into a single `block_on` — ✅ Correct (with plan-inherited caveat)
`manager.rs:214-225` takes the child out of the mutex, calls `start_kill()`, then runs a single `rt.block_on` with `tokio::select!`:
- waits for `child.wait()` up to 5s, OR
- on timeout, calls `start_kill()` again then `child.wait().await`.

This correctly resolves E0728 (`.await` outside async). The child is owned by the async block, so no `MutexGuard` is held across the await. Structurally sound.

**Caveat (plan-inherited, not implementer-introduced):** `tokio::process::Child::start_kill()` sends **SIGKILL** on Unix, not SIGTERM. So the actual sequence is SIGKILL → 5s → SIGKILL, not the global constraint's "SIGTERM → 5s → SIGKILL". The plan itself used `child.kill().await` (also SIGKILL), so the implementer faithfully consolidated the plan's logic. The second `start_kill()` in the timeout arm is redundant but harmless. Flagging for awareness; not a quality failure of the fix.

### Fix 3: `MutexGuard` held across `.await` in supervisor — ✅ Correct and safe
Both `supervise` (`supervisor.rs:73-82`) and `supervise_with_qr` (`supervisor.rs:126-135`) now take the child out of the mutex in a nested block before awaiting `c.wait().await`:
```rust
let child = { let mut mp = child_ref.lock().unwrap(); mp.child.take() };
match child { Some(mut c) => c.wait().await..., None => None }
```
This correctly resolves "future not Send". `drop(mp)` alone would indeed have been insufficient since the borrow checker tracks the guard's lifetime through the await.

**Safety of the take:** After `mp.child.take()`, the mutex holds `child: None`. If `stop()` races and takes the child first, the supervisor gets `None` and skips `wait()` — no double-kill, no panic. One process owns the child at any time. ✅

**Minor design-level note (plan-inherited):** If `stop()` takes the child first and sets state to `Stopped`, the supervisor may then overwrite it with `Failed` (exit_code `None`). This is a benign state race present in the plan's design, not introduced by this fix.

### Fix 4: `pub` → `pub(crate)` for supervise/supervise_with_qr — ✅ Acceptable
The functions take `pub(crate) ManagedProcess` as a parameter, so `pub` visibility triggers E0446 (private interface leaked). Lowering to `pub(crate)` is the correct fix. Both are only called from `manager.rs:138` and `manager.rs:193` via `super::supervisor::`, so crate-level visibility is exactly right. No external consumer needs them.

## Other checks

- **No code comments:** Grep for `//` and `/*` across `src-tauri/src/process/` → no matches. ✅
- **`cargo check`:** Report states it finishes with no errors/warnings. The diff shows no obvious type/borrow issues; trusting the report.
- **Stop order (bridge first, then server):** `stop()` takes a `ProcessTarget` parameter — cross-target ordering is the caller's responsibility, not this module's. No violation in scope.
- **`kill_on_drop(true)`** set on both server and bridge commands (`manager.rs:112`, `166`) — good defensive measure.

## Summary
Implementation matches the spec on every required interface. All four compile fixes are correct: the duplicate-match removal, the `block_on`+`select!` consolidation, the child-take-out-of-mutex pattern, and the `pub(crate)` visibility downgrade. The only behavioral nuance — `start_kill()` being SIGKILL rather than SIGTERM — is inherited from the plan, not introduced here.

**Spec: ✅ | Quality: PASS**
