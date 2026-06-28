# OpenCodeDeck 设计规格

- **日期**: 2026-06-28
- **状态**: 已批准（待审查）
- **范围**: MVP 首版

## 1. 概述

OpenCodeDeck 是一个基于 Tauri v2 的跨平台桌面应用，用于管理 opencode server 与 opencode-im-bridge 两个子进程的生命周期、配置与监控。

### 1.1 目标

- 托管 `opencode serve` 子进程：启停、端口配置、状态监控。
- 托管 `opencode-im-bridge` 子进程：启停、渠道凭证配置、状态监控、微信扫码二维码展示。
- 提供系统托盘常驻入口，关闭主窗口缩到托盘。

### 1.2 非目标（MVP 不做）

- 监控面板的会话/模型概览、渠道连接状态详情（MVP 监控仅含：进程状态、日志流、server 健康检查）。
- 开机自启、崩溃自动重启。
- 多配置 profile 切换（单 profile）。
- 内置 bun/node 运行时（依赖系统已装）。
- 内置 bridge 源码副本（从 GitHub 拉取）。

### 1.3 平台支持

macOS、Windows、Linux 三平台。

### 1.4 运行时依赖

app 依赖系统已安装的运行时（启动时检测，缺失则禁用对应功能并提示）：

| 依赖 | 用途 | 检测方式 |
|------|------|----------|
| `opencode` | 启动 opencode server | `which opencode` |
| `bun`（优先）或 `node`+`npm` | 启动 bridge | `which bun`，失败回退 `which node` + `which npm` |
| `git` | 首次安装/更新 bridge | `which git` |

## 2. 整体架构

采用**双层进程管理**方案：Rust 后端负责所有子进程生命周期与配置生成，前端只发命令、收事件。

```
┌─────────────────────────────────────────────────┐
│  Tauri App (OpenCodeDeck)                       │
│  ┌───────────────────────────────────────────┐  │
│  │  Frontend (React + TS + Tailwind + shadcn)│  │
│  │  - 托盘菜单 / 主窗口                       │  │
│  │  - 配置编辑器 / 进程状态 / 日志流          │  │
│  │  - 微信二维码展示                          │  │
│  └──────────────┬────────────────────────────┘  │
│            Tauri Commands + Events              │
│  ┌──────────────┴────────────────────────────┐  │
│  │  Rust Backend                              │  │
│  │  ├─ ProcessManager (opencode serve 进程)   │  │
│  │  ├─ ProcessManager (bridge 子进程)         │  │
│  │  ├─ ConfigStore (app 配置目录读写)         │  │
│  │  ├─ ConfigRenderer (→ .env + jsonc 生成)   │  │
│  │  ├─ BridgeInstaller (git clone/更新)       │  │
│  │  ├─ StdoutParser (微信二维码/日志结构化)   │  │
│  │  └─ HealthChecker (轮询 server API)        │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
        │                              │
        ▼                              ▼
  opencode serve                  opencode-im-bridge
  (localhost:{port})              (bun run src/index.ts)
        │                              │
        └──────── HTTP API + SSE ──────┘
```

### 2.1 进程模型

**opencode server 进程**：
- 命令：`opencode serve --port {port}`
- 工作目录：`config.server.cwd`
- 由 Rust `tokio::process::Command` 托管，PID 持久化于内存状态。

**bridge 进程**：
- 启动前由 ConfigRenderer 把 app 配置渲染成 `.env` + `opencode-im.jsonc` 写到 bridge 工作目录。
- 启动命令按以下优先级回退（启动前 `check_deps` 检测结果决定用哪一档，选定后不再回退）：
  1. `bun run src/index.ts`（bun 存在时，首选）
  2. `npx tsx src/index.ts`（node 存在、bun 缺失时）
  - 若 node 也缺失，禁用 bridge 启动并提示。
- 工作目录：bridge 安装路径。
- 环境变量注入：`OPENCODE_SERVER_URL`、`OPENCODE_CWD`、各渠道凭证（也写入 .env，双保险）。

**启动顺序**：
1. 先拉起 server。
2. 健康检查（轮询 `GET {opencodeServerUrl}/session/status`）通过或超时仍无响应则标记 unhealthy。
3. 拉起 bridge（bridge 内部已有重连逻辑，仍按序启动更稳）。

**停止顺序**（反序）：
1. bridge 先 SIGTERM → 等待最多 5s → 仍存活则 SIGKILL。
2. server SIGTERM → 等待最多 5s → SIGKILL。

