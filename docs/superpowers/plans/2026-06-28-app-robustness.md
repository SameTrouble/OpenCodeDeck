# App Robustness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Systematically fix 18 robustness issues (P0/P1/P2) in OpenCodeDeck without changing functionality or the Tauri Command/Event contract.

**Architecture:** Targeted in-place fixes to existing modules. New small utilities (lock helper, command resolver, formatError, ErrorBoundary). No module renames, no new features. Frontend invoke calls unchanged; only internal implementations and one signature change on `restart_async` (internal).

**Tech Stack:** Rust (Tauri v2, tokio, serde, thiserror, reqwest, `which` crate—new), React 19 + TypeScript + Vite + shadcn/ui + sonner.

## Global Constraints

- Rust edition 2021; dependencies in `src-tauri/Cargo.toml`.
- New dep: `which = "7"` (cross-platform PATH resolution, no transitive deps beyond libc).
- `reqwest` drops `blocking` feature, keeps default (async).
- Frontend: no new npm deps. No test framework added (YAGNI—MVP).
- Tauri Command/Event signatures in `commands.rs` stay backward-compatible (invoke args from frontend unchanged). Exception: `start_process`/`start_all`/`do_start_all` change `fn` → `async fn` (Tauri handles transparently, frontend `invoke` calls unchanged).
- Build verification: `cargo build`, `cargo test`, `cargo clippy` in `src-tauri/`; `npm run build` at root.
- Commit style: `fix(<scope>): <subject>` or `refactor(<scope>): <subject>`, matching existing repo conventions.

## File Structure

**Rust (`src-tauri/src/`):**
- `process/mod.rs` — add `lock_or_recover` helper, re-export.
- `process/manager.rs` — `ManagedProcess.stopping` flag, `restart_async` signature, use `resolve_command`, use `lock_or_recover`.
- `process/supervisor.rs` — no-panic child take, honor `stopping`, log IO errors, use `lock_or_recover`.
- `bridge/env_check.rs` — use `which::which` instead of hand-rolled `which`/`where`.
- `bridge/installer.rs` — git ops `async`, use `resolve_command`.
- `config/store.rs` — atomic save (tmp+rename), corrupt-tolerant load (backup+default).
- `monitor/health.rs` — `check_once` → `async fn`, async reqwest client.
- `commands.rs` — `start_process`/`start_all`/`do_start_all` → `async`; pass `bridge_dir` to `restart_async`; bump `config_version` on save.
- `lib.rs` — health check via tokio + config version; tray `start_all`/`quit` async; use `lock_or_recover`.
- `error.rs` — add `From<which::Error>` for `AppError`.
- `state.rs` — add `config_version: Arc<AtomicU64>`.
- `process/mod.rs` (new helper) `command_util.rs` — `resolve_command(name) -> AppResult<Command>`.

**Frontend (`src/`):**
- `lib/utils.ts` — add `formatError(e: unknown): string`.
- `hooks/useTauriEvent.ts` — cancel-safe listen.
- `hooks/useProcessState.tsx` — `console.error` on refresh fail.
- `components/ErrorBoundary.tsx` — new class component.
- `main.tsx` — wrap `<App/>` in `<ErrorBoundary>`.
- `pages/Dashboard.tsx`, `pages/Config.tsx`, `pages/Bridge.tsx`, `pages/Channels.tsx`, `components/ProcessCard.tsx`, `components/LogView.tsx` — use `formatError` in catch handlers.

---

### Task 1: Add `which` crate, `resolve_command` helper, and `From<which::Error>`

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/process/command_util.rs`
- Modify: `src-tauri/src/process/mod.rs`
- Modify: `src-tauri/src/error.rs`

**Interfaces:**
- Produces: `crate::process::resolve_command(name: &str) -> AppResult<std::process::Command>`; `AppError` gains `From<which::Error>`.

- [ ] **Step 1: Add `which` dependency**

Modify `src-tauri/Cargo.toml` deps section—add after the `nix` line:

```toml
which = "7"
```

- [ ] **Step 2: Add `From<which::Error>` to `AppError`**

Modify `src-tauri/src/error.rs` — add this impl after the existing `From<serde_json::Error>` impl:

```rust
impl From<which::Error> for AppError {
    fn from(e: which::Error) -> Self {
        AppError::EnvNotFound(e.to_string())
    }
}
```

- [ ] **Step 3: Create `command_util.rs`**

Create `src-tauri/src/process/command_util.rs`:

```rust
use std::process::Command;
use crate::error::AppResult;

/// Resolve `name` to an absolute path via PATH (cross-platform, including
/// Windows `.cmd`/`.bat`/`.exe` extensions) and return a `Command` for it.
pub fn resolve_command(name: &str) -> AppResult<Command> {
    let path = which::which(name)?;
    Ok(Command::new(path))
}
```

- [ ] **Step 4: Wire up module**

Modify `src-tauri/src/process/mod.rs` — add the submodule and re-export. Replace the entire file with:

```rust
pub mod manager;
pub mod supervisor;
pub mod command_util;

pub use manager::{ProcessManager, ProcessState, ProcessTarget, ProcessStateKind, StateCallback, LogCallback, QrCallback};
pub use command_util::resolve_command;

use std::sync::{Mutex, MutexGuard};

pub(crate) fn lock_or_recover<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}
```

- [ ] **Step 5: Verify build**

Run: `cargo build` (in `src-tauri/`)
Expected: compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/error.rs src-tauri/src/process/mod.rs src-tauri/src/process/command_util.rs
git commit -m "feat(process): add resolve_command helper and lock_or_recover util"
```

---

### Task 2: Replace all `lock().unwrap()` with `lock_or_recover`

