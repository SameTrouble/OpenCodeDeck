# OpenCodeDeck Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Tauri v2 cross-platform desktop app that manages opencode server and opencode-im-bridge as child processes, with config editing, system tray, process status/logs, and WeChat QR display.

**Architecture:** Rust backend owns all child-process lifecycle and config rendering (double-layer process management); React frontend issues Tauri commands and subscribes to Tauri events. Config is stored in the app's own config dir and rendered to `.env` + `opencode-im.jsonc` in the bridge workdir before launch.

**Tech Stack:** Tauri v2, Rust (tokio, serde, dirs, thiserror, reqwest), React 18 + TypeScript + Vite + Tailwind CSS + shadcn/ui + lucide-react + qrcode.

## Global Constraints

- Tauri v2 (tauri-cli 2.11+ confirmed installed).
- Rust edition 2021, MSRV 1.94.
- Node 26+, npm.
- Cross-platform: macOS, Windows, Linux.
- Bridge source pulled from `https://github.com/ET06731/opencode-im-bridge` via git.
- Runtime deps detected from system PATH: `opencode`, `bun` (preferred) or `node`+`npx tsx`, `git`.
- Credentials stored as plaintext in `config.json` (user accepted this risk).
- No code comments unless explicitly requested.
- ESM-style imports not relevant to Rust; Rust uses 2021 module system.
- Frontend uses 2-space indent, no trailing commas where unnecessary (match bridge repo style).
- Process stop order: bridge first (SIGTERM → 5s → SIGKILL), then server.
- Log ring buffer: 5000 lines per source.
- Health check: poll `GET {opencodeServerUrl}/session/status` every 5s.

---

## File Structure

### Rust backend (`src-tauri/src/`)

| File | Responsibility |
|------|----------------|
| `main.rs` | Tauri app entry: build app, register commands, setup tray, manage window hide/show |
| `lib.rs` | Re-exports modules for integration tests |
| `error.rs` | `AppError` enum with serde::Serialize |
| `state.rs` | `AppState` holding `Mutex<ProcessManager>`, `ConfigStore`, log buffers, tray handle |
| `commands.rs` | All `#[tauri::command]` functions |
| `config/mod.rs` | Re-exports |
| `config/store.rs` | `ConfigStore`: read/write config.json, default config, path resolution |
| `config/renderer.rs` | `ConfigRenderer`: AppConfig → `.env` + `opencode-im.jsonc` strings |
| `process/mod.rs` | Re-exports |
| `process/manager.rs` | `ProcessManager`: per-target `ManagedProcess`, spawn/stop/restart, state machine |
| `process/supervisor.rs` | tokio task: read stdout/stderr line-by-line, push events, detect exit |
| `bridge/mod.rs` | Re-exports |
| `bridge/installer.rs` | `BridgeInstaller`: clone/pull/reinstall, path resolution |
| `bridge/env_check.rs` | `check_deps()`: detect opencode/bun/node/npm/git on PATH |
| `monitor/mod.rs` | Re-exports |
| `monitor/log_buffer.rs` | `LogBuffer`: VecDeque<LogEntry> with capacity 5000 |
| `monitor/stdout_parser.rs` | `StdoutParser`: detect WeChat ASCII QR / URL |
| `monitor/health.rs` | `HealthChecker`: periodic reqwest poll, emit events |

### Frontend (`src/`)

| File | Responsibility |
|------|----------------|
| `main.tsx` | React root, mount App |
| `App.tsx` | Layout: sidebar nav + routed pages + WechatQrDialog |
| `lib/types.ts` | TS types mirroring Rust structs |
| `lib/tauri.ts` | Typed wrappers around `invoke` and `listen` |
| `hooks/useTauriEvent.ts` | Generic event subscription hook |
| `hooks/useProcessState.ts` | Subscribes to state://update, exposes current state |
| `components/ProcessCard.tsx` | Process state display card |
| `components/LogView.tsx` | Log viewer with tabs, pause/clear/export |
| `components/WechatQrDialog.tsx` | QR display dialog |
| `components/ui/*` | shadcn components (button, card, input, switch, tabs, dialog, toast) |
| `pages/Dashboard.tsx` | Overview: two ProcessCards + start/stop all + recent logs |
| `pages/Processes.tsx` | Per-process control |
| `pages/Config.tsx` | Edit server section of config |
| `pages/Bridge.tsx` | Edit bridge section + install/update/reinstall buttons + dep status |
| `pages/Channels.tsx` | Edit channels section |
| `pages/Logs.tsx` | Full log viewer |

### Config

| File | Responsibility |
|------|----------------|
| `src-tauri/tauri.conf.json` | Tauri config: window, tray, capabilities |
| `src-tauri/Cargo.toml` | Rust deps |
| `package.json` | Frontend deps + scripts |
| `vite.config.ts` | Vite + Tauri plugin |
| `tailwind.config.ts` | Tailwind config |
| `tsconfig.json` | TS config |
| `src/styles/globals.css` | Tailwind directives + shadcn vars |

---

## Task 1: Scaffold Tauri + React project

**Files:**
- Create: `package.json`, `vite.config.ts`, `tsconfig.json`, `tailwind.config.ts`, `postcss.config.js`, `src/styles/globals.css`, `src/main.tsx`, `index.html`
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/build.rs`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/icons/` (placeholder icons)
- Create: `src-tauri/capabilities/default.json`

**Interfaces:**
- Produces: a runnable `npm run tauri dev` that shows an empty window with "OpenCodeDeck" title.

- [ ] **Step 1: Initialize npm project and install frontend deps**

Run from repo root:

```bash
npm init -y
npm install react react-dom
npm install -D typescript @types/react @types/react-dom vite @vitejs/plugin-react tailwindcss postcss autoprefixer @tauri-apps/cli @tauri-apps/api
```

- [ ] **Step 2: Write package.json scripts**

Overwrite `package.json` scripts section (preserve installed deps):

```json
{
  "name": "opencodedeck",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  }
}
```

- [ ] **Step 3: Write frontend config files**

`tsconfig.json`:
```json
{
  "compilerOptions": {
    "target": "ES2021",
    "useDefineForClassFields": true,
    "lib": ["ES2021", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"]
}
```

`vite.config.ts`:
```ts
import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_"],
  build: { target: "es2021", minify: "esbuild", sourcemap: false },
})
```

`tailwind.config.ts`:
```ts
import type { Config } from "tailwindcss"

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: { extend: {} },
  plugins: [],
} satisfies Config
```

`postcss.config.js`:
```js
export default {
  plugins: { tailwindcss: {}, autoprefixer: {} },
}
```

`index.html`:
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>OpenCodeDeck</title>
  </head>
  <body class="bg-background text-foreground">
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

`src/styles/globals.css`:
```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```

`src/main.tsx`:
```tsx
import React from "react"
import ReactDOM from "react-dom/client"
import App from "./App"
import "./styles/globals.css"

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
```

`src/App.tsx` (placeholder):
```tsx
export default function App() {
  return <div className="p-8 text-lg">OpenCodeDeck</div>
}
```

- [ ] **Step 4: Scaffold Tauri Rust backend**

`src-tauri/Cargo.toml`:
```toml
[package]
name = "opencodedeck"
version = "0.1.0"
edition = "2021"

[lib]
name = "opencodedeck_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
dirs = "5"
thiserror = "1"
reqwest = { version = "0.12", features = ["blocking"] }
chrono = "0.4"
```

`src-tauri/build.rs`:
```rust
fn main() {
    tauri_build::build()
}
```

`src-tauri/src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    opencodedeck_lib::run()
}
```

`src-tauri/src/lib.rs`:
```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: Write tauri.conf.json**

`src-tauri/tauri.conf.json`:
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "OpenCodeDeck",
  "version": "0.1.0",
  "identifier": "com.opencodedeck.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "OpenCodeDeck",
        "width": 1000,
        "height": 700,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"]
  }
}
```

`src-tauri/capabilities/default.json`:
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": ["core:default", "shell:allow-open"]
}
```

- [ ] **Step 6: Add placeholder icons**

Run (creates 1x1 transparent PNGs so build doesn't fail; replace later):

```bash
npx @tauri-apps/cli icon --help > /dev/null 2>&1 || true
```

If `tauri icon` needs a source, create a minimal 1024x1024 PNG first. For MVP, download Tauri default icon set or generate:

```bash
mkdir -p src-tauri/icons
printf '\x89PNG\r\n\x1a\n' > src-tauri/icons/icon.png
```

Then run:
```bash
npx @tauri-apps/cli icon src-tauri/icons/icon.png --output src-tauri/icons
```

If the 1x1 PNG is too small for `tauri icon`, instead copy from a Tauri template. Fallback: use `cargo tauri icon` with any valid PNG >= 512x512. If no source available, create a 512x512 solid-color PNG via a tiny script and retry.

- [ ] **Step 7: Verify dev server starts**

Run:
```bash
npm run tauri dev
```

Expected: a window opens titled "OpenCodeDeck" showing "OpenCodeDeck" text. No Rust compile errors. Stop the dev server with Ctrl+C.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "chore: scaffold Tauri v2 + React + Tailwind project"
```

---

## Task 2: AppError and shared types

**Files:**
- Create: `src-tauri/src/error.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod error;`)

**Interfaces:**
- Produces: `AppError` enum (Io/Config/Process/BridgeInstall/EnvNotFound) with `serde::Serialize`; convertible from `io::Error`.

- [ ] **Step 1: Write error.rs**

`src-tauri/src/error.rs`:
```rust
use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("io error: {0}")]
    Io(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("process error: {0}")]
    Process(String),
    #[error("bridge install error: {0}")]
    BridgeInstall(String),
    #[error("dependency not found: {0}")]
    EnvNotFound(String),
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Config(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
```

- [ ] **Step 2: Register module in lib.rs**

Modify `src-tauri/src/lib.rs`, add at top:
```rust
pub mod error;
```

- [ ] **Step 3: Verify it compiles**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/error.rs src-tauri/src/lib.rs
git commit -m "feat(error): add AppError enum with serde serialization"
```

---

## Task 3: AppConfig types and ConfigStore

**Files:**
- Create: `src-tauri/src/config/mod.rs`, `src-tauri/src/config/store.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Produces: `AppConfig`, `ServerConfig`, `BridgeConfig`, `ChannelsConfig`, and per-channel config structs (all `Serialize`/`Deserialize`). `ConfigStore::load() -> AppResult<AppConfig>`, `ConfigStore::save(&AppConfig) -> AppResult<()>`, `ConfigStore::config_path() -> PathBuf`, `ConfigStore::default_config() -> AppConfig`.

- [ ] **Step 1: Write config types and ConfigStore**

`src-tauri/src/config/mod.rs`:
```rust
pub mod store;
pub mod renderer;

pub use store::{
    AppConfig, ServerConfig, BridgeConfig, ChannelsConfig, FeishuConfig, QqConfig,
    TelegramConfig, DiscordConfig, WechatConfig, ConfigStore,
};
```