**bridge 安装与更新**：
- 默认安装路径：`{app_config_dir}/bridges/opencode-im-bridge/`。
- 首次启动 bridge 前，若该目录不存在或非 git 仓库，执行 `git clone https://github.com/ET06731/opencode-im-bridge {path}`。
- "检查更新"：在 bridge 目录执行 `git fetch` + 比较 `git rev-parse HEAD` 与 `git rev-parse origin/main`，不同则 `upToDate=false`。
- "更新"：`git pull --ff-only`。
- "重新安装"：删除目录后重新 clone。

## 3. 配置模型

### 3.1 App 配置目录

跨平台，用 `dirs` crate 解析：

| 平台 | 路径 |
|------|------|
| macOS | `~/Library/Application Support/OpenCodeDeck/` |
| Windows | `%APPDATA%\OpenCodeDeck\` |
| Linux | `~/.config/opencodedeck/` |

目录结构：
```
OpenCodeDeck/
├── config.json          # app 主配置
├── profiles/            # 预留多 profile（MVP 单 profile，未使用）
│   └── default.json
└── bridges/
    └── opencode-im-bridge/   # git clone 目标（默认在此）
```

### 3.2 config.json schema

```json
{
  "version": 1,
  "server": {
    "port": 4097,
    "opencodeServerUrl": "http://127.0.0.1:4097",
    "cwd": "/Users/gcy/code/some-project",
    "extraEnv": {}
  },
  "bridge": {
    "installPath": null,
    "defaultAgent": "build",
    "dataDir": "./data",
    "progress": { "debounceMs": 500, "maxDebounceMs": 3000 },
    "launcher": {
      "enabled": true,
      "autoStartServer": true,
      "serverCommand": "opencode serve",
      "serverStartTimeoutMs": 30000,
      "probeTimeoutMs": 4000
    }
  },
  "channels": {
    "feishu": {
      "enabled": false,
      "appId": "",
      "appSecret": "",
      "verificationToken": "",
      "webhookPort": 3001,
      "encryptKey": ""
    },
    "qq": { "enabled": false, "appId": "", "secret": "" },
    "telegram": { "enabled": false, "botToken": "", "allowedChatIds": [] },
    "discord": { "enabled": false, "botToken": "", "allowedChannelIds": [] },
    "wechat": { "enabled": false }
  }
}
```

字段说明：
- `server.port`：opencode serve 监听端口。
- `server.opencodeServerUrl`：bridge 连接 server 的 URL（默认 `http://127.0.0.1:{port}`）。
- `server.cwd`：会话发现目录（注入为 `OPENCODE_CWD`）。
- `server.extraEnv`：额外环境变量，启动 server 时注入。
- `bridge.installPath`：bridge 安装路径；`null` 则用默认 `{app_config_dir}/bridges/opencode-im-bridge/`。
- `bridge.defaultAgent`/`dataDir`/`progress`/`launcher`：对应 `opencode-im.jsonc` 字段。
- `channels.*.enabled`：渠道是否启用；仅 enabled 渠道的凭证写入 .env 与 jsonc。

凭证以明文存于 config.json（用户已确认接受此风险）。

### 3.3 ConfigRenderer 数据流

启动 bridge 前执行：

1. 读 `config.json` → 校验（必填项：enabled 渠道的凭证非空、端口在 1-65535）。
2. 渲染 `.env` 到 bridge 工作目录：
   - `FEISHU_APP_ID`、`FEISHU_APP_SECRET`、`FEISHU_VERIFICATION_TOKEN`、`FEISHU_ENCRYPT_KEY`（feishu enabled 时）
   - `QQ_APP_ID`、`QQ_SECRET`（qq enabled）
   - `TELEGRAM_BOT_TOKEN`、`TELEGRAM_ALLOWED_CHAT_IDS`（telegram enabled）
   - `DISCORD_BOT_TOKEN`、`DISCORD_ALLOWED_CHANNEL_IDS`（discord enabled）
   - `WECHAT_ENABLED=true`（wechat enabled）
   - `OPENCODE_SERVER_URL`、`OPENCODE_CWD`、`FEISHU_WEBHOOK_PORT`
3. 渲染 `opencode-im.jsonc` 到 bridge 工作目录：
   - `feishu` 段（feishu enabled）：appId/appSecret/verificationToken/webhookPort/encryptKey
   - `defaultAgent`、`dataDir`、`progress`、`launcher` 段