**Files:**
- Modify: `src-tauri/src/process/manager.rs`
- Modify: `src-tauri/src/process/supervisor.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `crate::process::lock_or_recover` (Task 1).

- [ ] **Step 1: Replace in `manager.rs`**

In `src-tauri/src/process/manager.rs`, replace every occurrence of `.lock().unwrap()` with `crate::process::lock_or_recover(&...)`. Concretely, replace these call sites (use `replaceAll: false` per unique match—each has distinct surrounding code):

Replace `self.server.lock().unwrap()` → `crate::process::lock_or_recover(&self.server)` (appears in `start_server`, `emit_state` via target_ref? No—target_ref returns the Arc; the lock call is at call sites).

The distinct call sites in `manager.rs`:
- `self.target_ref(target).lock().unwrap()` (in `emit_state`)
- `self.server.lock().unwrap()` (in `start_server`, twice)
- `self.bridge.lock().unwrap()` (in `start_bridge`, twice)
- `self.target_ref(target).lock().unwrap()` (in `stop_async`, twice)
- `self.target_ref(target).lock().unwrap()` (in `get_state`)
- `mp_ref.lock().unwrap()` (in `stop_async`, twice)
- `self.target_ref(target).lock().unwrap()` (in `set_health`)

Run this sed-equivalent via the Edit tool with `replaceAll: true` for each pattern:
- oldString `self.target_ref(target).lock().unwrap()` → newString `crate::process::lock_or_recover(self.target_ref(target))` (replaceAll: true)
- oldString `self.server.lock().unwrap()` → newString `crate::process::lock_or_recover(&self.server)` (replaceAll: true)
- oldString `self.bridge.lock().unwrap()` → newString `crate::process::lock_or_recover(&self.bridge)` (replaceAll: true)
- oldString `mp_ref.lock().unwrap()` → newString `crate::process::lock_or_recover(&mp_ref)` (replaceAll: true)

- [ ] **Step 2: Replace in `supervisor.rs`**

In `src-tauri/src/process/supervisor.rs`, replace:
- oldString `process.lock().unwrap()` → newString `crate::process::lock_or_recover(&process)` (replaceAll: true)
- oldString `child_ref.lock().unwrap()` → newString `crate::process::lock_or_recover(&child_ref)` (replaceAll: true)
- oldString `parser.lock().unwrap()` → newString `crate::process::lock_or_recover(&parser)` (replaceAll: true)

- [ ] **Step 3: Replace in `commands.rs`**

In `src-tauri/src/commands.rs`, replace:
- oldString `state.log_buffer.lock().unwrap()` → newString `crate::process::lock_or_recover(&state.log_buffer)` (replaceAll: true)

- [ ] **Step 4: Replace in `lib.rs`**

In `src-tauri/src/lib.rs`, replace:
- oldString `let mut buf = log_buffer_for_cb.lock().unwrap();` → newString `let mut buf = crate::process::lock_or_recover(&log_buffer_for_cb);`

- [ ] **Step 5: Verify build**

Run: `cargo build` (in `src-tauri/`)
Expected: compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/process/manager.rs src-tauri/src/process/supervisor.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "fix(process): use lock_or_recover to avoid panic on poisoned mutex"
```

---

### Task 3: supervisor honors `stopping` flag and never panics on missing child

**Files:**
- Modify: `src-tauri/src/process/manager.rs`
- Modify: `src-tauri/src/process/supervisor.rs`

**Interfaces:**
- Consumes: `crate::process::lock_or_recover` (Task 1).
- Produces: `ManagedProcess.stopping: bool` field; supervisor sets `Stopped` (not `Failed`) when `stopping` was true.

- [ ] **Step 1: Add `stopping` field to `ManagedProcess`**

In `src-tauri/src/process/manager.rs`, replace the `ManagedProcess` struct and its `new()`:

```rust
pub(crate) struct ManagedProcess {
    pub state: ProcessState,
    pub child: Option<Child>,
    pub started_at_instant: Option<Instant>,
    pub stopping: bool,
}

impl ManagedProcess {
    fn new() -> Self {
        Self { state: ProcessState::default(), child: None, started_at_instant: None, stopping: false }
    }
}
```

- [ ] **Step 2: Set `stopping = true` in `stop_async`**

In `src-tauri/src/process/manager.rs` `stop_async`, the block that matches `Running | Starting` and sets `Stopping` — add `mp.stopping = true;`. Replace:

```rust
            match mp.state.state {
                ProcessStateKind::Running | ProcessStateKind::Starting => {
                    mp.state.state = ProcessStateKind::Stopping;
                    _pid = mp.state.pid;
                }
                _ => return Ok(()),
            }
```

with:

```rust
            match mp.state.state {
                ProcessStateKind::Running | ProcessStateKind::Starting => {
                    mp.state.state = ProcessStateKind::Stopping;
                    mp.stopping = true;
                    _pid = mp.state.pid;
                }
                _ => return Ok(()),
            }
```

- [ ] **Step 3: Clear `stopping` in `stop_async` after successful stop**

In `stop_async`, the block that sets the final `Stopped` state — add `mp.stopping = false;`. Replace:

```rust
            {
                let mut mp = crate::process::lock_or_recover(&mp_ref);
                mp.state = ProcessState {
                    state: ProcessStateKind::Stopped,
                    pid: None, started_at: None, uptime_sec: None,
                    exit_code, healthy: None,
                };
                mp.started_at_instant = None;
            }
```

with:

```rust
            {
                let mut mp = crate::process::lock_or_recover(&mp_ref);
                mp.state = ProcessState {
                    state: ProcessStateKind::Stopped,
                    pid: None, started_at: None, uptime_sec: None,
                    exit_code, healthy: None,
                };
                mp.started_at_instant = None;
                mp.stopping = false;
            }
```

- [ ] **Step 4: Rewrite `supervise` exit handling**

In `src-tauri/src/process/supervisor.rs`, replace the `supervise` function body from the child-take block onward. Replace:

```rust
    let exit_code = {
        let child = {
            let mut mp = child_ref.lock().unwrap();
            mp.child.take()
        };
        match child {
            Some(mut c) => c.wait().await.ok().and_then(|s| s.code()),
            None => None,
        }
    };
    {
        let mut mp = child_ref.lock().unwrap();
        mp.state = ProcessState {
            state: ProcessStateKind::Failed,
            pid: None, started_at: None, uptime_sec: None,
            exit_code, healthy: None,
        };
        mp.started_at_instant = None;
        let state = mp.state.clone();
        drop(mp);
        on_state(target, state);
    }
```

with:

```rust
    let exit_code = {
        let child = {
            let mut mp = crate::process::lock_or_recover(&child_ref);
            mp.child.take()
        };
        match child {
            Some(mut c) => c.wait().await.ok().and_then(|s| s.code()),
            None => None,
        }
    };
    {
        let mut mp = crate::process::lock_or_recover(&child_ref);
        let next_state = if mp.stopping {
            mp.stopping = false;
            ProcessStateKind::Stopped
        } else {
            ProcessStateKind::Failed
        };
        mp.state = ProcessState {
            state: next_state,
            pid: None, started_at: None, uptime_sec: None,
            exit_code, healthy: None,
        };
        mp.started_at_instant = None;
        let state = mp.state.clone();
        drop(mp);
        on_state(target, state);
    }
```

- [ ] **Step 5: Apply the same change to `supervise_with_qr`**

