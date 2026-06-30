# AGENTS.md

Tauri 2 desktop app (React 19 + Vite + Rust) that launches and supervises two child processes: `opencode serve` and `opencode-im-bridge`.

## Commands

| Task | Command | Notes |
|------|---------|-------|
| Dev (full app) | `npm run tauri dev` | Starts Vite on `:1420` (strictPort) then launches Tauri. |
| Frontend build / typecheck | `npm run build` | Runs `tsc` then `vite build`. `tsc` is the only typecheck. |
| Fast typecheck only | `npx tsc --noEmit` | `tsconfig.json` already has `noEmit: true`. |
| Build macOS | `npm run tauri build` (or `build:mac` for dmg) | `beforeBuildCommand` runs `npm run build` automatically. |
| Build Linux | `npm run build:linux` | Bundles `deb`. |
| Build Windows | `npm run build:windows` | Bundles `nsis`. |
| Rust checks | `cargo check` (in `src-tauri/`) | |
| Rust tests | `cargo test` (in `src-tauri/`) | Unit tests live inline as `#[cfg(test)] mod tests` (e.g. `env_path.rs`). |

There is **no lint script** and **no frontend test runner** configured. Do not assume vitest/jest exists. For verification: `npx tsc --noEmit` for TS, `cargo test` for Rust.

## Architecture

- **Frontend entry**: `src/main.tsx` → `src/App.tsx`. Pages in `src/pages/` (Providers, Processes, Bridge, Logs, Config, Channels).
- **Frontend→Rust IPC**: all `invoke()` wrappers centralized in `src/lib/tauri.ts`. Add new commands there.
- **Rust entry**: `src-tauri/src/lib.rs::run()` is the app bootstrap. Modules: `process` (supervisor/manager), `bridge` (installer + env check), `config` (store + renderer), `monitor` (log buffer, stdout parser, health), `opencode_config`, `env_path`, `commands`, `state`, `error`.
- **Tauri events use `://` as separator** (e.g. `state://update`, `log://entry`, `health://update`, `wechat://qrcode`, `wechat://logined`). This is intentional, not a typo — match it when emitting/listening.
- **Runtime PATH augmentation**: `env_path::augment_path()` runs at startup (`lib.rs`) to add homebrew/nvm/snap/bun/cargo dirs so the GUI app can find `opencode`, `bun`, `git`. Per-platform lists in `src-tauri/src/env_path.rs`. If a CLI "not found" issue arises, check whether its install dir is listed there.
- **Health check loop**: polls each running server every 5s (`lib.rs` setup task); reconfigures checkers when the config version bumps.
- **Window close → hide to tray**: close is intercepted via `prevent_close` + `hide()`; real exit goes through the tray "quit" menu which stops all processes first.

## Conventions

- **Path alias**: `@/*` → `./src/*` (configured in both `tsconfig.json` and `vite.config.ts`). Use `@/...` imports.
- **shadcn/ui** (new-york style, neutral base, lucide icons). UI primitives live in `src/components/ui/`. Aliases in `components.json` point to `@/components`, `@/lib/utils`, `@/hooks`.
- **Tailwind v4 hybrid**: `src/styles/globals.css` uses `@import "tailwindcss"` (v4) but loads the v3-style `tailwind.config.ts` via the `@config "../../tailwind.config.ts"` directive. Do **not** delete `tailwind.config.ts` or "migrate" to pure-v4 config without reconciling the `@config` reference — theme tokens (CSS vars in `globals.css`) and the JS config are both in use.
- **Dark mode**: class-based (`darkMode: ["class"]`), toggled via `next-themes`.
- **Rust**: edition 2021, crate `opencodedeck_lib` (`crate-type` includes `rlib` for `cargo test`). Errors flow through `error::AppResult`/`AppError`; commands return `AppResult<T>`.
- **Tauri commands** use `#[serde(rename_all = "camelCase")]` on response structs so Rust snake_case serializes to JS camelCase — keep this when adding new DTOs.
- **Platform-specific code** is split under `src-tauri/src/process/platform/{unix,windows}.rs`; `env_path.rs` also has per-platform path lists. README claims macOS + Linux, but Windows is supported too (build script + platform module exist).

## External runtime dependencies

The app shells out to CLIs that must be on PATH at runtime (not build time): `opencode`, `bun` (runs the bridge), `git` (bridge auto-install/update). The bridge (`opencode-im-bridge`) is git-cloned on first launch into the configured install path.