4. 启动 bridge 子进程，环境变量同时通过 `Command::env()` 注入（与 .env 双保险，因 bridge 的 `loadEnvFile` 会读 .env）。

### 3.4 与 bridge 现有配置的关系

- app 不读 bridge 自带的 `opencode-im.jsonc`/`.env`，启动时**覆盖写入**。
- 停止后文件保留（便于调试）。
- bridge 的 `setup wizard`/`pickConfig` 在 app 托管模式下被绕过：因工作目录下存在 app 写入的 `opencode-im.jsonc`，`pickConfig` 会直接选中，不触发 wizard。

## 4. 进程监控与日志

### 4.1 进程状态机

每个进程独立维护状态：

```
Stopped ──start──▶ Starting ──spawn ok──▶ Running
   ▲                   │                       │
   │                 fail/timeout              │ exit/crash
   │                   ▼                       │
   └────────────  Failed ◀─────────────────────┘
                  Stopping ──sigterm+wait──▶ Stopped
                              超时则 sigkill
```

状态字段：`state`（Stopped/Starting/Running/Stopping/Failed）、`pid`、`uptime`（Running 时）、`exitCode`（Failed/Stopped 时）、`startedAt`。

状态变更通过 Tauri Event `state://update` 推送前端。

### 4.2 日志流

- Rust 侧用 `tokio::process::Command` 拿到 stdout/stderr 的 `ChildStdout`/`ChildStderr`，按行读取。
- 每行封装为 `LogEntry { ts: i64, source: "server"|"bridge", level: "info"|"error", line: String }`：
  - stderr 行 → `level: "error"`
  - stdout 行 → `level: "info"`
- 通过 Tauri Event `log://entry` 推前端。
- Rust 侧维护**环形缓冲**（每个 source 默认 5000 行，用 `VecDeque` + 容量限制），前端打开日志页时一次性拉历史 + 订阅增量。
- 前端日志页：tabs（server/bridge/全部），自动滚动到底，暂停/清空/导出按钮（导出用 Tauri fs dialog 写文件）。

### 4.3 微信二维码解析

bridge 使用 `@wechatbot/wechatbot` 扫码登录，二维码以终端字符画（ASCII QR）或 URL 形式输出到 stdout。

Rust `StdoutParser` 维护特征匹配规则（MVP 两种）：

1. **ASCII QR 检测**：检测由块字符（`██`、`  `、`▀`、`▄`、`█` 等）组成的方阵行序列。识别到起始行后连续收集直到结束，透传原始文本块到前端，前端用等宽字体渲染。
2. **URL 检测**：匹配含 `login.weixin.qq.com`、`wx.qq.com` 等二维码登录 URL 的行，提取 URL，前端用 `qrcode` npm 库本地生成二维码图片。

检测到二维码时发 `wechat://qrcode` 事件，payload `{ kind: "ascii"|"url", data: String }`，前端弹窗展示。

登录成功：bridge 输出含 `login`/`logged in`/`登录成功` 等关键词的日志时，发 `wechat://logined` 事件，前端关闭弹窗。

**风险与对策**：bridge 微信渠道实际输出格式需实测确认。MVP 先实现上述两种解析；若实测格式不符，调整 StdoutParser 的匹配规则（属配置化调整，不改架构）。

### 4.4 健康检查

server 进程进入 Running 状态后，Rust 侧每 5s 轮询 `GET {opencodeServerUrl}/session/status`：
- 成功（HTTP 200）→ `healthy: true`
- 失败（连接拒绝/超时/非 200）→ `healthy: false`

状态通过 `health://update` 事件推送，并入仪表盘的 server 状态卡片展示。

bridge 进程不做 HTTP 健康检查（bridge 无标准健康端点），仅靠进程存活 + 日志判断。

## 5. UI 结构

### 5.1 主窗口

左侧导航 + 右侧内容区：

- **仪表盘**：server/bridge 两进程状态卡片（状态、PID、运行时长、健康、退出码）+ 一键启停按钮 + 最近 50 行日志预览。
- **进程页**：server/bridge 各一卡片，独立启停、重启、查看详情、跳转日志。
- **配置页**：编辑 `config.json` 的 server 段（端口、URL、cwd、extraEnv）。
- **Bridge 页**：installPath、defaultAgent、dataDir、progress、launcher 字段编辑；"检查更新"/"更新"/"重新安装"/"打开目录"按钮；依赖检测状态（bun/node/git 是否就绪）。
- **渠道页**：飞书/QQ/Telegram/Discord/微信 各一表单，enabled 开关 + 凭证字段，保存即写 config.json。
- **日志页**：tabs(server/bridge/全部) + 环形缓冲历史 + 暂停/清空/导出。
- **微信二维码弹窗**：收到 `wechat://qrcode` 事件时弹出，收到 `wechat://logined` 关闭。