In `src-tauri/src/process/supervisor.rs`, the `supervise_with_qr` function has an identical exit-handling block. Replace the same pattern (from `let exit_code = {` through the closing `}` of the state-update block) with the identical new code from Step 4. The `mp.child.as_mut().expect("no child")` at the top of both functions is NOT changed by this step (that is Task 4's concern—but note: after Step 4 the child-take already uses `lock_or_recover`; the `as_mut().expect` at top still exists and is addressed in Task 4).

Wait—Task 4 is folded here. Let me restate: also fix the `expect("no child")` at the top of both `supervise` and `supervise_with_qr`.

Replace in both functions (the top block):

```rust
    let (stdout, stderr, child_ref) = {
        let mut mp = process.lock().unwrap();
        let child = mp.child.as_mut().expect("no child");
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        (stdout, stderr, process.clone())
    };
```

with:

```rust
    let (stdout, stderr, child_ref) = {
        let mut mp = crate::process::lock_or_recover(&process);
        let child = match mp.child.as_mut() {
            Some(c) => c,
            None => return,
        };
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        (stdout, stderr, process.clone())
    };
```

Apply to both `supervise` and `supervise_with_qr` (the top block is identical in both).

- [ ] **Step 6: Verify build**

Run: `cargo build` (in `src-tauri/`)
Expected: compiles with no errors.

- [ ] **Step 7: Verify existing tests pass**

Run: `cargo test` (in `src-tauri/`)
Expected: all existing tests pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/process/manager.rs src-tauri/src/process/supervisor.rs
git commit -m "fix(process): supervisor honors stopping flag and never panics on missing child"
```

---

### Task 4: `restart_async` rewrites bridge config and takes `bridge_dir` param (#2, #17)

**Files:**
- Modify: `src-tauri/src/process/manager.rs`
- Modify: `src-tauri/src/commands.rs`

**Interfaces:**
- Produces: `ProcessManager::restart_async(target, cfg, bridge_dir: &Path, use_bun: bool) -> AppResult<ProcessState>`. The old `ConfigStore::new()` inside is removed.

- [ ] **Step 1: Change `restart_async` signature and bridge branch**

In `src-tauri/src/process/manager.rs`, replace the `restart_async` method:

```rust
    pub async fn restart_async(&self, target: ProcessTarget, cfg: &crate::config::AppConfig, use_bun: bool) -> AppResult<ProcessState> {
        self.stop_async(target).await?;
        match target {
            ProcessTarget::Server => self.start_server(cfg.server.port, &cfg.server.cwd, &cfg.server.extra_env),
            ProcessTarget::Bridge => {
                let store = crate::config::ConfigStore::new();
                let bridge_dir = store.bridge_install_path(cfg);
                self.start_bridge(&bridge_dir, use_bun)
            }
        }
    }
```

with:

```rust
    pub async fn restart_async(&self, target: ProcessTarget, cfg: &crate::config::AppConfig, bridge_dir: &Path, use_bun: bool) -> AppResult<ProcessState> {
        self.stop_async(target).await?;
        match target {
            ProcessTarget::Server => self.start_server(cfg.server.port, &cfg.server.cwd, &cfg.server.extra_env),
            ProcessTarget::Bridge => {
                crate::config::renderer::write_bridge_files(cfg, bridge_dir)?;
                self.start_bridge(bridge_dir, use_bun)
            }
        }
    }
```

Also update the sync `restart` wrapper below it. Replace:

```rust
    pub fn restart(&self, target: ProcessTarget, cfg: &crate::config::AppConfig, use_bun: bool) -> AppResult<ProcessState> {
        let rt = &self.runtime;
        rt.block_on(self.restart_async(target, cfg, use_bun))
    }
```

with:

```rust
    pub fn restart(&self, target: ProcessTarget, cfg: &crate::config::AppConfig, bridge_dir: &Path, use_bun: bool) -> AppResult<ProcessState> {
        let rt = &self.runtime;
        rt.block_on(self.restart_async(target, cfg, bridge_dir, use_bun))
    }
```

Add the `use std::path::Path;` import at the top of `manager.rs` if not already present (it is not currently imported—`std::path::PathBuf` is not in this file; bridge_dir was `&std::path::Path` inline before). Add after the existing `use` lines:

```rust
use std::path::Path;
```

- [ ] **Step 2: Update `restart_process` command**

In `src-tauri/src/commands.rs`, replace the `restart_process` command:

```rust
#[tauri::command]
pub async fn restart_process(target: String, state: State<'_, AppState>) -> AppResult<ProcessState> {
    let target = parse_target(&target)?;
    let cfg = state.load_config()?;
    let deps = bridge_check_deps();
    state.process_manager.restart_async(target, &cfg, deps.bun).await
}
```

with:

```rust
#[tauri::command]
pub async fn restart_process(target: String, state: State<'_, AppState>) -> AppResult<ProcessState> {
    let target = parse_target(&target)?;
    let cfg = state.load_config()?;
    let bridge_dir = state.config_store.bridge_install_path(&cfg);
    let deps = bridge_check_deps();
    state.process_manager.restart_async(target, &cfg, &bridge_dir, deps.bun).await
}
```

- [ ] **Step 3: Verify build**

Run: `cargo build` (in `src-tauri/`)
Expected: compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/process/manager.rs src-tauri/src/commands.rs
git commit -m "fix(process): restart_async rewrites bridge config and takes bridge_dir param"
```

---

### Task 5: Use `resolve_command` in `start_server`, `start_bridge`, `env_check`, `installer`

**Files:**
- Modify: `src-tauri/src/process/manager.rs`
- Modify: `src-tauri/src/bridge/env_check.rs`

> Note: `installer.rs` git ops become async in Task 8; we'll switch it to `resolve_command` there. Here we cover the synchronous call sites that remain sync after Task 8: `start_server`, `start_bridge`, and `env_check`.

**Interfaces:**
- Consumes: `crate::process::resolve_command` (Task 1).

- [ ] **Step 1: Replace server/bridge command construction in `manager.rs`**

In `src-tauri/src/process/manager.rs`, replace in `start_server`:

```rust
        let mut cmd = tokio::process::Command::new("opencode");
        cmd.arg("serve").arg("--port").arg(port.to_string());
```

with:

```rust
        let mut cmd = tokio::process::Command::from(crate::process::resolve_command("opencode")?);
        cmd.arg("serve").arg("--port").arg(port.to_string());
```

Replace in `start_bridge`:

```rust
        let mut cmd = if use_bun {
            let mut c = tokio::process::Command::new("bun");
            c.arg("run").arg("src/index.ts");
            c
        } else {
            let mut c = tokio::process::Command::new("npx");
            c.arg("tsx").arg("src/index.ts");
            c
        };
```

with:

```rust
        let mut cmd = if use_bun {
            let mut c = tokio::process::Command::from(crate::process::resolve_command("bun")?);
            c.arg("run").arg("src/index.ts");
            c
        } else {
            let mut c = tokio::process::Command::from(crate::process::resolve_command("npx")?);
            c.arg("tsx").arg("src/index.ts");
            c
        };
```

- [ ] **Step 2: Rewrite `env_check.rs` using `which::which`**

Replace the entire contents of `src-tauri/src/bridge/env_check.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepStatus {
    pub opencode: bool,
    pub bun: bool,
    pub node: bool,
    pub npm: bool,
    pub git: bool,
}

fn which(cmd: &str) -> bool {
    which::which(cmd).is_ok()
}

pub fn check_deps() -> DepStatus {
    DepStatus {
        opencode: which("opencode"),
        bun: which("bun"),
        node: which("node"),
        npm: which("npm"),
        git: which("git"),
    }
}
```

- [ ] **Step 3: Verify build**

Run: `cargo build` (in `src-tauri/`)
Expected: compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/process/manager.rs src-tauri/src/bridge/env_check.rs
git commit -m "fix(process): resolve commands via PATH for cross-platform support"
```

---

### Task 6: Atomic config save + corrupt-tolerant load (#7, #8)

**Files:**
- Modify: `src-tauri/src/config/store.rs`

- [ ] **Step 1: Write failing test for corrupt-tolerant load**

In `src-tauri/src/config/store.rs`, add a `#[cfg(test)]` module (append to file) with:

```rust
#[cfg(test)]
mod robustness_tests {
    use super::*;
    use std::io::Write;

    fn temp_store() -> (ConfigStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore { config_dir: dir.path().to_path_buf() };
        (store, dir)
    }

    #[test]
    fn load_backs_up_corrupt_file_and_returns_default() {
        let (store, _dir) = temp_store();
        std::fs::create_dir_all(store.config_dir()).unwrap();
        std::fs::write(store.config_path(), "{ not valid json").unwrap();

        let cfg = store.load().unwrap();
        assert_eq!(cfg.server.port, 4097);

        // corrupt file was backed up
        let mut entries = std::fs::read_dir(store.config_dir()).unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        entries.sort();
        assert!(entries.iter().any(|n| n.starts_with("config.json.corrupt-")),
            "expected a corrupt backup, got: {:?}", entries);
        // config.json now exists and is valid
        assert!(store.config_path().exists());
    }

    #[test]
    fn save_is_atomic_no_tmp_residue() {
        let (store, _dir) = temp_store();
        let cfg = ConfigStore::default_config();
        store.save(&cfg).unwrap();
        let entries: Vec<_> = std::fs::read_dir(store.config_dir()).unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(!entries.iter().any(|n| n == "config.json.tmp"),
            "tmp file should not remain after save, got: {:?}", entries);
        assert!(entries.iter().any(|n| n == "config.json"));
    }
}
```

Add `tempfile` dev-dependency to `src-tauri/Cargo.toml` under `[dev-dependencies]`:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test robustness_tests -- --nocapture` (in `src-tauri/`)
Expected: FAIL — `load_backs_up_corrupt_file_and_returns_default` panics on `store.load().unwrap()` (current `load` returns Err for corrupt JSON). `save_is_atomic_no_tmp_residue` may pass trivially (current save writes directly, no tmp) but the load test fails.

- [ ] **Step 3: Implement atomic save**

In `src-tauri/src/config/store.rs`, replace the `save` method:

```rust
    pub fn save(&self, config: &AppConfig) -> AppResult<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        let content = serde_json::to_string_pretty(config)?;
        std::fs::write(self.config_path(), content)?;
        Ok(())
    }