`src-tauri/src/config/store.rs`:
```rust
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub server: ServerConfig,
    pub bridge: BridgeConfig,
    pub channels: ChannelsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub port: u16,
    pub opencode_server_url: String,
    pub cwd: String,
    #[serde(default)]
    pub extra_env: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeConfig {
    #[serde(default)]
    pub install_path: Option<String>,
    pub default_agent: String,
    pub data_dir: String,
    #[serde(default)]
    pub progress: ProgressConfig,
    #[serde(default)]
    pub launcher: LauncherConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressConfig {
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    #[serde(default = "default_max_debounce_ms")]
    pub max_debounce_ms: u64,
}

fn default_debounce_ms() -> u64 { 500 }
fn default_max_debounce_ms() -> u64 { 3000 }

impl Default for ProgressConfig {
    fn default() -> Self {
        Self { debounce_ms: 500, max_debounce_ms: 3000 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub auto_start_server: bool,
    #[serde(default = "default_server_command")]
    pub server_command: String,
    #[serde(default = "default_server_start_timeout_ms")]
    pub server_start_timeout_ms: u64,
    #[serde(default = "default_probe_timeout_ms")]
    pub probe_timeout_ms: u64,
}

fn default_true() -> bool { true }
fn default_server_command() -> String { "opencode serve".to_string() }
fn default_server_start_timeout_ms() -> u64 { 30000 }
fn default_probe_timeout_ms() -> u64 { 4000 }

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_start_server: true,
            server_command: "opencode serve".to_string(),
            server_start_timeout_ms: 30000,
            probe_timeout_ms: 4000,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelsConfig {
    #[serde(default)]
    pub feishu: FeishuConfig,
    #[serde(default)]
    pub qq: QqConfig,
    #[serde(default)]
    pub telegram: TelegramConfig,
    #[serde(default)]
    pub discord: DiscordConfig,
    #[serde(default)]
    pub wechat: WechatConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeishuConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub app_secret: String,
    #[serde(default)]
    pub verification_token: String,
    #[serde(default = "default_webhook_port")]
    pub webhook_port: u16,
    #[serde(default)]
    pub encrypt_key: String,
}

fn default_webhook_port() -> u16 { 3001 }

impl Default for FeishuConfig {
    fn default() -> Self {
        Self {
            enabled: false, app_id: String::new(), app_secret: String::new(),
            verification_token: String::new(), webhook_port: 3001, encrypt_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QqConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub secret: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub allowed_chat_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub allowed_channel_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatConfig {
    #[serde(default)]
    pub enabled: bool,
}

pub struct ConfigStore {
    config_dir: PathBuf,
}

impl ConfigStore {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(if cfg!(target_os = "macos") { "OpenCodeDeck" }
                  else if cfg!(target_os = "windows") { "OpenCodeDeck" }
                  else { "opencodedeck" });
        Self { config_dir }
    }

    pub fn config_dir(&self) -> &Path { &self.config_dir }

    pub fn config_path(&self) -> PathBuf { self.config_dir.join("config.json") }

    pub fn default_config() -> AppConfig {
        AppConfig {
            version: 1,
            server: ServerConfig {
                port: 4097,
                opencode_server_url: "http://127.0.0.1:4097".to_string(),
                cwd: dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
                extra_env: Default::default(),
            },
            bridge: BridgeConfig {
                install_path: None,
                default_agent: "build".to_string(),
                data_dir: "./data".to_string(),
                progress: ProgressConfig::default(),
                launcher: LauncherConfig::default(),
            },
            channels: ChannelsConfig::default(),
        }
    }

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

    pub fn save(&self, config: &AppConfig) -> AppResult<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        let content = serde_json::to_string_pretty(config)?;
        std::fs::write(self.config_path(), content)?;
        Ok(())
    }

    pub fn bridge_install_path(&self, config: &AppConfig) -> PathBuf {
        if let Some(p) = &config.bridge.install_path {
            PathBuf::from(p)
        } else {
            self.config_dir.join("bridges").join("opencode-im-bridge")
        }
    }
}
```

- [ ] **Step 2: Register module**

Modify `src-tauri/src/lib.rs`, add:
```rust
pub mod config;
```

- [ ] **Step 3: Verify it compiles**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/config/ src-tauri/src/lib.rs
git commit -m "feat(config): add AppConfig types and ConfigStore"
```

---

## Task 4: ConfigRenderer (AppConfig → .env + jsonc)

**Files:**
- Create: `src-tauri/src/config/renderer.rs`
- Create: `src-tauri/src/config/renderer_tests.rs` (or inline `#[cfg(test)]` mod)

**Interfaces:**
- Consumes: `AppConfig` from Task 3.
- Produces: `ConfigRenderer::render_env(&AppConfig) -> String`, `ConfigRenderer::render_jsonc(&AppConfig) -> String`, `ConfigRenderer::write_bridge_files(&AppConfig, &Path) -> AppResult<()>`.

- [ ] **Step 1: Write the failing test**