### 5.2 系统托盘

- 图标随状态变色：全绿（两进程 Running）/ 部分红（有 Failed/Stopped）/ 全灰（全 Stopped）。
- 托盘菜单：
  - `[✓] opencode server`（状态指示）
  - `[✓] bridge`（状态指示）
  - `启动全部` / `停止全部` / `重启全部`
  - `显示主窗口`
  - `退出`（先停子进程再退出 app）
- 关闭主窗口 → 最小化到托盘（不退出）；托盘"退出"才真正退出。

## 6. Tauri 契约

### 6.1 Command（前端 → Rust）

| Command | 入参 | 返回 |
|---|---|---|
| `get_state` | - | `{ server: ProcessState, bridge: ProcessState }` |
| `start_process` | `{ target: "server"\|"bridge" }` | `Result<ProcessState, AppError>` |
| `stop_process` | `{ target }` | `Result<(), AppError>` |
| `restart_process` | `{ target }` | `Result<ProcessState, AppError>` |
| `start_all` | - | `Result<(), AppError>` |
| `stop_all` | - | `Result<(), AppError>` |
| `restart_all` | - | `Result<(), AppError>` |
| `get_config` | - | `Result<AppConfig, AppError>` |
| `save_config` | `{ config: AppConfig }` | `Result<(), AppError>` |
| `check_bridge_update` | - | `Result<{ upToDate: bool }, AppError>` |
| `update_bridge` | - | `Result<(), AppError>` |
| `reinstall_bridge` | - | `Result<(), AppError>` |
| `get_log_history` | `{ source: "server"\|"bridge"\|"all", limit: usize }` | `Result<Vec<LogEntry>, AppError>` |
| `export_logs` | `{ source }` | `Result<String, AppError>`（返回保存路径） |
| `check_deps` | - | `Result<DepStatus, AppError>`（opencode/bun/node/git 是否就绪） |

`ProcessState`：
```json
{ "state": "Stopped|Starting|Running|Stopping|Failed",
  "pid": 12345 | null,
  "startedAt": 1719561600 | null,
  "uptimeSec": 720 | null,
  "exitCode": 0 | null,
  "healthy": true | false | null }
```

`DepStatus`：
```json
{ "opencode": true, "bun": true, "node": true, "npm": true, "git": true }
```

### 6.2 Event（Rust → 前端）

| Event | Payload |
|---|---|
| `state://update` | `{ target: "server"\|"bridge", state: ProcessState }` |
| `log://entry` | `{ ts: i64, source: "server"\|"bridge", level: "info"\|"error", line: String }` |
| `wechat://qrcode` | `{ kind: "ascii"\|"url", data: String }` |
| `wechat://logined` | `{}` |
| `health://update` | `{ target: "server"\|"bridge", healthy: bool }` |

## 7. 项目结构

```
OpenCodeDeck/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs              # Tauri app 入口
│   │   ├── lib.rs
│   │   ├── commands.rs          # #[tauri::command] 定义
│   │   ├── state.rs             # AppState (Mutex<ProcessManager>, ConfigStore)
│   │   ├── process/
│   │   │   ├── mod.rs
│   │   │   ├── manager.rs       # ProcessManager: spawn/stop/restart, 状态机
│   │   │   └── supervisor.rs    # tokio task: 读 stdout/stderr, 推事件, 崩溃检测
│   │   ├── config/
│   │   │   ├── mod.rs
│   │   │   ├── store.rs         # ConfigStore: 读写 config.json
│   │   │   └── renderer.rs      # ConfigRenderer: → .env + opencode-im.jsonc
│   │   ├── bridge/
│   │   │   ├── mod.rs
│   │   │   ├── installer.rs     # git clone/pull, 路径解析
│   │   │   └── env_check.rs     # 检测 bun/node/opencode/git
│   │   ├── monitor/
│   │   │   ├── mod.rs
│   │   │   ├── log_buffer.rs    # 环形缓冲 (VecDeque<LogEntry>)
│   │   │   ├── stdout_parser.rs # 微信二维码/日志解析
│   │   │   └── health.rs        # 健康检查轮询
│   │   └── error.rs             # AppError + serde::Serialize
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── icons/
├── src/                         # 前端 React
│   ├── main.tsx
│   ├── App.tsx
│   ├── pages/
│   │   ├── Dashboard.tsx
│   │   ├── Processes.tsx
│   │   ├── Config.tsx
│   │   ├── Bridge.tsx
│   │   ├── Channels.tsx
│   │   └── Logs.tsx
│   ├── components/
│   │   ├── ui/                  # shadcn 组件
│   │   ├── ProcessCard.tsx
│   │   ├── LogView.tsx
│   │   └── WechatQrDialog.tsx
│   ├── hooks/
│   │   ├── useTauriEvent.ts
│   │   └── useProcessState.ts
│   ├── lib/
│   │   ├── tauri.ts             # invoke 封装
│   │   └── types.ts             # 与 Rust 对应的 TS 类型
│   └── styles/
├── package.json
├── vite.config.ts
├── tailwind.config.ts
└── tsconfig.json
```