```

with:

```rust
    pub fn save(&self, config: &AppConfig) -> AppResult<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        let content = serde_json::to_string_pretty(config)?;
        let tmp = self.config_path().with_extension("json.tmp");
        std::fs::write(&tmp, &content)?;
        std::fs::rename(&tmp, self.config_path())?;
        Ok(())
    }
```

- [ ] **Step 4: Implement corrupt-tolerant load**

In `src-tauri/src/config/store.rs`, replace the `load` method:

```rust
    pub fn load(&self) -> AppResult<AppConfig> {
        let path = self.config_path();
        if !path.exists() {
            let cfg = Self::default_config();
            self.save(&cfg)?;
            return Ok(cfg);
        }
        let content = std::fs::read_to_string(&path)?;
        let cfg: AppConfig = serde_json::from_str(&content)?;
        Ok(cfg)
    }
```

with:

```rust
    pub fn load(&self) -> AppResult<AppConfig> {
        let path = self.config_path();
        if !path.exists() {
            let cfg = Self::default_config();
            self.save(&cfg)?;
            return Ok(cfg);
        }
        let content = std::fs::read_to_string(&path)?;
        match serde_json::from_str::<AppConfig>(&content) {
            Ok(cfg) => Ok(cfg),
            Err(_) => {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let backup = path.with_extension(format!("json.corrupt-{}", ts));
                let _ = std::fs::rename(&path, &backup);
                let cfg = Self::default_config();
                let _ = self.save(&cfg);
                Ok(cfg)
            }
        }
    }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test robustness_tests` (in `src-tauri/`)
Expected: PASS — both tests pass.

- [ ] **Step 6: Run full test suite**

Run: `cargo test` (in `src-tauri/`)
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/config/store.rs
git commit -m "fix(config): atomic save and corrupt-tolerant load with backup"
```

---

### Task 7: `read_stream` logs IO errors (#11)

**Files:**
- Modify: `src-tauri/src/process/supervisor.rs`

- [ ] **Step 1: Rewrite `read_stream` to surface IO errors**

In `src-tauri/src/process/supervisor.rs`, replace the `read_stream` function:

```rust
async fn read_stream<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    source: String,
    level: String,
    on_log: LogCallback,
) {
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        on_log(LogEntry { ts: now_ts(), source: source.clone(), level: level.clone(), line });
    }
}
```

with:

```rust
async fn read_stream<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    source: String,
    level: String,
    on_log: LogCallback,
) {
    let mut lines = BufReader::new(reader).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => on_log(LogEntry { ts: now_ts(), source: source.clone(), level: level.clone(), line }),
            Ok(None) => break,
            Err(e) => {
                on_log(LogEntry {
                    ts: now_ts(),
                    source: source.clone(),
                    level: "error".to_string(),
                    line: format!("stream read error: {}", e),
                });
                break;
            }
        }
    }
}
```