Append to `src-tauri/src/config/renderer.rs` (we'll write the module then its test mod):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::store::*;

    fn sample_config() -> AppConfig {
        let mut cfg = ConfigStore::default_config();
        cfg.channels.feishu.enabled = true;
        cfg.channels.feishu.app_id = "cli_abc".to_string();
        cfg.channels.feishu.app_secret = "secret123".to_string();
        cfg.channels.feishu.verification_token = "tok".to_string();
        cfg.channels.feishu.encrypt_key = "key".to_string();
        cfg.channels.wechat.enabled = true;
        cfg
    }

    #[test]
    fn render_env_includes_enabled_channels() {
        let cfg = sample_config();
        let env = render_env(&cfg);
        assert!(env.contains("FEISHU_APP_ID=cli_abc"));
        assert!(env.contains("FEISHU_APP_SECRET=secret123"));
        assert!(env.contains("FEISHU_VERIFICATION_TOKEN=tok"));
        assert!(env.contains("FEISHU_ENCRYPT_KEY=key"));
        assert!(env.contains("WECHAT_ENABLED=true"));
        assert!(env.contains("OPENCODE_SERVER_URL=http://127.0.0.1:4097"));
    }

    #[test]
    fn render_env_excludes_disabled_channels() {
        let cfg = ConfigStore::default_config();
        let env = render_env(&cfg);
        assert!(!env.contains("FEISHU_APP_ID="));
        assert!(!env.contains("WECHAT_ENABLED="));
    }

    #[test]
    fn render_jsonc_has_required_fields() {
        let cfg = sample_config();
        let jsonc = render_jsonc(&cfg);
        assert!(jsonc.contains("\"defaultAgent\": \"build\""));
        assert!(jsonc.contains("\"appId\": \"cli_abc\""));
        assert!(jsonc.contains("\"webhookPort\": 3001"));
    }

    #[test]
    fn render_env_telegram_joins_chat_ids() {
        let mut cfg = ConfigStore::default_config();
        cfg.channels.telegram.enabled = true;
        cfg.channels.telegram.bot_token = "tok".to_string();
        cfg.channels.telegram.allowed_chat_ids = vec!["111".to_string(), "222".to_string()];
        let env = render_env(&cfg);
        assert!(env.contains("TELEGRAM_BOT_TOKEN=tok"));
        assert!(env.contains("TELEGRAM_ALLOWED_CHAT_IDS=111,222"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml config::renderer
```

Expected: FAIL — `render_env` / `render_jsonc` not found.

- [ ] **Step 3: Write the implementation**

`src-tauri/src/config/renderer.rs`:
```rust
use std::path::Path;
use crate::error::AppResult;
use crate::config::store::AppConfig;

pub fn render_env(config: &AppConfig) -> String {
    let mut lines = Vec::new();

    lines.push(format!("OPENCODE_SERVER_URL={}", config.server.opencode_server_url));
    if !config.server.cwd.is_empty() {
        lines.push(format!("OPENCODE_CWD={}", config.server.cwd));
    }

    let f = &config.channels.feishu;
    if f.enabled {
        lines.push(format!("FEISHU_APP_ID={}", f.app_id));
        lines.push(format!("FEISHU_APP_SECRET={}", f.app_secret));
        if !f.verification_token.is_empty() {
            lines.push(format!("FEISHU_VERIFICATION_TOKEN={}", f.verification_token));
        }
        lines.push(format!("FEISHU_WEBHOOK_PORT={}", f.webhook_port));
        if !f.encrypt_key.is_empty() {
            lines.push(format!("FEISHU_ENCRYPT_KEY={}", f.encrypt_key));
        }
    }

    let q = &config.channels.qq;
    if q.enabled {
        lines.push(format!("QQ_APP_ID={}", q.app_id));
        lines.push(format!("QQ_SECRET={}", q.secret));
    }

    let t = &config.channels.telegram;
    if t.enabled {
        lines.push(format!("TELEGRAM_BOT_TOKEN={}", t.bot_token));
        if !t.allowed_chat_ids.is_empty() {
            lines.push(format!("TELEGRAM_ALLOWED_CHAT_IDS={}", t.allowed_chat_ids.join(",")));
        }
    }

    let d = &config.channels.discord;
    if d.enabled {
        lines.push(format!("DISCORD_BOT_TOKEN={}", d.bot_token));
        if !d.allowed_channel_ids.is_empty() {
            lines.push(format!("DISCORD_ALLOWED_CHANNEL_IDS={}", d.allowed_channel_ids.join(",")));
        }
    }

    let w = &config.channels.wechat;
    if w.enabled {
        lines.push("WECHAT_ENABLED=true".to_string());
    }

    lines.push(format!("OPENCODE_SERVER_PORT={}", config.server.port));

    lines.join("\n") + "\n"
}

pub fn render_jsonc(config: &AppConfig) -> String {
    let f = &config.channels.feishu;
    let mut s = String::new();
    s.push_str("{\n");

    if f.enabled {
        s.push_str("  \"feishu\": {\n");
        s.push_str(&format!("    \"appId\": \"{}\",\n", f.app_id));
        s.push_str(&format!("    \"appSecret\": \"{}\",\n", f.app_secret));
        s.push_str(&format!("    \"verificationToken\": \"{}\",\n", f.verification_token));
        s.push_str(&format!("    \"webhookPort\": {},\n", f.webhook_port));
        s.push_str(&format!("    \"encryptKey\": \"{}\"\n", f.encrypt_key));
        s.push_str("  },\n");
    }

    s.push_str(&format!("  \"defaultAgent\": \"{}\",\n", config.bridge.default_agent));
    s.push_str(&format!("  \"dataDir\": \"{}\",\n", config.bridge.data_dir));

    s.push_str("  \"progress\": {\n");
    s.push_str(&format!("    \"debounceMs\": {},\n", config.bridge.progress.debounce_ms));
    s.push_str(&format!("    \"maxDebounceMs\": {}\n", config.bridge.progress.max_debounce_ms));
    s.push_str("  },\n");

    let l = &config.bridge.launcher;
    s.push_str("  \"launcher\": {\n");
    s.push_str(&format!("    \"enabled\": {},\n", l.enabled));
    s.push_str(&format!("    \"autoStartServer\": {},\n", l.auto_start_server));
    s.push_str(&format!("    \"serverCommand\": \"{}\",\n", l.server_command));
    s.push_str(&format!("    \"serverStartTimeoutMs\": {},\n", l.server_start_timeout_ms));
    s.push_str(&format!("    \"probeTimeoutMs\": {}\n", l.probe_timeout_ms));
    s.push_str("  }\n");

    s.push_str("}\n");
    s
}

pub fn write_bridge_files(config: &AppConfig, bridge_dir: &Path) -> AppResult<()> {
    let env_content = render_env(config);
    let jsonc_content = render_jsonc(config);
    std::fs::write(bridge_dir.join(".env"), env_content)?;
    std::fs::write(bridge_dir.join("opencode-im.jsonc"), jsonc_content)?;
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml config::renderer
```

Expected: 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config/renderer.rs
git commit -m "feat(config): add ConfigRenderer for .env + jsonc generation"
```

---

## Task 5: LogBuffer (ring buffer)

**Files:**
- Create: `src-tauri/src/monitor/mod.rs`, `src-tauri/src/monitor/log_buffer.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Produces: `LogEntry { ts: i64, source: String, level: String, line: String }`, `LogBuffer` with `push(entry)`, `recent(source, limit) -> Vec<LogEntry>`, `recent_all(limit) -> Vec<LogEntry>`, `clear(source)`.

- [ ] **Step 1: Write the failing test**

`src-tauri/src/monitor/log_buffer.rs`:
```rust
use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub ts: i64,
    pub source: String,
    pub level: String,
    pub line: String,
}

pub struct LogBuffer {
    server: VecDeque<LogEntry>,
    bridge: VecDeque<LogEntry>,
    capacity: usize,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            server: VecDeque::with_capacity(capacity),
            bridge: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        let buf = if entry.source == "server" { &mut self.server } else { &mut self.bridge };
        if buf.len() >= self.capacity {
            buf.pop_front();
        }
        buf.push_back(entry);
    }

    pub fn recent(&self, source: &str, limit: usize) -> Vec<LogEntry> {
        let buf = if source == "server" { &self.server } else { &self.bridge };
        let skip = buf.len().saturating_sub(limit);
        buf.iter().skip(skip).cloned().collect()
    }

    pub fn recent_all(&self, limit: usize) -> Vec<LogEntry> {
        let mut all: Vec<LogEntry> = self.server.iter().chain(self.bridge.iter()).cloned().collect();
        all.sort_by_key(|e| e.ts);
        if all.len() > limit {
            all.drain(0..all.len() - limit);
        }
        all
    }

    pub fn clear(&mut self, source: &str) {
        if source == "server" { self.server.clear(); }
        else { self.bridge.clear(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(src: &str, n: i64) -> LogEntry {
        LogEntry { ts: n, source: src.to_string(), level: "info".to_string(), line: format!("line {}", n) }
    }

    #[test]
    fn evicts_oldest_when_over_capacity() {
        let mut buf = LogBuffer::new(3);
        for i in 0..5 { buf.push(entry("server", i)); }
        let recent = buf.recent("server", 10);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].line, "line 2");
    }

    #[test]
    fn recent_all_merges_and_sorts() {
        let mut buf = LogBuffer::new(100);
        buf.push(entry("server", 2));
        buf.push(entry("bridge", 1));
        buf.push(entry("server", 3));
        let all = buf.recent_all(10);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].ts, 1);
        assert_eq!(all[1].ts, 2);
        assert_eq!(all[2].ts, 3);
    }

    #[test]
    fn clear_removes_entries() {
        let mut buf = LogBuffer::new(100);
        buf.push(entry("server", 1));
        buf.clear("server");
        assert_eq!(buf.recent("server", 10).len(), 0);
    }
}
```

- [ ] **Step 2: Run test to verify it passes (test-first since impl is inline)**

Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml monitor::log_buffer
```

Expected: 3 tests PASS.

- [ ] **Step 3: Register module**

`src-tauri/src/monitor/mod.rs`:
```rust
pub mod log_buffer;
pub mod stdout_parser;
pub mod health;

pub use log_buffer::{LogBuffer, LogEntry};
```

Modify `src-tauri/src/lib.rs`, add:
```rust
pub mod monitor;
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/monitor/
git commit -m "feat(monitor): add LogBuffer ring buffer with tests"
```

---

## Task 6: StdoutParser (WeChat QR detection)

**Files:**
- Create: `src-tauri/src/monitor/stdout_parser.rs`

**Interfaces:**
- Produces: `StdoutParser` with `new()`, `feed_line(&str) -> Option<WechatQrEvent>`. `WechatQrEvent { kind: QrKind, data: String }`, `QrKind::Ascii | QrKind::Url`.

- [ ] **Step 1: Write the failing test**

`src-tauri/src/monitor/stdout_parser.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum QrKind {
    Ascii,
    Url,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WechatQrEvent {
    pub kind: QrKind,
    pub data: String,
}

pub struct StdoutParser {
    ascii_buffer: Vec<String>,
    collecting_ascii: bool,
}

impl StdoutParser {
    pub fn new() -> Self {
        Self { ascii_buffer: Vec::new(), collecting_ascii: false }
    }

    pub fn feed_line(&mut self, line: &str) -> Option<WechatQrEvent> {
        let trimmed = line.trim_end();
        if let Some(url) = extract_wechat_url(trimmed) {
            self.collecting_ascii = false;
            self.ascii_buffer.clear();
            return Some(WechatQrEvent { kind: QrKind::Url, data: url.to_string() });
        }

        if is_ascii_qr_line(trimmed) {
            if !self.collecting_ascii {
                self.collecting_ascii = true;
                self.ascii_buffer.clear();
            }
            self.ascii_buffer.push(trimmed.to_string());
            if self.ascii_buffer.len() >= 21 {
                let data = self.ascii_buffer.join("\n");
                self.collecting_ascii = false;
                self.ascii_buffer.clear();
                return Some(WechatQrEvent { kind: QrKind::Ascii, data });
            }
            return None;
        }

        if self.collecting_ascii && self.ascii_buffer.len() >= 10 {
            let data = self.ascii_buffer.join("\n");
            self.collecting_ascii = false;
            self.ascii_buffer.clear();
            return Some(WechatQrEvent { kind: QrKind::Ascii, data });
        }

        None
    }
}

fn is_ascii_qr_line(line: &str) -> bool {
    let block_chars = ['█', '▀', '▄', '▌', '▐', '■', '□'];
    let block_count = line.chars().filter(|c| block_chars.contains(c)).count();
    block_count >= 10
}

fn extract_wechat_url(line: &str) -> Option<&str> {
    let markers = ["login.weixin.qq.com", "wx.qq.com", "login.wechat.com"];
    for marker in &markers {
        if let Some(pos) = line.find(marker) {
            let start = line[..pos].rfind("https://")
                .or_else(|| line[..pos].rfind("http://"))
                .unwrap_or(0);
            let rest = &line[start..];
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            return Some(&rest[..end]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_wechat_url() {
        let mut p = StdoutParser::new();
        let ev = p.feed_line("请扫码登录: https://login.weixin.qq.com/qrcode/abc123 ");
        assert_eq!(ev, Some(WechatQrEvent {
            kind: QrKind::Url,
            data: "https://login.weixin.qq.com/qrcode/abc123".to_string(),
        }));
    }

    #[test]
    fn detects_ascii_qr_block() {
        let mut p = StdoutParser::new();
        let mut last_ev = None;
        for _ in 0..21 {
            let line: String = std::iter::repeat('█').take(20).collect();
            last_ev = p.feed_line(&line);
        }
        assert!(matches!(last_ev, Some(WechatQrEvent { kind: QrKind::Ascii, .. })));
    }

    #[test]
    fn ignores_normal_lines() {
        let mut p = StdoutParser::new();
        assert_eq!(p.feed_line("[INFO] server starting on port 4096"), None);
    }

    #[test]
    fn url_detection_resets_ascii_collection() {
        let mut p = StdoutParser::new();
        let block_line: String = std::iter::repeat('█').take(20).collect();
        let _ = p.feed_line(&block_line);
        let ev = p.feed_line("https://login.weixin.qq.com/qrcode/xyz");
        assert_eq!(ev.unwrap().kind, QrKind::Url);
    }
}
```

- [ ] **Step 2: Run tests**

Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml monitor::stdout_parser
```

Expected: 4 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/monitor/stdout_parser.rs
git commit -m "feat(monitor): add StdoutParser for WeChat QR detection"
```

---

## Task 7: HealthChecker

**Files:**
- Create: `src-tauri/src/monitor/health.rs`

**Interfaces:**
- Produces: `HealthChecker::new(url: String)`, `check_once() -> bool` (blocking GET to `/session/status`).

- [ ] **Step 1: Write implementation with test**

`src-tauri/src/monitor/health.rs`:
```rust
use std::time::Duration;

pub struct HealthChecker {
    url: String,
    client: reqwest::blocking::Client,
}

impl HealthChecker {
    pub fn new(server_url: &str) -> Self {
        let url = format!("{}/session/status", server_url.trim_end_matches('/'));
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Self { url, client }
    }

    pub fn check_once(&self) -> bool {
        match self.client.get(&self.url).send() {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/monitor/health.rs
git commit -m "feat(monitor): add HealthChecker for server status polling"
```

---

## Task 8: ProcessManager and supervisor

**Files:**
- Create: `src-tauri/src/process/mod.rs`, `src-tauri/src/process/manager.rs`, `src-tauri/src/process/supervisor.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `AppError` (Task 2), `LogEntry`/`LogBuffer` (Task 5), `StdoutParser` (Task 6).
- Produces: `ProcessTarget` enum (`Server`/`Bridge`), `ProcessState` struct, `ProcessManager` with `start_server`, `start_bridge`, `stop`, `restart`, `get_state`. Emits via callback closures rather than Tauri events directly (decoupled for testability).

- [ ] **Step 1: Write ProcessState and ProcessTarget**

`src-tauri/src/process/mod.rs`:
```rust
pub mod manager;
pub mod supervisor;

pub use manager::{ProcessManager, ProcessState, ProcessTarget, ProcessStateKind};
```

`src-tauri/src/process/manager.rs`:
```rust
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use tokio::process::Child;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ProcessStateKind {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessState {
    pub state: ProcessStateKind,
    pub pid: Option<u32>,
    pub started_at: Option<i64>,
    pub uptime_sec: Option<u64>,
    pub exit_code: Option<i32>,
    pub healthy: Option<bool>,
}

impl Default for ProcessState {
    fn default() -> Self {
        Self {
            state: ProcessStateKind::Stopped,
            pid: None, started_at: None, uptime_sec: None, exit_code: None, healthy: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessTarget {
    Server,
    Bridge,
}

pub(crate) struct ManagedProcess {
    pub state: ProcessState,
    pub child: Option<Child>,
    pub started_at_instant: Option<Instant>,
}

impl ManagedProcess {
    fn new() -> Self {
        Self { state: ProcessState::default(), child: None, started_at_instant: None }
    }
}

pub type StateCallback = Arc<dyn Fn(ProcessTarget, ProcessState) + Send + Sync>;
pub type LogCallback = Arc<dyn Fn(crate::monitor::LogEntry) + Send + Sync>;
pub type QrCallback = Arc<dyn Fn(crate::monitor::stdout_parser::WechatQrEvent) + Send + Sync>;

pub struct ProcessManager {
    server: Arc<Mutex<ManagedProcess>>,
    bridge: Arc<Mutex<ManagedProcess>>,
    on_state: StateCallback,
    on_log: LogCallback,
    on_qr: QrCallback,
    runtime: tokio::runtime::Runtime,
}

impl ProcessManager {
    pub fn new(on_state: StateCallback, on_log: LogCallback, on_qr: QrCallback) -> Self {
        Self {
            server: Arc::new(Mutex::new(ManagedProcess::new())),
            bridge: Arc::new(Mutex::new(ManagedProcess::new())),
            on_state,
            on_log,
            on_qr,
            runtime: tokio::runtime::Runtime::new().expect("failed to create tokio runtime"),
        }
    }

    fn target_ref(&self, target: ProcessTarget) -> &Arc<Mutex<ManagedProcess>> {
        match target {
            ProcessTarget::Server => &self.server,
            ProcessTarget::Bridge => &self.bridge,
        }
    }

    fn emit_state(&self, target: ProcessTarget) {
        let state = self.target_ref(target).lock().unwrap().state.clone();
        (self.on_state)(target, state);
    }

    fn now_ts() -> i64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
    }

    pub fn start_server(&self, port: u16, cwd: &str, extra_env: &std::collections::HashMap<String, String>) -> AppResult<ProcessState> {
        {
            let mut mp = self.server.lock().unwrap();
            if mp.state.state == ProcessStateKind::Running || mp.state.state == ProcessStateKind::Starting {
                return Err(AppError::Process("server already running".into()));
            }
            mp.state = ProcessState { state: ProcessStateKind::Starting, ..Default::default() };
        }
        self.emit_state(ProcessTarget::Server);

        let mut cmd = tokio::process::Command::new("opencode");
        cmd.arg("serve").arg("--port").arg(port.to_string());
        cmd.current_dir(if cwd.is_empty() { "." } else { cwd });
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.envs(extra_env.iter());
        cmd.kill_on_drop(true);

        let child = cmd.spawn().map_err(|e| AppError::Process(format!("failed to spawn opencode: {}", e)))?;
        let pid = child.id();
        let now = Self::now_ts();
        let instant = Instant::now();

        {
            let mut mp = self.server.lock().unwrap();
            mp.child = Some(child);
            mp.started_at_instant = Some(instant);
            mp.state = ProcessState {
                state: ProcessStateKind::Running,
                pid,
                started_at: Some(now),
                uptime_sec: Some(0),
                exit_code: None,
                healthy: None,
            };
        }
        self.emit_state(ProcessTarget::Server);

        let on_log = self.on_log.clone();
        let on_state = self.on_state.clone();
        let server_ref = self.server.clone();
        self.runtime.spawn(async move {
            supervisor::supervise(server_ref, ProcessTarget::Server, on_log, on_state).await;
        });

        Ok(self.server.lock().unwrap().state.clone())
    }

    pub fn start_bridge(&self, bridge_dir: &std::path::Path, use_bun: bool) -> AppResult<ProcessState> {
        {
            let mut mp = self.bridge.lock().unwrap();
            if mp.state.state == ProcessStateKind::Running || mp.state.state == ProcessStateKind::Starting {
                return Err(AppError::Process("bridge already running".into()));
            }
            mp.state = ProcessState { state: ProcessStateKind::Starting, ..Default::default() };
        }
        self.emit_state(ProcessTarget::Bridge);

        let mut cmd = if use_bun {
            let mut c = tokio::process::Command::new("bun");
            c.arg("run").arg("src/index.ts");
            c
        } else {
            let mut c = tokio::process::Command::new("npx");
            c.arg("tsx").arg("src/index.ts");
            c
        };
        cmd.current_dir(bridge_dir);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.kill_on_drop(true);

        let child = cmd.spawn().map_err(|e| AppError::Process(format!("failed to spawn bridge: {}", e)))?;
        let pid = child.id();
        let now = Self::now_ts();
        let instant = Instant::now();

        {
            let mut mp = self.bridge.lock().unwrap();
            mp.child = Some(child);
            mp.started_at_instant = Some(instant);
            mp.state = ProcessState {
                state: ProcessStateKind::Running,
                pid,
                started_at: Some(now),
                uptime_sec: Some(0),
                exit_code: None,
                healthy: None,
            };
        }
        self.emit_state(ProcessTarget::Bridge);

        let on_log = self.on_log.clone();
        let on_state = self.on_state.clone();
        let on_qr = self.on_qr.clone();
        let bridge_ref = self.bridge.clone();
        self.runtime.spawn(async move {
            supervisor::supervise_with_qr(bridge_ref, ProcessTarget::Bridge, on_log, on_state, on_qr).await;
        });

        Ok(self.bridge.lock().unwrap().state.clone())
    }

    pub fn stop(&self, target: ProcessTarget) -> AppResult<()> {
        let mp_ref = self.target_ref(target).clone();
        let pid;
        {
            let mut mp = mp_ref.lock().unwrap();
            match mp.state.state {
                ProcessStateKind::Running | ProcessStateKind::Starting => {
                    mp.state.state = ProcessStateKind::Stopping;
                    pid = mp.state.pid;
                }
                _ => return Ok(()),
            }
        }
        self.emit_state(target);

        if let Some(mut child) = mp_ref.lock().unwrap().child.take() {
            let _ = child.start_kill();
            let rt = &self.runtime;
            let exited = rt.block_on(async {
                tokio::select! {
                    _ = child.wait() => true,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => false,
                }
            });
            if !exited {
                let _ = child.kill().await;
            }
            let exit_code = rt.block_on(async { child.wait().await }).ok().and_then(|s| s.code());
            {
                let mut mp = mp_ref.lock().unwrap();
                mp.state = ProcessState {
                    state: ProcessStateKind::Stopped,
                    pid: None, started_at: None, uptime_sec: None,
                    exit_code, healthy: None,
                };
                mp.started_at_instant = None;
            }
            self.emit_state(target);
        }
        Ok(())
    }

    pub fn restart(&self, target: ProcessTarget, cfg: &crate::config::AppConfig, use_bun: bool) -> AppResult<ProcessState> {
        self.stop(target)?;
        match target {
            ProcessTarget::Server => self.start_server(cfg.server.port, &cfg.server.cwd, &cfg.server.extra_env),
            ProcessTarget::Bridge => {
                let store = crate::config::ConfigStore::new();
                let bridge_dir = store.bridge_install_path(cfg);
                self.start_bridge(&bridge_dir, use_bun)
            }
        }
    }

    pub fn get_state(&self, target: ProcessTarget) -> ProcessState {
        let mp = self.target_ref(target).lock().unwrap();
        let mut state = mp.state.clone();
        if state.state == ProcessStateKind::Running {
            if let Some(instant) = mp.started_at_instant {
                state.uptime_sec = Some(instant.elapsed().as_secs());
            }
        }
        state
    }

    pub fn set_health(&self, target: ProcessTarget, healthy: bool) {
        let mp_ref = self.target_ref(target).clone();
        let mut mp = mp_ref.lock().unwrap();
        if mp.state.state == ProcessStateKind::Running {
            mp.state.healthy = Some(healthy);
            let state = mp.state.clone();
            drop(mp);
            (self.on_state)(target, state);
        }
    }
}
```

`src-tauri/src/process/supervisor.rs`:
```rust
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use super::manager::{ManagedProcess, ProcessTarget, ProcessState, ProcessStateKind, StateCallback, LogCallback, QrCallback};
use crate::monitor::{LogEntry, stdout_parser::StdoutParser};

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn source_str(target: ProcessTarget) -> &'static str {
    match target { ProcessTarget::Server => "server", ProcessTarget::Bridge => "bridge" }
}

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

async fn read_stream_with_qr<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    source: String,
    level: String,
    on_log: LogCallback,
    on_qr: QrCallback,
    parser: Arc<Mutex<StdoutParser>>,
) {
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if level == "info" {
            if let Some(ev) = parser.lock().unwrap().feed_line(&line) {
                on_qr(ev);
            }
        }
        on_log(LogEntry { ts: now_ts(), source: source.clone(), level: level.clone(), line });
    }
}

pub async fn supervise(
    process: Arc<Mutex<ManagedProcess>>,
    target: ProcessTarget,
    on_log: LogCallback,
    on_state: StateCallback,
) {
    let (stdout, stderr, child_ref) = {
        let mut mp = process.lock().unwrap();
        let child = mp.child.as_mut().expect("no child");
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        (stdout, stderr, process.clone())
    };
    let source = source_str(target).to_string();
    let log_clone = on_log.clone();
    let stdout_task = if let Some(out) = stdout {
        Some(tokio::spawn(read_stream(out, source.clone(), "info".into(), on_log)))
    } else { None };
    let stderr_task = if let Some(err) = stderr {
        Some(tokio::spawn(read_stream(err, source.clone(), "error".into(), log_clone)))
    } else { None };

    if let Some(t) = stdout_task { let _ = t.await; }
    if let Some(t) = stderr_task { let _ = t.await; }

    let exit_code = {
        let mut mp = child_ref.lock().unwrap();
        let child = mp.child.as_mut().expect("no child");
        child.wait().await.ok().and_then(|s| s.code())
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
}

pub async fn supervise_with_qr(
    process: Arc<Mutex<ManagedProcess>>,
    target: ProcessTarget,
    on_log: LogCallback,
    on_state: StateCallback,
    on_qr: QrCallback,
) {
    let (stdout, stderr, child_ref) = {
        let mut mp = process.lock().unwrap();
        let child = mp.child.as_mut().expect("no child");
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        (stdout, stderr, process.clone())
    };
    let source = source_str(target).to_string();
    let parser = Arc::new(Mutex::new(StdoutParser::new()));
    let log_clone = on_log.clone();
    let qr_clone = on_qr.clone();
    let parser_clone = parser.clone();
    let stdout_task = if let Some(out) = stdout {
        Some(tokio::spawn(read_stream_with_qr(out, source.clone(), "info".into(), on_log, qr_clone, parser_clone)))
    } else { None };
    let stderr_task = if let Some(err) = stderr {
        Some(tokio::spawn(read_stream(err, source.clone(), "error".into(), log_clone)))
    } else { None };

    if let Some(t) = stdout_task { let _ = t.await; }
    if let Some(t) = stderr_task { let _ = t.await; }

    let exit_code = {
        let mut mp = child_ref.lock().unwrap();
        let child = mp.child.as_mut().expect("no child");
        child.wait().await.ok().and_then(|s| s.code())
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
}
```

Note: `ManagedProcess` and its fields are `pub(crate)` so the supervisor module can access `child` and `state`. The `ProcessManager` fields `server`/`bridge`/`runtime` are private; supervisor receives `Arc<Mutex<ManagedProcess>>` clones via the spawn call.

- [ ] **Step 2: Register modules in lib.rs**

Modify `src-tauri/src/lib.rs`, add:
```rust
pub mod process;
```

- [ ] **Step 3: Verify it compiles**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: no errors. Fix any visibility/import issues that arise.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/process/
git commit -m "feat(process): add ProcessManager and supervisor with stdout/stderr streaming"
```

---

## Task 9: BridgeInstaller and env_check

**Files:**
- Create: `src-tauri/src/bridge/mod.rs`, `src-tauri/src/bridge/installer.rs`, `src-tauri/src/bridge/env_check.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Produces: `BridgeInstaller::new(default_path: PathBuf)`, `is_installed() -> bool`, `install() -> AppResult<()>`, `check_update() -> AppResult<bool>`, `update() -> AppResult<()>`, `reinstall() -> AppResult<()>`. `DepStatus { opencode, bun, node, npm, git }`, `check_deps() -> DepStatus`.

- [ ] **Step 1: Write env_check.rs**

`src-tauri/src/bridge/env_check.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::process::Command;

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
    let check = if cfg!(target_os = "windows") {
        Command::new("where").arg(cmd).output()
    } else {
        Command::new("which").arg(cmd).output()
    };
    match check {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
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

- [ ] **Step 2: Write installer.rs**

`src-tauri/src/bridge/installer.rs`:
```rust
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
```

- [ ] **Step 3: Write mod.rs and register**

`src-tauri/src/bridge/mod.rs`:
```rust
pub mod installer;
pub mod env_check;

pub use installer::BridgeInstaller;
pub use env_check::{check_deps, DepStatus};
```

Modify `src-tauri/src/lib.rs`, add:
```rust
pub mod bridge;
```

- [ ] **Step 4: Verify it compiles**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/bridge/
git commit -m "feat(bridge): add BridgeInstaller and dependency checker"
```

---

## Task 10: AppState and Tauri commands

**Files:**
- Create: `src-tauri/src/state.rs`, `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: all prior modules.
- Produces: `AppState` (managed Tauri state), all `#[tauri::command]` functions, wired into `run()`.

- [ ] **Step 1: Write state.rs**

`src-tauri/src/state.rs`:
```rust
use std::sync::Mutex;
use crate::config::{ConfigStore, AppConfig};
use crate::process::ProcessManager;
use crate::monitor::LogBuffer;

pub struct AppState {
    pub config_store: ConfigStore,
    pub process_manager: ProcessManager,
    pub log_buffer: Mutex<LogBuffer>,
}

impl AppState {
    pub fn new(
        process_manager: ProcessManager,
    ) -> Self {
        Self {
            config_store: ConfigStore::new(),
            process_manager,
            log_buffer: Mutex::new(LogBuffer::new(5000)),
        }
    }

    pub fn load_config(&self) -> crate::error::AppResult<AppConfig> {
        self.config_store.load()
    }

    pub fn save_config(&self, config: &AppConfig) -> crate::error::AppResult<()> {
        self.config_store.save(config)
    }
}
```

- [ ] **Step 2: Write commands.rs**

`src-tauri/src/commands.rs`:
```rust
use std::sync::Mutex;
use tauri::State;
use crate::bridge::{check_deps, DepStatus, BridgeInstaller};
use crate::config::{AppConfig, ConfigStore, renderer};
use crate::error::{AppError, AppResult};
use crate::monitor::{LogEntry, stdout_parser::WechatQrEvent};
use crate::process::{ProcessManager, ProcessState, ProcessTarget};
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
            let deps = check_deps();
            state.process_manager.start_bridge(installer.path(), deps.bun)
        }
    }
}

#[tauri::command]
pub fn stop_process(target: String, state: State<'_, AppState>) -> AppResult<()> {
    let target = parse_target(&target)?;
    state.process_manager.stop(target)
}

#[tauri::command]
pub fn restart_process(target: String, state: State<'_, AppState>) -> AppResult<ProcessState> {
    let target = parse_target(&target)?;
    let cfg = state.load_config()?;
    let deps = check_deps();
    state.process_manager.restart(target, &cfg, deps.bun)
}

pub fn do_start_all(state: &AppState) -> AppResult<()> {
    let cfg = state.load_config()?;
    state.process_manager.start_server(cfg.server.port, &cfg.server.cwd, &cfg.server.extra_env)?;
    let installer = BridgeInstaller::new(state.config_store.bridge_install_path(&cfg));
    if !installer.is_installed() {
        installer.install()?;
    }
    renderer::write_bridge_files(&cfg, installer.path())?;
    let deps = check_deps();
    state.process_manager.start_bridge(installer.path(), deps.bun)?;
    Ok(())
}

pub fn do_stop_all(state: &AppState) -> AppResult<()> {
    state.process_manager.stop(ProcessTarget::Bridge)?;
    state.process_manager.stop(ProcessTarget::Server)?;
    Ok(())
}

pub fn do_restart_all(state: &AppState) -> AppResult<()> {
    do_stop_all(state)?;
    do_start_all(state)
}

#[tauri::command]
pub fn start_all(state: State<'_, AppState>) -> AppResult<()> { do_start_all(state.inner()) }

#[tauri::command]
pub fn stop_all(state: State<'_, AppState>) -> AppResult<()> { do_stop_all(state.inner()) }

#[tauri::command]
pub fn restart_all(state: State<'_, AppState>) -> AppResult<()> { do_restart_all(state.inner()) }

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
pub fn export_logs(source: String, state: State<'_, AppState>) -> AppResult<String> {
    let buf = state.log_buffer.lock().unwrap();
    let entries = if source == "all" { buf.recent_all(100000) } else { buf.recent(&source, 100000) };
    drop(buf);
    let content = entries.iter()
        .map(|e| format!("[{}] [{}] [{}] {}", e.ts, e.source, e.level, e.line))
        .collect::<Vec<_>>()
        .join("\n");
    let path = dirs::download_dir()
        .or_else(|| dirs::home_dir())
        .unwrap_or_default()
        .join(format!("opencodedeck-logs-{}.txt", source));
    std::fs::write(&path, content)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn check_deps() -> AppResult<DepStatus> {
    Ok(check_deps())
}
```

- [ ] **Step 3: Write state.rs**

`src-tauri/src/state.rs`:
```rust
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
```

- [ ] **Step 4: Write lib.rs (final wiring)**

Replace `src-tauri/src/lib.rs` entirely with:
```rust
pub mod error;
pub mod config;
pub mod process;
pub mod bridge;
pub mod monitor;
pub mod state;
pub mod commands;

use std::sync::{Arc, Mutex};
use tauri::{Manager, Emitter};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let handle = app.handle().clone();

            let log_buffer: Arc<Mutex<monitor::LogBuffer>> = Arc::new(Mutex::new(monitor::LogBuffer::new(5000)));
            let log_buffer_for_cb = log_buffer.clone();

            let on_state: process::StateCallback = Arc::new({
                let handle = handle.clone();
                move |target, state| {
                    let target_str = if target == process::ProcessTarget::Server { "server" } else { "bridge" };
                    let _ = handle.emit("state://update", serde_json::json!({ "target": target_str, "state": state }));
                }
            });

            let on_log: process::LogCallback = Arc::new({
                let handle = handle.clone();
                move |entry: monitor::LogEntry| {
                    let mut buf = log_buffer_for_cb.lock().unwrap();
                    buf.push(entry.clone());
                    drop(buf);
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
            app.manage(app_state);

            let handle2 = handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tauri::async_runtime::sleep(std::time::Duration::from_secs(5)).await;
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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::start_process,
            commands::stop_process,
            commands::restart_process,
            commands::start_all,
            commands::stop_all,
            commands::restart_all,
            commands::get_config,
            commands::save_config,
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
```

- [ ] **Step 5: Verify it compiles**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: no errors. `ManagedProcess` fields are `pub(crate)` from Task 8, so the supervisor can access them. `ProcessManager` keeps callbacks as `Arc` (cloned into supervisor tasks in Task 8). No `replace_callbacks` needed — callbacks are constructed in `setup` with the `AppHandle` already available, before `ProcessManager::new`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/state.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(app): wire AppState, commands, and Tauri event emission"
```

---

## Task 11: System tray

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml` (ensure `tray-icon` feature)
- Modify: `src-tauri/tauri.conf.json` (window close-to-tray behavior)

**Interfaces:**
- Produces: a system tray with status icon and menu (start/stop/restart all, show window, quit). Close window hides to tray.

- [ ] **Step 1: Add tray setup in lib.rs**

Add to the `setup` closure in `src-tauri/src/lib.rs`, before `Ok(())`:

```rust
use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};

let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
let start_all_item = MenuItem::with_id(app, "start_all", "启动全部", true, None::<&str>)?;
let stop_all_item = MenuItem::with_id(app, "stop_all", "停止全部", true, None::<&str>)?;
let restart_all_item = MenuItem::with_id(app, "restart_all", "重启全部", true, None::<&str>)?;
let sep = PredefinedMenuItem::separator(app)?;
let menu = Menu::with_items(app, &[
    &start_all_item, &stop_all_item, &restart_all_item, &sep, &show_item, &sep, &quit_item,
])?;

let _tray = TrayIconBuilder::new()
    .icon(app.default_window_icon().unwrap().clone())
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
            use tauri::Manager;
            let state = app.state::<state::AppState>();
            let _ = state.process_manager.stop(process::ProcessTarget::Bridge);
            let _ = state.process_manager.stop(process::ProcessTarget::Server);
            app.exit(0);
        }
        "start_all" => {
            let state = app.state::<state::AppState>();
            let _ = commands::start_all(state.inner().clone().into());
        }
        "stop_all" => {
            let state = app.state::<state::AppState>();
            let _ = commands::stop_all(state.inner().clone().into());
        }
        "restart_all" => {
            let state = app.state::<state::AppState>();
            let _ = commands::restart_all(state.inner().clone().into());
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
```

The `start_all`/`stop_all`/`restart_all` menu handlers call the `do_start_all`/`do_stop_all`/`do_restart_all` free functions defined in Task 10's `commands.rs` (they take `&AppState`, not the Tauri `State` guard). In the tray menu `on_menu_event` closure, access `AppState` via `app.state::<state::AppState>()` and call them:

```rust
"start_all" => {
    let state = app.state::<state::AppState>();
    let _ = commands::do_start_all(state.inner());
}
"stop_all" => {
    let state = app.state::<state::AppState>();
    let _ = commands::do_stop_all(state.inner());
}
"restart_all" => {
    let state = app.state::<state::AppState>();
    let _ = commands::do_restart_all(state.inner());
}
```

- [ ] **Step 2: Handle window close-to-tray**

Add to the `setup` closure (after tray creation):
```rust
let main_window = app.get_webview_window("main").unwrap();
let hide_handle = main_window.clone();
main_window.on_window_event(move |event| {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = hide_handle.hide();
    }
});
```

- [ ] **Step 3: Verify it compiles and runs**

Run:
```bash
npm run tauri dev
```

Expected: window opens, tray icon appears, closing window hides it (doesn't quit), clicking tray toggles window, menu items work.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(tray): add system tray with start/stop/restart menu and close-to-tray"
```

---

## Task 12: Frontend types and Tauri bindings

**Files:**
- Create: `src/lib/types.ts`, `src/lib/tauri.ts`, `src/hooks/useTauriEvent.ts`, `src/hooks/useProcessState.ts`

**Interfaces:**
- Produces: TS types mirroring Rust structs, typed `invoke` wrappers, event hooks.

- [ ] **Step 1: Write types.ts**

`src/lib/types.ts`:
```ts
export type ProcessStateKind = "Stopped" | "Starting" | "Running" | "Stopping" | "Failed"

export interface ProcessState {
  state: ProcessStateKind
  pid: number | null
  startedAt: number | null
  uptimeSec: number | null
  exitCode: number | null
  healthy: boolean | null
}

export interface FullState {
  server: ProcessState
  bridge: ProcessState
}

export type ProcessTarget = "server" | "bridge"

export interface ServerConfig {
  port: number
  opencodeServerUrl: string
  cwd: string
  extraEnv: Record<string, string>
}

export interface ProgressConfig {
  debounceMs: number
  maxDebounceMs: number
}

export interface LauncherConfig {
  enabled: boolean
  autoStartServer: boolean
  serverCommand: string
  serverStartTimeoutMs: number
  probeTimeoutMs: number
}

export interface BridgeConfig {
  installPath: string | null
  defaultAgent: string
  dataDir: string
  progress: ProgressConfig
  launcher: LauncherConfig
}

export interface FeishuConfig {
  enabled: boolean
  appId: string
  appSecret: string
  verificationToken: string
  webhookPort: number
  encryptKey: string
}

export interface QqConfig {
  enabled: boolean
  appId: string
  secret: string
}

export interface TelegramConfig {
  enabled: boolean
  botToken: string
  allowedChatIds: string[]
}

export interface DiscordConfig {
  enabled: boolean
  botToken: string
  allowedChannelIds: string[]
}

export interface WechatConfig {
  enabled: boolean
}

export interface ChannelsConfig {
  feishu: FeishuConfig
  qq: QqConfig
  telegram: TelegramConfig
  discord: DiscordConfig
  wechat: WechatConfig
}

export interface AppConfig {
  version: number
  server: ServerConfig
  bridge: BridgeConfig
  channels: ChannelsConfig
}

export interface LogEntry {
  ts: number
  source: "server" | "bridge"
  level: "info" | "error"
  line: string
}

export type QrKind = "ascii" | "url"

export interface WechatQrEvent {
  kind: QrKind
  data: string
}

export interface DepStatus {
  opencode: boolean
  bun: boolean
  node: boolean
  npm: boolean
  git: boolean
}

export type AppError = { kind: "Io" | "Config" | "Process" | "BridgeInstall" | "EnvNotFound"; message: string }
```

- [ ] **Step 2: Write tauri.ts**

`src/lib/tauri.ts`:
```ts
import { invoke } from "@tauri-apps/api/core"
import type { AppConfig, DepStatus, FullState, LogEntry, ProcessState, ProcessTarget } from "./types"

export const getState = () => invoke<FullState>("get_state")
export const startProcess = (target: ProcessTarget) => invoke<ProcessState>("start_process", { target })
export const stopProcess = (target: ProcessTarget) => invoke<void>("stop_process", { target })
export const restartProcess = (target: ProcessTarget) => invoke<ProcessState>("restart_process", { target })
export const startAll = () => invoke<void>("start_all")
export const stopAll = () => invoke<void>("stop_all")
export const restartAll = () => invoke<void>("restart_all")
export const getConfig = () => invoke<AppConfig>("get_config")
export const saveConfig = (config: AppConfig) => invoke<void>("save_config", { config })
export const checkBridgeUpdate = () => invoke<boolean>("check_bridge_update")
export const updateBridge = () => invoke<void>("update_bridge")
export const reinstallBridge = () => invoke<void>("reinstall_bridge")
export const getLogHistory = (source: "server" | "bridge" | "all", limit: number) =>
  invoke<LogEntry[]>("get_log_history", { source, limit })
export const clearLogs = (source: string) => invoke<void>("clear_logs", { source })
export const exportLogs = (source: string) => invoke<string>("export_logs", { source })
export const checkDeps = () => invoke<DepStatus>("check_deps")
```

- [ ] **Step 3: Write hooks**

`src/hooks/useTauriEvent.ts`:
```ts
import { useEffect, useRef } from "react"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

export function useTauriEvent<T>(event: string, handler: (payload: T) => void) {
  const handlerRef = useRef(handler)
  handlerRef.current = handler

  useEffect(() => {
    let unlisten: UnlistenFn | undefined
    listen<T>(event, (e) => handlerRef.current(e.payload)).then((fn) => {
      unlisten = fn
    })
    return () => { unlisten?.() }
  }, [event])
}
```

`src/hooks/useProcessState.ts`:
```ts
import { useState, useCallback } from "react"
import { useTauriEvent } from "./useTauriEvent"
import { getState } from "../lib/tauri"
import type { FullState, ProcessTarget } from "../lib/types"

export function useProcessState() {
  const [state, setState] = useState<FullState>({ server: { state: "Stopped", pid: null, startedAt: null, uptimeSec: null, exitCode: null, healthy: null }, bridge: { state: "Stopped", pid: null, startedAt: null, uptimeSec: null, exitCode: null, healthy: null } })

  useTauriEvent<{ target: ProcessTarget; state: FullState["server"] }>("state://update", ({ target, state: ps }) => {
    setState((prev) => ({ ...prev, [target]: ps }))
  })

  const refresh = useCallback(() => { getState().then(setState).catch(() => {}) }, [])

  return { state, refresh }
}
```

- [ ] **Step 4: Verify it type-checks**

Run:
```bash
npx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/lib/ src/hooks/
git commit -m "feat(frontend): add TS types, Tauri bindings, and event hooks"
```

---

## Task 13: shadcn/ui setup and base components

**Files:**
- Create: `components.json`, `src/lib/utils.ts`, `src/components/ui/*` (button, card, input, switch, label, tabs, dialog, toast/sonner)
- Modify: `src/styles/globals.css` (add shadcn CSS variables)
- Modify: `tailwind.config.ts` (add shadcn theme)

**Interfaces:**
- Produces: shadcn/ui components available for import in pages.

- [ ] **Step 1: Initialize shadcn**

Run:
```bash
npx shadcn@latest init
```

Answer prompts: style=new york, base color=neutral, css variables=yes. This creates `components.json`, `src/lib/utils.ts`, and updates `globals.css`/`tailwind.config.ts`.

If interactive prompts fail in the agent environment, create files manually:

`components.json`:
```json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "new-york",
  "rsc": false,
  "tsx": true,
  "tailwind": {
    "config": "tailwind.config.ts",
    "css": "src/styles/globals.css",
    "baseColor": "neutral",
    "cssVariables": true,
    "prefix": ""
  },
  "aliases": {
    "components": "@/components",
    "utils": "@/lib/utils",
    "ui": "@/components/ui",
    "lib": "@/lib",
    "hooks": "@/hooks"
  },
  "iconLibrary": "lucide"
}
```

Add `@/*` path alias to `tsconfig.json` compilerOptions:
```json
"baseUrl": ".",
"paths": { "@/*": ["./src/*"] }
```

Update `vite.config.ts` to resolve the alias:
```ts
import path from "path"
import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"

export default defineConfig({
  plugins: [react()],
  resolve: { alias: { "@": path.resolve(__dirname, "./src") } },
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_"],
  build: { target: "es2021", minify: "esbuild", sourcemap: false },
})
```

`src/lib/utils.ts`:
```ts
import { type ClassValue, clsx } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
```

Install shadcn peer deps:
```bash
npm install clsx tailwind-merge class-variance-authority
```

- [ ] **Step 2: Add shadcn components**

Run:
```bash
npx shadcn@latest add button card input switch label tabs dialog sonner badge
```

If this fails, manually create each component file from shadcn docs. Verify they exist:
```bash
ls src/components/ui/
```

Expected: button.tsx, card.tsx, input.tsx, switch.tsx, label.tsx, tabs.tsx, dialog.tsx, sonner.tsx, badge.tsx

- [ ] **Step 3: Update globals.css with shadcn variables**

Replace `src/styles/globals.css`:
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  :root {
    --background: 0 0% 100%;
    --foreground: 0 0% 3.9%;
    --card: 0 0% 100%;
    --card-foreground: 0 0% 3.9%;
    --primary: 0 0% 9%;
    --primary-foreground: 0 0% 98%;
    --secondary: 0 0% 96.1%;
    --secondary-foreground: 0 0% 9%;
    --muted: 0 0% 96.1%;
    --muted-foreground: 0 0% 45.1%;
    --accent: 0 0% 96.1%;
    --accent-foreground: 0 0% 9%;
    --destructive: 0 84.2% 60.2%;
    --destructive-foreground: 0 0% 98%;
    --border: 0 0% 89.8%;
    --input: 0 0% 89.8%;
    --ring: 0 0% 3.9%;
    --radius: 0.5rem;
  }
  .dark {
    --background: 0 0% 3.9%;
    --foreground: 0 0% 98%;
    --card: 0 0% 3.9%;
    --card-foreground: 0 0% 98%;
    --primary: 0 0% 98%;
    --primary-foreground: 0 0% 9%;
    --secondary: 0 0% 14.9%;
    --secondary-foreground: 0 0% 98%;
    --muted: 0 0% 14.9%;
    --muted-foreground: 0 0% 63.9%;
    --accent: 0 0% 14.9%;
    --accent-foreground: 0 0% 98%;
    --destructive: 0 62.8% 30.6%;
    --destructive-foreground: 0 0% 98%;
    --border: 0 0% 14.9%;
    --input: 0 0% 14.9%;
    --ring: 0 0% 83.1%;
  }
}

@layer base {
  * { @apply border-border; }
  body { @apply bg-background text-foreground; }
}
```

Update `tailwind.config.ts` to include shadcn theme mapping:
```ts
import type { Config } from "tailwindcss"

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: ["class"],
  theme: {
    extend: {
      colors: {
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: { DEFAULT: "hsl(var(--primary))", foreground: "hsl(var(--primary-foreground))" },
        secondary: { DEFAULT: "hsl(var(--secondary))", foreground: "hsl(var(--secondary-foreground))" },
        destructive: { DEFAULT: "hsl(var(--destructive))", foreground: "hsl(var(--destructive-foreground))" },
        muted: { DEFAULT: "hsl(var(--muted))", foreground: "hsl(var(--muted-foreground))" },
        accent: { DEFAULT: "hsl(var(--accent))", foreground: "hsl(var(--accent-foreground))" },
        card: { DEFAULT: "hsl(var(--card))", foreground: "hsl(var(--card-foreground))" },
      },
      borderRadius: { lg: "var(--radius)", md: "calc(var(--radius) - 2px)", sm: "calc(var(--radius) - 4px)" },
    },
  },
  plugins: [],
} satisfies Config
```

- [ ] **Step 4: Verify build**

Run:
```bash
npx tsc --noEmit && npm run build
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(frontend): setup shadcn/ui with components and theme"
```

---

## Task 14: ProcessCard and LogView components

**Files:**
- Create: `src/components/ProcessCard.tsx`, `src/components/LogView.tsx`

**Interfaces:**
- Consumes: types and hooks from Task 12.
- Produces: `ProcessCard` (shows state, pid, uptime, health, start/stop/restart buttons), `LogView` (tabs, auto-scroll, pause/clear/export).

- [ ] **Step 1: Write ProcessCard.tsx**

`src/components/ProcessCard.tsx`:
```tsx
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Play, Square, RotateCcw } from "lucide-react"
import type { ProcessState, ProcessTarget } from "@/lib/types"
import { startProcess, stopProcess, restartProcess } from "@/lib/tauri"
import { toast } from "sonner"

const stateColor: Record<string, string> = {
  Running: "bg-green-500",
  Stopped: "bg-gray-400",
  Starting: "bg-yellow-500",
  Stopping: "bg-orange-500",
  Failed: "bg-red-500",
}

export function ProcessCard({ target, state }: { target: ProcessTarget; state: ProcessState }) {
  const label = target === "server" ? "opencode server" : "bridge"
  const isRunning = state.state === "Running"
  const isBusy = state.state === "Starting" || state.state === "Stopping"

  const handleStart = () => startProcess(target).catch((e) => toast.error(`启动失败: ${e}`))
  const handleStop = () => stopProcess(target).catch((e) => toast.error(`停止失败: ${e}`))
  const handleRestart = () => restartProcess(target).catch((e) => toast.error(`重启失败: ${e}`))

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">{label}</CardTitle>
        <div className="flex items-center gap-2">
          <span className={`inline-block h-2 w-2 rounded-full ${stateColor[state.state] ?? "bg-gray-400"}`} />
          <Badge variant="outline">{state.state}</Badge>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-1 text-xs text-muted-foreground">
          {state.pid && <div>PID: {state.pid}</div>}
          {state.uptimeSec != null && <div>运行时长: {state.uptimeSec}s</div>}
          {state.healthy != null && <div>健康: {state.healthy ? "正常" : "异常"}</div>}
          {state.exitCode != null && <div>退出码: {state.exitCode}</div>}
        </div>
        <div className="mt-3 flex gap-2">
          <Button size="sm" variant="outline" onClick={handleStart} disabled={isRunning || isBusy}>
            <Play className="mr-1 h-3 w-3" /> 启动
          </Button>
          <Button size="sm" variant="outline" onClick={handleStop} disabled={!isRunning || isBusy}>
            <Square className="mr-1 h-3 w-3" /> 停止
          </Button>
          <Button size="sm" variant="outline" onClick={handleRestart} disabled={isBusy}>
            <RotateCcw className="mr-1 h-3 w-3" /> 重启
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}
```

- [ ] **Step 2: Write LogView.tsx**

`src/components/LogView.tsx`:
```tsx
import { useEffect, useRef, useState } from "react"
import { Button } from "@/components/ui/button"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Pause, Play, Trash2, Download } from "lucide-react"
import { useTauriEvent } from "@/hooks/useTauriEvent"
import { getLogHistory, clearLogs, exportLogs } from "@/lib/tauri"
import type { LogEntry } from "@/lib/types"
import { toast } from "sonner"

export function LogView({ height = "400px" }: { height?: string }) {
  const [entries, setEntries] = useState<LogEntry[]>([])
  const [paused, setPaused] = useState(false)
  const [activeTab, setActiveTab] = useState<"all" | "server" | "bridge">("all")
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    getLogHistory("all", 500).then(setEntries).catch(() => {})
  }, [])

  useTauriEvent<LogEntry>("log://entry", (entry) => {
    if (paused) return
    setEntries((prev) => {
      const next = [...prev, entry]
      return next.length > 1000 ? next.slice(-1000) : next
    })
  })

  useEffect(() => {
    if (!paused) bottomRef.current?.scrollIntoView({ behavior: "smooth" })
  }, [entries, paused])

  const filtered = activeTab === "all" ? entries : entries.filter((e) => e.source === activeTab)

  const handleClear = () => {
    clearLogs(activeTab).then(() => {
      setEntries((prev) => activeTab === "all" ? [] : prev.filter((e) => e.source !== activeTab))
    }).catch(() => toast.error("清空失败"))
  }

  const handleExport = () => {
    exportLogs(activeTab).then((path) => toast.success(`已导出到: ${path}`)).catch(() => toast.error("导出失败"))
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as typeof activeTab)}>
          <TabsList>
            <TabsTrigger value="all">全部</TabsTrigger>
            <TabsTrigger value="server">Server</TabsTrigger>
            <TabsTrigger value="bridge">Bridge</TabsTrigger>
          </TabsList>
        </Tabs>
        <div className="flex gap-1">
          <Button size="sm" variant="ghost" onClick={() => setPaused((p) => !p)}>
            {paused ? <Play className="h-3 w-3" /> : <Pause className="h-3 w-3" />}
          </Button>
          <Button size="sm" variant="ghost" onClick={handleClear}><Trash2 className="h-3 w-3" /></Button>
          <Button size="sm" variant="ghost" onClick={handleExport}><Download className="h-3 w-3" /></Button>
        </div>
      </div>
      <div className={`overflow-auto rounded border bg-muted/30 p-2 font-mono text-xs`} style={{ height }}>
        {filtered.map((e, i) => (
          <div key={i} className={e.level === "error" ? "text-red-500" : "text-foreground"}>
            <span className="text-muted-foreground">[{new Date(e.ts * 1000).toLocaleTimeString()}]</span>{" "}
            <span className="text-muted-foreground">[{e.source}]</span>{" "}
            {e.line}
          </div>
        ))}
        <div ref={bottomRef} />
      </div>
    </div>
  )
}
```

- [ ] **Step 3: Verify type-check**

Run:
```bash
npx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/ProcessCard.tsx src/components/LogView.tsx
git commit -m "feat(frontend): add ProcessCard and LogView components"
```

---

## Task 15: WechatQrDialog and pages

**Files:**
- Create: `src/components/WechatQrDialog.tsx`
- Create: `src/pages/Dashboard.tsx`, `src/pages/Processes.tsx`, `src/pages/Config.tsx`, `src/pages/Bridge.tsx`, `src/pages/Channels.tsx`, `src/pages/Logs.tsx`
- Modify: `src/App.tsx` (layout + routing + dialog)

**Interfaces:**
- Consumes: all prior frontend modules.

- [ ] **Step 1: Write WechatQrDialog.tsx**

`src/components/WechatQrDialog.tsx`:
```tsx
import { useEffect, useState } from "react"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog"
import { useTauriEvent } from "@/hooks/useTauriEvent"
import QRCode from "qrcode"
import type { WechatQrEvent } from "@/lib/types"

export function WechatQrDialog() {
  const [open, setOpen] = useState(false)
  const [qrData, setQrData] = useState<WechatQrEvent | null>(null)
  const [qrUrl, setQrUrl] = useState<string>("")

  useTauriEvent<WechatQrEvent>("wechat://qrcode", (ev) => {
    setQrData(ev)
    setOpen(true)
  })

  useTauriEvent("wechat://logined", () => {
    setOpen(false)
    setQrData(null)
  })

  useEffect(() => {
    if (qrData?.kind === "url") {
      QRCode.toDataURL(qrData.data, { width: 256 }).then(setQrUrl).catch(() => {})
    } else {
      setQrUrl("")
    }
  }, [qrData])

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>微信扫码登录</DialogTitle>
          <DialogDescription>请使用微信扫描下方二维码完成登录</DialogDescription>
        </DialogHeader>
        <div className="flex justify-center p-4">
          {qrData?.kind === "url" && qrUrl ? (
            <img src={qrUrl} alt="QR Code" className="h-64 w-64" />
          ) : qrData?.kind === "ascii" ? (
            <pre className="font-mono text-[6px] leading-[6px] whitespace-pre">{qrData.data}</pre>
          ) : (
            <div className="h-64 w-64 animate-pulse bg-muted" />
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}
```

Install qrcode:
```bash
npm install qrcode @types/qrcode
```

- [ ] **Step 2: Write Dashboard.tsx**

`src/pages/Dashboard.tsx`:
```tsx
import { useEffect } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { ProcessCard } from "@/components/ProcessCard"
import { LogView } from "@/components/LogView"
import { useProcessState } from "@/hooks/useProcessState"
import { startAll, stopAll, restartAll } from "@/lib/tauri"
import { toast } from "sonner"

export function Dashboard() {
  const { state, refresh } = useProcessState()

  useEffect(() => { refresh() }, [refresh])

  return (
    <div className="space-y-4">
      <div className="flex gap-2">
        <Button onClick={() => startAll().catch((e) => toast.error(`启动失败: ${e}`))}>启动全部</Button>
        <Button variant="outline" onClick={() => stopAll().catch((e) => toast.error(`停止失败: ${e}`))}>停止全部</Button>
        <Button variant="outline" onClick={() => restartAll().catch((e) => toast.error(`重启失败: ${e}`))}>重启全部</Button>
      </div>
      <div className="grid grid-cols-2 gap-4">
        <ProcessCard target="server" state={state.server} />
        <ProcessCard target="bridge" state={state.bridge} />
      </div>
      <Card>
        <CardHeader><CardTitle className="text-sm">最近日志</CardTitle></CardHeader>
        <CardContent><LogView height="200px" /></CardContent>
      </Card>
    </div>
  )
}
```

- [ ] **Step 3: Write Processes.tsx**

`src/pages/Processes.tsx`:
```tsx
import { ProcessCard } from "@/components/ProcessCard"
import { useProcessState } from "@/hooks/useProcessState"

export function Processes() {
  const { state } = useProcessState()
  return (
    <div className="grid grid-cols-2 gap-4">
      <ProcessCard target="server" state={state.server} />
      <ProcessCard target="bridge" state={state.bridge} />
    </div>
  )
}
```

- [ ] **Step 4: Write Config.tsx**

`src/pages/Config.tsx`:
```tsx
import { useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { getConfig, saveConfig } from "@/lib/tauri"
import type { AppConfig } from "@/lib/types"
import { toast } from "sonner"

export function Config() {
  const [config, setConfig] = useState<AppConfig | null>(null)

  useEffect(() => { getConfig().then(setConfig).catch(() => toast.error("加载配置失败")) }, [])

  if (!config) return <div>加载中...</div>

  const save = () => saveConfig(config).then(() => toast.success("已保存")).catch(() => toast.error("保存失败"))

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader><CardTitle>opencode server</CardTitle></CardHeader>
        <CardContent className="space-y-3">
          <div className="space-y-1">
            <Label>端口</Label>
            <Input type="number" value={config.server.port}
              onChange={(e) => setConfig({ ...config, server: { ...config.server, port: Number(e.target.value) } })} />
          </div>
          <div className="space-y-1">
            <Label>opencodeServerUrl</Label>
            <Input value={config.server.opencodeServerUrl}
              onChange={(e) => setConfig({ ...config, server: { ...config.server, opencodeServerUrl: e.target.value } })} />
          </div>
          <div className="space-y-1">
            <Label>工作目录 (cwd)</Label>
            <Input value={config.server.cwd}
              onChange={(e) => setConfig({ ...config, server: { ...config.server, cwd: e.target.value } })} />
          </div>
        </CardContent>
      </Card>
      <Button onClick={save}>保存</Button>
    </div>
  )
}
```

- [ ] **Step 5: Write Bridge.tsx**

`src/pages/Bridge.tsx`:
```tsx
import { useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import { getConfig, saveConfig, checkBridgeUpdate, updateBridge, reinstallBridge, checkDeps } from "@/lib/tauri"
import type { AppConfig, DepStatus } from "@/lib/types"
import { toast } from "sonner"

export function Bridge() {
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [deps, setDeps] = useState<DepStatus | null>(null)

  useEffect(() => {
    getConfig().then(setConfig)
    checkDeps().then(setDeps)
  }, [])

  if (!config) return <div>加载中...</div>

  const save = () => saveConfig(config).then(() => toast.success("已保存")).catch(() => toast.error("保存失败"))

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader><CardTitle>依赖检测</CardTitle></CardHeader>
        <CardContent className="flex flex-wrap gap-2">
          {deps && Object.entries(deps).map(([k, v]) => (
            <Badge key={k} variant={v ? "default" : "destructive"}>{k}: {v ? "已安装" : "缺失"}</Badge>
          ))}
        </CardContent>
      </Card>
      <Card>
        <CardHeader><CardTitle>Bridge 配置</CardTitle></CardHeader>
        <CardContent className="space-y-3">
          <div className="space-y-1">
            <Label>安装路径（留空用默认）</Label>
            <Input value={config.bridge.installPath ?? ""}
              onChange={(e) => setConfig({ ...config, bridge: { ...config.bridge, installPath: e.target.value || null } })} />
          </div>
          <div className="space-y-1">
            <Label>defaultAgent</Label>
            <Input value={config.bridge.defaultAgent}
              onChange={(e) => setConfig({ ...config, bridge: { ...config.bridge, defaultAgent: e.target.value } })} />
          </div>
          <div className="space-y-1">
            <Label>dataDir</Label>
            <Input value={config.bridge.dataDir}
              onChange={(e) => setConfig({ ...config, bridge: { ...config.bridge, dataDir: e.target.value } })} />
          </div>
        </CardContent>
      </Card>
      <div className="flex gap-2">
        <Button variant="outline" onClick={() => checkBridgeUpdate().then((u) => toast.info(u ? "已是最新" : "有更新可用")).catch(() => toast.error("检查失败"))}>检查更新</Button>
        <Button variant="outline" onClick={() => updateBridge().then(() => toast.success("已更新")).catch(() => toast.error("更新失败"))}>更新</Button>
        <Button variant="outline" onClick={() => reinstallBridge().then(() => toast.success("已重装")).catch(() => toast.error("重装失败"))}>重新安装</Button>
      </div>
      <Button onClick={save}>保存</Button>
    </div>
  )
}
```

- [ ] **Step 6: Write Channels.tsx**

`src/pages/Channels.tsx`:
```tsx
import { useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { getConfig, saveConfig } from "@/lib/tauri"
import type { AppConfig } from "@/lib/types"
import { toast } from "sonner"

export function Channels() {
  const [config, setConfig] = useState<AppConfig | null>(null)
  useEffect(() => { getConfig().then(setConfig) }, [])
  if (!config) return <div>加载中...</div>

  const update = (channel: keyof AppConfig["channels"], patch: Partial<AppConfig["channels"][keyof AppConfig["channels"]]>) =>
    setConfig({ ...config, channels: { ...config.channels, [channel]: { ...config.channels[channel], ...patch } } })

  const save = () => saveConfig(config).then(() => toast.success("已保存")).catch(() => toast.error("保存失败"))

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>飞书</CardTitle>
          <Switch checked={config.channels.feishu.enabled} onCheckedChange={(v) => update("feishu", { enabled: v })} />
        </CardHeader>
        {config.channels.feishu.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>App ID</Label><Input value={config.channels.feishu.appId} onChange={(e) => update("feishu", { appId: e.target.value })} /></div>
            <div className="space-y-1"><Label>App Secret</Label><Input type="password" value={config.channels.feishu.appSecret} onChange={(e) => update("feishu", { appSecret: e.target.value })} /></div>
            <div className="space-y-1"><Label>Verification Token</Label><Input value={config.channels.feishu.verificationToken} onChange={(e) => update("feishu", { verificationToken: e.target.value })} /></div>
            <div className="space-y-1"><Label>Webhook Port</Label><Input type="number" value={config.channels.feishu.webhookPort} onChange={(e) => update("feishu", { webhookPort: Number(e.target.value) })} /></div>
            <div className="space-y-1"><Label>Encrypt Key</Label><Input value={config.channels.feishu.encryptKey} onChange={(e) => update("feishu", { encryptKey: e.target.value })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>QQ</CardTitle>
          <Switch checked={config.channels.qq.enabled} onCheckedChange={(v) => update("qq", { enabled: v })} />
        </CardHeader>
        {config.channels.qq.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>App ID</Label><Input value={config.channels.qq.appId} onChange={(e) => update("qq", { appId: e.target.value })} /></div>
            <div className="space-y-1"><Label>Secret</Label><Input type="password" value={config.channels.qq.secret} onChange={(e) => update("qq", { secret: e.target.value })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>Telegram</CardTitle>
          <Switch checked={config.channels.telegram.enabled} onCheckedChange={(v) => update("telegram", { enabled: v })} />
        </CardHeader>
        {config.channels.telegram.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>Bot Token</Label><Input type="password" value={config.channels.telegram.botToken} onChange={(e) => update("telegram", { botToken: e.target.value })} /></div>
            <div className="space-y-1"><Label>Allowed Chat IDs（逗号分隔）</Label><Input value={config.channels.telegram.allowedChatIds.join(",")} onChange={(e) => update("telegram", { allowedChatIds: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>Discord</CardTitle>
          <Switch checked={config.channels.discord.enabled} onCheckedChange={(v) => update("discord", { enabled: v })} />
        </CardHeader>
        {config.channels.discord.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>Bot Token</Label><Input type="password" value={config.channels.discord.botToken} onChange={(e) => update("discord", { botToken: e.target.value })} /></div>
            <div className="space-y-1"><Label>Allowed Channel IDs（逗号分隔）</Label><Input value={config.channels.discord.allowedChannelIds.join(",")} onChange={(e) => update("discord", { allowedChannelIds: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>微信</CardTitle>
          <Switch checked={config.channels.wechat.enabled} onCheckedChange={(v) => update("wechat", { enabled: v })} />
        </CardHeader>
        {config.channels.wechat.enabled && (
          <CardContent><p className="text-sm text-muted-foreground">微信使用扫码登录，启动后请在弹窗中扫码。</p></CardContent>
        )}
      </Card>

      <Button onClick={save}>保存</Button>
    </div>
  )
}
```

- [ ] **Step 7: Write Logs.tsx**

`src/pages/Logs.tsx`:
```tsx
import { LogView } from "@/components/LogView"

export function Logs() {
  return (
    <div className="space-y-2">
      <h2 className="text-lg font-semibold">日志</h2>
      <LogView height="600px" />
    </div>
  )
}
```

- [ ] **Step 8: Rewrite App.tsx with sidebar nav**

`src/App.tsx`:
```tsx
import { useState } from "react"
import { LayoutDashboard, Cpu, Settings, Boxes, Radio, ScrollText } from "lucide-react"
import { Dashboard } from "@/pages/Dashboard"
import { Processes } from "@/pages/Processes"
import { Config } from "@/pages/Config"
import { Bridge } from "@/pages/Bridge"
import { Channels } from "@/pages/Channels"
import { Logs } from "@/pages/Logs"
import { WechatQrDialog } from "@/components/WechatQrDialog"
import { Toaster } from "@/components/ui/sonner"
import { cn } from "@/lib/utils"

type Page = "dashboard" | "processes" | "config" | "bridge" | "channels" | "logs"

const navItems: { id: Page; label: string; icon: React.ReactNode }[] = [
  { id: "dashboard", label: "仪表盘", icon: <LayoutDashboard className="h-4 w-4" /> },
  { id: "processes", label: "进程", icon: <Cpu className="h-4 w-4" /> },
  { id: "config", label: "配置", icon: <Settings className="h-4 w-4" /> },
  { id: "bridge", label: "Bridge", icon: <Boxes className="h-4 w-4" /> },
  { id: "channels", label: "渠道", icon: <Radio className="h-4 w-4" /> },
  { id: "logs", label: "日志", icon: <ScrollText className="h-4 w-4" /> },
]

export default function App() {
  const [page, setPage] = useState<Page>("dashboard")

  return (
    <div className="flex h-screen">
      <nav className="w-16 border-r bg-muted/30 flex flex-col items-center py-4 gap-2">
        {navItems.map((item) => (
          <button key={item.id} onClick={() => setPage(item.id)}
            className={cn("flex flex-col items-center gap-1 rounded-md p-2 text-xs transition-colors w-14",
              page === item.id ? "bg-primary text-primary-foreground" : "hover:bg-muted text-muted-foreground")}>
            {item.icon}
            <span>{item.label}</span>
          </button>
        ))}
      </nav>
      <main className="flex-1 overflow-auto p-6">
        {page === "dashboard" && <Dashboard />}
        {page === "processes" && <Processes />}
        {page === "config" && <Config />}
        {page === "bridge" && <Bridge />}
        {page === "channels" && <Channels />}
        {page === "logs" && <Logs />}
      </main>
      <WechatQrDialog />
      <Toaster />
    </div>
  )
}
```

- [ ] **Step 9: Verify type-check and build**

Run:
```bash
npx tsc --noEmit && npm run build
```

Expected: no errors.

- [ ] **Step 10: Commit**

```bash
git add src/
git commit -m "feat(frontend): add all pages, WechatQrDialog, and app layout"
```

---

## Task 16: End-to-end manual verification

**Files:** none (verification only)

- [ ] **Step 1: Run dev mode**

Run:
```bash
npm run tauri dev
```

- [ ] **Step 2: Verify dashboard and config**

- Window opens, sidebar navigates between pages.
- Dashboard shows two process cards in Stopped state.
- Config page loads default config, editing port and saving persists (reopen shows saved value).
- Bridge page shows dep detection badges (bun/git should be true on this machine).
- Channels page: toggle Feishu on, fill appId/secret, save.

- [ ] **Step 3: Verify bridge install**

- On Bridge page click "重新安装" → should clone the repo to config dir.
- Check the path exists: `{config_dir}/bridges/opencode-im-bridge/.git`.

- [ ] **Step 4: Verify process start/stop**

- Click "启动全部" on Dashboard.
- Watch Logs page: server stdout should appear, then bridge stdout.
- Both cards should show Running with PID.
- Server card should show healthy: true within 5s.
- Click "停止全部" → both stop, exit codes shown.

- [ ] **Step 5: Verify tray**

- Close window → hides to tray (app still running).
- Click tray icon → window reappears.
- Right-click tray → "退出" → app quits after stopping processes.

- [ ] **Step 6: Commit final state**

```bash
git add -A
git commit -m "chore: e2e verification complete"
```

---

## Self-Audit Notes

**Spec coverage check:**
- §2 Architecture: Tasks 1-11 (Rust backend), 12-15 (frontend).
- §3 Config model: Task 3 (ConfigStore + types), Task 4 (renderer).
- §4 Monitoring: Task 5 (log buffer), Task 6 (stdout parser), Task 7 (health), Task 8 (supervisor).
- §5 UI: Tasks 13-15.
- §6 Tauri contract: Task 10 (commands), Task 12 (TS bindings).
- §7 Project structure: matches spec file table.
- §8 Error handling: Task 2 (AppError), Task 8 (crash → Failed state).
- §9 Testing: Tasks 4, 5, 6 have unit tests; Task 16 is manual E2E.
- §10 Dependencies: Cargo.toml (Task 1) + package.json (Tasks 1, 13, 15).
- §11 Open issues: WeChat QR format handled with two parser strategies; bridge start command fallback in Task 8.

**Type consistency:** `ProcessState`, `ProcessTarget`, `LogEntry`, `AppConfig` field names match across Rust (snake_case with serde camelCase) and TS (camelCase). `WechatQrEvent`/`QrKind` consistent. `DepStatus` fields lowercase match Rust struct fields.