## 8. 错误处理

### 8.1 AppError

Rust 侧统一 `AppError` 枚举，实现 `serde::Serialize`，通过 `Result<T, AppError>` 返回前端，前端 toast 展示：

```rust
#[derive(Debug, thiserror::Error, serde::Serialize)]
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
```

### 8.2 子进程崩溃

supervisor tokio task 监测到子进程非预期退出：
- 状态置 `Failed` + 记录 `exitCode`。
- 推 `state://update` 事件。
- MVP **不做自动重启**，前端提供"重启"按钮由用户手动触发。

### 8.3 启动失败

- spawn 失败（如命令不存在）：状态置 `Failed`，返回 `AppError::Process`。
- bridge 启动前 ConfigRenderer 校验失败：返回 `AppError::Config`，不启动进程。
- bridge 未安装且未执行安装：返回 `AppError::BridgeInstall`，前端引导安装。

## 9. 测试策略

### 9.1 Rust 单元测试

- `config::renderer`：给定 AppConfig，断言渲染出的 .env 与 jsonc 内容正确（含 enabled 渠道、不含 disabled 渠道）。
- `monitor::stdout_parser`：构造 ASCII QR 与 URL 两类输入，断言正确识别。
- `monitor::log_buffer`：写入超容量行，断言保留最新 N 行。
- `bridge::env_check`：mock PATH，断言检测逻辑（用临时目录放假可执行文件）。

### 9.2 Rust 集成测试

- `process::manager`：用 `echo`/`sleep` 等假进程测启动→Running、停止→Stopped、kill→Failed 状态机流转与事件。

### 9.3 前端测试

- 组件用 Vitest + Testing Library。
- hooks 用 `@tauri-apps/api/mocks` mock invoke 与 event。

### 9.4 E2E

MVP 不做（Tauri E2E 门槛高），靠手动验证跨平台关键路径：
- 安装 bridge → 配置渠道 → 启动 server → 启动 bridge → 日志流 → 停止。
- 微信扫码二维码展示（需真实微信渠道）。

## 10. 依赖清单

### 10.1 Rust（Cargo.toml）

- `tauri` v2（含 `tray-icon` feature）
- `tokio`（full）
- `serde`、`serde_json`
- `dirs`（配置目录解析）
- `thiserror`
- `reqwest`（健康检查，blocking 或 async）
- `ringbuf` 或手写 `VecDeque`（日志环形缓冲）

### 10.2 前端（package.json）

- `@tauri-apps/api` v2
- `react`、`react-dom`
- `typescript`
- `vite`、`@vitejs/plugin-react`
- `tailwindcss`、`postcss`、`autoprefixer`
- `shadcn/ui`（含 `radix-ui` 依赖）
- `lucide-react`（图标）
- `qrcode`（URL→二维码图片，微信扫码用）
- `vitest`、`@testing-library/react`（测试）

## 11. 开放问题

1. **微信扫码实测**：bridge 微信渠道 stdout 实际输出格式需在实现期实测，可能需调整 StdoutParser 匹配规则。
2. **bridge 启动命令跨平台一致性**：`bun run src/index.ts` 与 `npx tsx src/index.ts` 在三平台行为应一致，需验证 Windows 上 bun/tsx 的可用性。
3. **opencode 命令跨平台**：`opencode serve` 在 Windows 上的可执行文件名/路径需确认（可能是 `opencode.exe` 或需通过 PATH）。