- [ ] **Step 2: Build**

Run: `cargo build` (in `src-tauri/`)
Expected: compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/process/supervisor.rs
git commit -m "fix(process): log stream read errors instead of silently dropping"
```

---

### Task 8: Make `BridgeInstaller` async; make `start_process`/`start_all`/`do_start_all` async (#13)

**Files:**
- Modify: `src-tauri/src/bridge/installer.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Produces: `BridgeInstaller::install/update/reinstall/check_update` become `async fn`. `do_start_all` becomes `async fn`. `start_process`/`start_all` commands become `async fn`. Tauri invoke contract unchanged.

- [ ] **Step 1: Rewrite `installer.rs` async using `resolve_command` + tokio**

Replace the entire contents of `src-tauri/src/bridge/installer.rs`:

```rust
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
```

- [ ] **Step 2: Make `start_process` async**

In `src-tauri/src/commands.rs`, replace `start_process`:

```rust
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
```

with:

```rust
#[tauri::command]
pub async fn start_process(target: String, state: State<'_, AppState>) -> AppResult<ProcessState> {
    let target = parse_target(&target)?;
    let cfg = state.load_config()?;
    match target {
        ProcessTarget::Server => state.process_manager.start_server(cfg.server.port, &cfg.server.cwd, &cfg.server.extra_env),
        ProcessTarget::Bridge => {
            let installer = BridgeInstaller::new(state.config_store.bridge_install_path(&cfg));
            if !installer.is_installed() {
                installer.install().await?;
            }
            renderer::write_bridge_files(&cfg, installer.path())?;
            let deps = bridge_check_deps();
            state.process_manager.start_bridge(installer.path(), deps.bun)
        }
    }
}
```

- [ ] **Step 3: Make `do_start_all` async**

In `src-tauri/src/commands.rs`, replace `do_start_all`:

```rust
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
```

with:

```rust
pub async fn do_start_all(state: &AppState) -> AppResult<()> {
    let cfg = state.load_config()?;
    state.process_manager.start_server(cfg.server.port, &cfg.server.cwd, &cfg.server.extra_env)?;
    let installer = BridgeInstaller::new(state.config_store.bridge_install_path(&cfg));
    if !installer.is_installed() {
        installer.install().await?;
    }
    renderer::write_bridge_files(&cfg, installer.path())?;
    let deps = bridge_check_deps();
    state.process_manager.start_bridge(installer.path(), deps.bun)?;
    Ok(())
}
```

- [ ] **Step 4: Make `start_all` command async**

In `src-tauri/src/commands.rs`, replace:

```rust
#[tauri::command]
pub fn start_all(state: State<'_, AppState>) -> AppResult<()> { do_start_all(state.inner()) }
```

with:

```rust
#[tauri::command]
pub async fn start_all(state: State<'_, AppState>) -> AppResult<()> { do_start_all(state.inner()).await }
```

- [ ] **Step 5: Update async command wrappers `check_bridge_update`/`update_bridge`/`reinstall_bridge`**

In `src-tauri/src/commands.rs`, replace these three commands:

```rust
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
```

with:

```rust
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
```

- [ ] **Step 6: Update tray `start_all` handler in `lib.rs`**

In `src-tauri/src/lib.rs`, replace the `"start_all"` tray menu handler:

```rust
                    "start_all" => {
                        let state = app.state::<state::AppState>();
                        let _ = commands::do_start_all(state.inner());
                    }
```

with:

```rust
                    "start_all" => {
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = handle.state::<state::AppState>();
                            let _ = commands::do_start_all(state.inner()).await;
                        });
                    }
```

- [ ] **Step 7: Verify build**

Run: `cargo build` (in `src-tauri/`)
Expected: compiles with no errors. (If `std::process::Command` import in installer.rs is now unused, remove it—`resolve_command` returns `std::process::Command` so the `use` is not needed; the new file already has no `use std::process::Command`.)

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/bridge/installer.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "fix(bridge): async install/update to avoid blocking command thread"
```

---

### Task 9: Tray `quit` async (#14)

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Replace `quit` handler**

In `src-tauri/src/lib.rs`, replace:

```rust
                    "quit" => {
                        let state = app.state::<state::AppState>();
                        let _ = state.process_manager.stop(process::ProcessTarget::Bridge);
                        let _ = state.process_manager.stop(process::ProcessTarget::Server);
                        app.exit(0);
                    }
```

with:

```rust
                    "quit" => {
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = handle.state::<state::AppState>();
                            let _ = state.process_manager.stop_async(process::ProcessTarget::Bridge).await;
                            let _ = state.process_manager.stop_async(process::ProcessTarget::Server).await;
                            handle.exit(0);
                        });
                    }
```

- [ ] **Step 2: Verify build**

Run: `cargo build` (in `src-tauri/`)
Expected: compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "fix(tray): async quit to avoid blocking main thread up to 10s"
```

---

### Task 10: Async health check via tokio + config version (#16, #18)

**Files:**
- Modify: `src-tauri/Cargo.toml` (drop `blocking`)
- Modify: `src-tauri/src/monitor/health.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Produces: `HealthChecker::check_once(&self) -> impl Future<Output = bool>` (async). `AppState.config_version: Arc<std::sync::atomic::AtomicU64>`. `save_config` command bumps the version after a successful save.

- [ ] **Step 1: Drop `blocking` feature from reqwest**

In `src-tauri/Cargo.toml`, replace:

```toml
reqwest = { version = "0.12", features = ["blocking"] }
```

with:

```toml
reqwest = { version = "0.12", default-features = true, features = ["rustls-tls"] }
```

> Rationale: switch to async client; keep TLS. Using `rustls-tls` to avoid system OpenSSL dependency on macOS/Windows. If the build fails due to feature conflicts, fall back to `reqwest = { version = "0.12", features = ["json"] }` (the default feature set already includes TLS via native-tls; on macOS this links to the system SecureTransport). Prefer the fallback if Step 6 build fails.

- [ ] **Step 2: Rewrite `health.rs` async**

Replace the entire contents of `src-tauri/src/monitor/health.rs`:

```rust
use std::time::Duration;

pub struct HealthChecker {
    url: String,
    client: reqwest::Client,
}

