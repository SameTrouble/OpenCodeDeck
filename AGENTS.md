# AGENTS.md

Tauri 2 桌面应用（React 19 + Vite + Rust），启动并监管两个子进程：`opencode serve`（服务器）和 `opencode-im-bridge`（IM 桥接）。支持 macOS 与 Linux —— `env_path.rs` 按平台组合 PATH 候选目录（macOS: homebrew + nvm + bun + opencode；Linux: snap + nvm + bun + opencode + ~/.local/bin）。

## 命令

- `npm run tauri dev` —— 完整应用开发（在 1420 端口运行 `npm run dev`，然后启动 Tauri）。除非只改前端，否则用这个，不要单独用 `npm run dev`。
- `npm run build` —— `tsc && vite build`。**这是唯一的类型检查**；没有 `lint`/`typecheck`/`test` npm 脚本。未配置 ESLint/Prettier。
- `cargo test` —— 运行 Rust 测试（在 `src-tauri/` 内执行）。
- 不存在前端测试。
- Linux 本地构建需传 `--bundles appimage`（`npm run tauri build -- --bundles appimage`），否则 `tauri.conf.json` 的 `"targets": "all"` 会尝试 deb/rpm/appimage 全部格式而可能失败（CI 已用此参数绕开）。

Vite 端口 1420 为 `strictPort: true`，且 Tauri 的 `devUrl` 依赖它 —— 不要修改。

## 架构

前端↔后端桥接是单文件：`src/lib/tauri.ts` 包装了每个 `invoke<T>()`。类型定义在 `src/lib/types.ts`（TS），**必须手动保持同步**于 `src-tauri/src/config/store.rs` 和 `commands.rs` 中的 Rust 结构体。所有 Rust 跨边界结构体使用 `#[serde(rename_all = "camelCase")]` —— 新增命令时需保持此约定。

Tauri 事件（由 Rust 发射，TS 通过 `useTauriEvent` 监听）：`state://update`、`log://entry`、`health://update`、`wechat://qrcode`、`wechat://logined`。名称使用 `://` 后缀。

后端布局（`src-tauri/src/`）：
- `lib.rs` —— 应用初始化、托盘图标、健康检查循环（5s 间隔）、注册所有 `invoke_handler` 命令。
- `process/` —— `ProcessManager` 生成/监管子进程；`supervisor.rs` 读取 stdout/stderr 并检测退出（用 `stopping` 标志区分 Stopped 与 Failed）。`command_util.rs::resolve_command` 用 `which` 解析 `opencode`/`bun`/`git` 等。
- `bridge/installer.rs` —— 首次启动时 git clone `opencode-im-bridge` 到 `<config_dir>/bridges/opencode-im-bridge`；`update`/`reinstall` 通过 git pull/rmtree。
- `config/renderer.rs` —— 每次启动/重启时根据 `AppConfig` 重写 bridge 的 `.env` 和 `opencode-im.jsonc`。
- `env_path.rs` —— 启动时增强 `PATH`，让 GUI 应用能找到 `opencode`、`bun`、`node`、`git`（homebrew/nvm 目录）。修改它会破坏打包应用的依赖发现。
- `monitor/` —— `LogBuffer`（每个来源 5000 条环形缓冲）、`health.rs`（`GET <server_url>/session/status`）、`stdout_parser.rs`（检测微信二维码为 ASCII 块或 URL）。

## 约定 / 陷阱

- `renderer.rs` 中的 `derive_server_url` **用 `config.server.port` 覆盖配置 URL 的端口**。这是有意的 —— 保证 bridge 和启动的服务器指向同一实例。不要"修复"这个不一致。
- `@/*` 路径别名 → `./src/*`（`tsconfig.json` 和 `vite.config.ts` 均如此）。
- TS 为 `strict` 且开启 `noUnusedLocals`/`noUnusedParameters` —— 未使用的导入/变量会导致 `npm run build` 失败。
- shadcn/ui 组件位于 `src/components/ui`（样式："new-york"，图标库：lucide）。新增组件用 shadcn CLI，不要手写。
- 关闭主窗口只是隐藏（托盘行为）。仅通过托盘菜单 → `quit` 退出，会先停止两个进程。
- `ConfigStore::load` 会将损坏的 `config.json` 备份为 `config.json.corrupt-<ts>` 并重写默认值 —— 依赖此机制，不要额外加 fallback 路径。
- 部分 `cargo test` 测试调用 `std::mem::forget(pm)` 跳过运行时清理 —— 有意为之，不是要修复的泄漏。
- `AppError` 为 `#[serde(tag = "kind", content = "message")]`；`kind` 字符串（如 `"Process"`、`"EnvNotFound"`）是 TS `AppError` 联合类型 switch 的依据。

## CI / GitHub Actions

工作流定义在 `.github/workflows/build.yml`，覆盖 ubuntu-24.04(x64)、macos-13(x64)、macos-14(arm64)。

- **push/PR** → 仅构建验证（3 个 job 并行）。
- **push `v*` tag** → 构建 + macOS 签名公证 + 上传到 GitHub Release。
- Ubuntu 产出 `*.AppImage`，macOS 产出 `*.dmg`。

### macOS 签名 Secrets

仅在 tag 触发时注入（push/PR 不签名）。未配置时 macOS 仍能构建，产出未签名 `.dmg`。

| Secret | 用途 |
|--------|------|
| `APPLE_CERTIFICATE` | Developer ID Application `.p12` 的 base64 |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` 导出密码 |
| `APPLE_ID` | Apple ID 邮箱（公证用） |
| `APPLE_PASSWORD` | App 专用密码（appleid.apple.com 生成） |
| `APPLE_TEAM_ID` | 10 位团队 ID |
| `APPLE_SIGNING_IDENTITY` | 证书 CN，如 `Developer ID Application: Name (TEAMID)` |

### 权限要求

仓库 Settings → Actions → General → Workflow permissions 需设为 **Read and write permissions**（`GITHUB_TOKEN` 需要 `contents: write` 来创建 Release）。