impl HealthChecker {
    pub fn new(server_url: &str) -> Self {
        let url = format!("{}/session/status", server_url.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { url, client }
    }

    pub async fn check_once(&self) -> bool {
        match self.client.get(&self.url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}
```

- [ ] **Step 3: Add `config_version` to `AppState`**

In `src-tauri/src/state.rs`, replace the entire file:

```rust
use std::sync::{Arc, Mutex, atomic::AtomicU64};
use crate::config::{ConfigStore, AppConfig};
use crate::process::ProcessManager;
use crate::monitor::LogBuffer;

pub struct AppState {
    pub config_store: ConfigStore,
    pub process_manager: ProcessManager,
    pub log_buffer: Arc<Mutex<LogBuffer>>,
    pub config_version: Arc<AtomicU64>,
}

impl AppState {
    pub fn new_with_buffer(process_manager: ProcessManager, log_buffer: Arc<Mutex<LogBuffer>>) -> Self {
        Self {
            config_store: ConfigStore::new(),
            process_manager,
            log_buffer,
            config_version: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn load_config(&self) -> crate::error::AppResult<AppConfig> {
        self.config_store.load()
    }

    pub fn save_config(&self, config: &AppConfig) -> crate::error::AppResult<()> {
        self.config_store.save(config)?;
        self.config_version.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    pub fn config_version(&self) -> Arc<AtomicU64> {
        self.config_version.clone()
    }
}
```

- [ ] **Step 4: `save_config` command now uses `AppState::save_config`**

In `src-tauri/src/commands.rs`, the `save_config` command currently calls `state.save_config(&config)` which previously just delegated to `config_store.save`. It now also bumps the version automatically. Replace:

```rust
#[tauri::command]
pub fn save_config(config: AppConfig, state: State<'_, AppState>) -> AppResult<()> {
    state.save_config(&config)
}
```

with:

```rust
#[tauri::command]
pub fn save_config(config: AppConfig, state: State<'_, AppState>) -> AppResult<()> {
    state.save_config(&config)
}
```

> No code change needed — `state.save_config` already delegates. The version bump happens inside `AppState::save_config` now. This step is a no-op confirmation; skip if already correct.

- [ ] **Step 5: Replace health-check thread with tokio task**

In `src-tauri/src/lib.rs`, replace the `std::thread::spawn` health-check block:

```rust
            let handle2 = handle.clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(5));
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

with:

```rust
            let handle2 = handle.clone();
            let config_version = app_state.config_version();
            tauri::async_runtime::spawn(async move {
                let mut last_version: u64 = 0;
                let mut current_url: Option<String> = None;
                let mut checker: Option<monitor::health::HealthChecker> = None;
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    let state = handle2.state::<state::AppState>();
                    let server_state = state.process_manager.get_state(process::ProcessTarget::Server);
                    if server_state.state != process::ProcessStateKind::Running {
                        continue;
                    }
                    let v = config_version.load(std::sync::atomic::Ordering::Relaxed);
                    if v != last_version || checker.is_none() {
                        last_version = v;
                        let cfg = state.load_config().unwrap_or_else(|_| config::ConfigStore::default_config());
                        current_url = Some(cfg.server.opencode_server_url.clone());
                        checker = Some(monitor::health::HealthChecker::new(&cfg.server.opencode_server_url));
                    }
                    let healthy = match &checker {
                        Some(c) => c.check_once().await,
                        None => false,
                    };
                    state.process_manager.set_health(process::ProcessTarget::Server, healthy);
                    let _ = handle2.emit("health://update", serde_json::json!({ "target": "server", "healthy": healthy }));
                }
            });
```

Note: `current_url` is assigned but not read further (kept for clarity of intent). To avoid an unused-variable warning, either prefix with `_` or remove it. Remove it for cleanliness—replace `current_url = Some(...)` line with nothing. Final block:

```rust
            let handle2 = handle.clone();
            let config_version = app_state.config_version();
            tauri::async_runtime::spawn(async move {
                let mut last_version: u64 = 0;
                let mut checker: Option<monitor::health::HealthChecker> = None;
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    let state = handle2.state::<state::AppState>();
                    let server_state = state.process_manager.get_state(process::ProcessTarget::Server);
                    if server_state.state != process::ProcessStateKind::Running {
                        continue;
                    }
                    let v = config_version.load(std::sync::atomic::Ordering::Relaxed);
                    if v != last_version || checker.is_none() {
                        last_version = v;
                        let cfg = state.load_config().unwrap_or_else(|_| config::ConfigStore::default_config());
                        checker = Some(monitor::health::HealthChecker::new(&cfg.server.opencode_server_url));
                    }
                    let healthy = match &checker {
                        Some(c) => c.check_once().await,
                        None => false,
                    };
                    state.process_manager.set_health(process::ProcessTarget::Server, healthy);
                    let _ = handle2.emit("health://update", serde_json::json!({ "target": "server", "healthy": healthy }));
                }
            });
```

> Important: `app_state` is moved into `app.manage(app_state)` on the line right after this block currently. We need `config_version` cloned BEFORE that move. Reorder: call `app_state.config_version()` before `app.manage(app_state)`. The current code is:
> ```rust
>             let pm = process::ProcessManager::new(on_state, on_log, on_qr);
>             let app_state = state::AppState::new_with_buffer(pm, log_buffer);
>             app.manage(app_state);
> ```
> Change to:
> ```rust
>             let pm = process::ProcessManager::new(on_state, on_log, on_qr);
>             let app_state = state::AppState::new_with_buffer(pm, log_buffer);
>             let config_version = app_state.config_version();
>             app.manage(app_state);
> ```

- [ ] **Step 6: Verify build**

Run: `cargo build` (in `src-tauri/`)
Expected: compiles with no errors. If reqwest feature combo fails, adjust per Step 1 fallback note.

- [ ] **Step 7: Run tests**

Run: `cargo test` (in `src-tauri/`)
Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/monitor/health.rs src-tauri/src/state.rs src-tauri/src/lib.rs
git commit -m "fix(monitor): async health check via tokio with config version caching"
```

---

### Task 11: Frontend — cancel-safe `useTauriEvent` (#3)

**Files:**
- Modify: `src/hooks/useTauriEvent.ts`

- [ ] **Step 1: Rewrite `useTauriEvent`**

Replace the entire contents of `src/hooks/useTauriEvent.ts`:

```ts
import { useEffect, useRef } from "react"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

export function useTauriEvent<T>(event: string, handler: (payload: T) => void) {
  const handlerRef = useRef(handler)
  handlerRef.current = handler

  useEffect(() => {
    let unlisten: UnlistenFn | undefined
    let cancelled = false
    listen<T>(event, (e) => handlerRef.current(e.payload)).then((fn) => {
      if (cancelled) {
        fn()
      } else {
        unlisten = fn
      }
    })
    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [event])
}
```

- [ ] **Step 2: Verify frontend build**

Run: `npm run build` (at repo root)
Expected: tsc + vite build succeeds with no errors.

- [ ] **Step 3: Commit**

```bash
git add src/hooks/useTauriEvent.ts
git commit -m "fix(hooks): cancel-safe useTauriEvent to prevent listener leaks"
```

---

### Task 12: Frontend — `formatError` + use across pages; refresh logs errors (#9, #10)

**Files:**
- Modify: `src/lib/utils.ts`
- Modify: `src/hooks/useProcessState.tsx`
- Modify: `src/pages/Dashboard.tsx`
- Modify: `src/pages/Config.tsx`
- Modify: `src/pages/Bridge.tsx`
- Modify: `src/pages/Channels.tsx`
- Modify: `src/components/ProcessCard.tsx`
- Modify: `src/components/LogView.tsx`

- [ ] **Step 1: Add `formatError` to utils**

In `src/lib/utils.ts`, append:

```ts
export function formatError(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) {
    return String((e as { message: unknown }).message)
  }
  return String(e)
}
```

- [ ] **Step 2: `useProcessState` refresh logs errors**

In `src/hooks/useProcessState.tsx`, replace:

```ts
  const refresh = useCallback(() => { getState().then(setState).catch(() => {}) }, [])
```

with:

```ts
  const refresh = useCallback(() => {
    getState().then(setState).catch((e) => console.error("[refresh process state]", e))
  }, [])
```

- [ ] **Step 3: Dashboard — use formatError**

In `src/pages/Dashboard.tsx`, add import and replace the three catch handlers. Replace:

```tsx
import { toast } from "sonner"
```

with:

```tsx
import { toast } from "sonner"
import { formatError } from "@/lib/utils"
```

Replace:

```tsx
        <Button onClick={() => startAll().catch((e) => toast.error(`启动失败: ${e}`))}>启动全部</Button>
        <Button variant="outline" onClick={() => stopAll().catch((e) => toast.error(`停止失败: ${e}`))}>停止全部</Button>
        <Button variant="outline" onClick={() => restartAll().catch((e) => toast.error(`重启失败: ${e}`))}>重启全部</Button>
```

with:

```tsx
        <Button onClick={() => startAll().catch((e) => toast.error(`启动失败: ${formatError(e)}`))}>启动全部</Button>
        <Button variant="outline" onClick={() => stopAll().catch((e) => toast.error(`停止失败: ${formatError(e)}`))}>停止全部</Button>
        <Button variant="outline" onClick={() => restartAll().catch((e) => toast.error(`重启失败: ${formatError(e)}`))}>重启全部</Button>
```

- [ ] **Step 4: ProcessCard — use formatError**

In `src/components/ProcessCard.tsx`, replace:

```tsx
import { toast } from "sonner"
```

with:

```tsx
import { toast } from "sonner"
import { formatError } from "@/lib/utils"
```

Replace:

```tsx
  const handleStart = () => startProcess(target).catch((e) => toast.error(`启动失败: ${e}`))
  const handleStop = () => stopProcess(target).catch((e) => toast.error(`停止失败: ${e}`))
  const handleRestart = () => restartProcess(target).catch((e) => toast.error(`重启失败: ${e}`))
```

with:

```tsx
  const handleStart = () => startProcess(target).catch((e) => toast.error(`启动失败: ${formatError(e)}`))
  const handleStop = () => stopProcess(target).catch((e) => toast.error(`停止失败: ${formatError(e)}`))
  const handleRestart = () => restartProcess(target).catch((e) => toast.error(`重启失败: ${formatError(e)}`))
```

- [ ] **Step 5: Config — use formatError**

In `src/pages/Config.tsx`, replace:

```tsx
import { toast } from "sonner"
```

with:

```tsx
import { toast } from "sonner"
import { formatError } from "@/lib/utils"
```

Replace:

```tsx
  useEffect(() => { getConfig().then(setConfig).catch(() => toast.error("加载配置失败")) }, [])

  if (!config) return <div>加载中...</div>

  const save = () => saveConfig(config).then(() => toast.success("已保存")).catch(() => toast.error("保存失败"))
```

with:

```tsx
  useEffect(() => { getConfig().then(setConfig).catch((e) => toast.error(`加载配置失败: ${formatError(e)}`)) }, [])

  if (!config) return <div>加载中...</div>

  const save = () => saveConfig(config).then(() => toast.success("已保存")).catch((e) => toast.error(`保存失败: ${formatError(e)}`))
```

- [ ] **Step 6: Bridge — use formatError**

In `src/pages/Bridge.tsx`, replace:

```tsx
import { toast } from "sonner"
```

with:

```tsx
import { toast } from "sonner"
import { formatError } from "@/lib/utils"
```

Replace:

```tsx
  const save = () => saveConfig(config).then(() => toast.success("已保存")).catch(() => toast.error("保存失败"))
```

with:

```tsx
  const save = () => saveConfig(config).then(() => toast.success("已保存")).catch((e) => toast.error(`保存失败: ${formatError(e)}`))
```

Replace:

```tsx
        <Button variant="outline" onClick={() => checkBridgeUpdate().then((u) => toast.info(u ? "已是最新" : "有更新可用")).catch(() => toast.error("检查失败"))}>检查更新</Button>
        <Button variant="outline" onClick={() => updateBridge().then(() => toast.success("已更新")).catch(() => toast.error("更新失败"))}>更新</Button>
        <Button variant="outline" onClick={() => reinstallBridge().then(() => toast.success("已重装")).catch(() => toast.error("重装失败"))}>重新安装</Button>
```

with:

```tsx
        <Button variant="outline" onClick={() => checkBridgeUpdate().then((u) => toast.info(u ? "已是最新" : "有更新可用")).catch((e) => toast.error(`检查失败: ${formatError(e)}`))}>检查更新</Button>
        <Button variant="outline" onClick={() => updateBridge().then(() => toast.success("已更新")).catch((e) => toast.error(`更新失败: ${formatError(e)}`))}>更新</Button>
        <Button variant="outline" onClick={() => reinstallBridge().then(() => toast.success("已重装")).catch((e) => toast.error(`重装失败: ${formatError(e)}`))}>重新安装</Button>
```

- [ ] **Step 7: Channels — use formatError**

In `src/pages/Channels.tsx`, replace:

```tsx
import { toast } from "sonner"
```

with:

```tsx
import { toast } from "sonner"
import { formatError } from "@/lib/utils"
```

Replace:

```tsx
  const save = () => saveConfig(config).then(() => toast.success("已保存")).catch(() => toast.error("保存失败"))
```

with:

```tsx
  const save = () => saveConfig(config).then(() => toast.success("已保存")).catch((e) => toast.error(`保存失败: ${formatError(e)}`))
```

- [ ] **Step 8: LogView — use formatError**

In `src/components/LogView.tsx`, replace:

```tsx
import { toast } from "sonner"
```

with:

```tsx
import { toast } from "sonner"
import { formatError } from "@/lib/utils"
```

Replace:

```tsx
    }).catch(() => toast.error("清空失败"))
```

with:

```tsx
    }).catch((e) => toast.error(`清空失败: ${formatError(e)}`))
```

Replace:

```tsx
    exportLogs(activeTab).then((path) => toast.success(`已导出到: ${path}`)).catch(() => toast.error("导出失败"))
```

with:

```tsx
    exportLogs(activeTab).then((path) => toast.success(`已导出到: ${path}`)).catch((e) => toast.error(`导出失败: ${formatError(e)}`))
```

- [ ] **Step 9: Verify frontend build**

Run: `npm run build` (at repo root)
Expected: tsc + vite build succeeds with no errors.

- [ ] **Step 10: Commit**

```bash
git add src/lib/utils.ts src/hooks/useProcessState.tsx src/pages/Dashboard.tsx src/pages/Config.tsx src/pages/Bridge.tsx src/pages/Channels.tsx src/components/ProcessCard.tsx src/components/LogView.tsx
git commit -m "fix(ui): show readable error messages and log refresh failures"
```

---

### Task 13: Frontend — Error Boundary (#15)

**Files:**
- Create: `src/components/ErrorBoundary.tsx`
- Modify: `src/main.tsx`

- [ ] **Step 1: Create `ErrorBoundary`**

Create `src/components/ErrorBoundary.tsx`:

```tsx
import { Component, type ErrorInfo, type ReactNode } from "react"

interface Props {
  children: ReactNode
}

interface State {
  hasError: boolean
  message: string
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, message: "" }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, message: error.message || String(error) }
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[ErrorBoundary]", error, info)
  }

  reset = () => this.setState({ hasError: false, message: "" })

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex h-screen flex-col items-center justify-center gap-4 p-6 text-center">
          <h1 className="text-lg font-semibold">出错了</h1>
          <p className="text-sm text-muted-foreground">应用遇到错误，请尝试重置或重启应用。</p>
          <pre className="max-w-md overflow-auto rounded bg-muted p-2 text-xs">{this.state.message}</pre>
          <button
            className="rounded bg-primary px-4 py-2 text-sm text-primary-foreground"
            onClick={this.reset}
          >
            重置
          </button>
        </div>
      )
    }
    return this.props.children
  }
}
```

- [ ] **Step 2: Wrap `<App/>` in `main.tsx`**

Replace the entire contents of `src/main.tsx`:

```tsx
import React from "react"
import ReactDOM from "react-dom/client"
import App from "./App"
import { ErrorBoundary } from "@/components/ErrorBoundary"
import "./styles/globals.css"

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
)
```

- [ ] **Step 3: Verify frontend build**

Run: `npm run build` (at repo root)
Expected: tsc + vite build succeeds with no errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/ErrorBoundary.tsx src/main.tsx
git commit -m "fix(ui): add ErrorBoundary to prevent white screen on render errors"
```

---

### Task 14: Final verification

**Files:** none modified.

- [ ] **Step 1: Rust full build + test + clippy**

Run from `src-tauri/`:
```
cargo build && cargo test && cargo clippy --all-targets -- -D warnings
```
Expected: all pass, no warnings.

- [ ] **Step 2: Frontend build**

Run from repo root:
```
npm run build
```
Expected: tsc + vite build succeeds with no errors.

- [ ] **Step 3: Verify no `lock().unwrap()` remains in src-tauri/src**

Run: `rg "lock\(\)\.unwrap\(\)" src-tauri/src` (from repo root)
Expected: no matches.

- [ ] **Step 4: Verify no `.catch(() =>` swallowing remains in src**

Run: `rg "\.catch\(\(\) =>" src` (from repo root)
Expected: no matches (all converted to `.catch((e) => ...)`).

If any remain, fix them with `formatError` per Task 12 pattern.

- [ ] **Step 5: Commit (if any cleanup)**

Only if Step 3 or 4 found stragglers:
```bash
git add -A
git commit -m "fix: clean up remaining lock unwraps / silent catches"
```

---

## 自审

**1. 规格覆盖：** 对照 spec 的 18 项：
- #1 supervisor stopping 标志 → Task 3 ✓
- #2 restart 重写配置 → Task 4 ✓
- #3 useTauriEvent 不泄漏 → Task 11 ✓
- #4 expect panic → Task 3 Step 5 ✓
- #5 Windows 命令查找 → Task 1 (resolve_command) + Task 5 (应用) + Task 8 (installer) ✓
- #6 lock().unwrap() → Task 1 (helper) + Task 2 (全局替换) ✓
- #7 原子写入 → Task 6 ✓
- #8 容错 load → Task 6 ✓
- #9 toast 可读 → Task 12 ✓
- #10 refresh 不吞错 → Task 12 Step 2 ✓
- #11 read_stream 记录 → Task 7 ✓
- #12 托盘 start_all 异步 → Task 8 Step 6 ✓
- #13 bridge install 异步 → Task 8 ✓
- #14 quit 异步 → Task 9 ✓
- #15 Error Boundary → Task 13 ✓
- #16 健康检查 tokio → Task 10 ✓
- #17 restart_async 复用 ConfigStore → Task 4 ✓
- #18 配置版本号 → Task 10 ✓
全部覆盖。

**2. 占位符扫描：** 无 TBD/TODO；每步都有完整代码。

**3. 类型一致性：**
- `resolve_command(name: &str) -> AppResult<Command>` — Task 1 定义，Task 5/8 使用，一致。
- `lock_or_recover<T>(m: &Mutex<T>) -> MutexGuard<'_, T>` — Task 1 定义，Task 2 使用 `&self.server`/`&mp_ref` 形式，一致。
- `restart_async(target, cfg, bridge_dir: &Path, use_bun: bool)` — Task 4 定义，`commands.rs` Task 4 Step 2 调用传 `&bridge_dir`，一致。
- `HealthChecker::check_once(&self) -> async bool` — Task 10 定义并使用，一致。
- `AppState.config_version: Arc<AtomicU64>` + `config_version()` 方法 — Task 10 定义并在 lib.rs 使用，一致。
- `ManagedProcess.stopping: bool` — Task 3 定义并在 stop_async/supervise 使用，一致。
- `formatError(e: unknown): string` — Task 12 定义，多处使用，一致。

无遗漏，无类型不一致。
