# OpenCodeDeck 稳健性提升设计

- **日期**: 2026-06-28
- **状态**: 已批准（待审查）
- **范围**: 对现有 MVP 代码的系统性稳健性修复，不新增功能

## 1. 背景

OpenCodeDeck MVP 已实现，但代码审查发现多类稳健性问题：进程状态机竞态、panic 风险、配置非原子写入、前端事件监听泄漏、错误展示不可读等。本设计按严重性分级（P0/P1/P2）系统性修复，不改变现有功能行为，只提升健壮性。

## 2. 修复清单

### P0 — 会导致错误行为或崩溃

| # | 问题 | 位置 |
|---|------|------|
| 1 | supervisor 覆盖 Stopped→Failed：用户主动 stop_async 后 supervisor 仍把状态设为 Failed | `process/supervisor.rs:83-94` |
| 2 | restart bridge 不重写配置：restart_async 对 bridge 只 stop+start，不调 write_bridge_files | `process/manager.rs:254-264` |
| 3 | 前端事件监听泄漏：useTauriEvent 的 listen() 异步，effect 在 resolve 前 cleanup 时 unlisten 仍为 undefined | `hooks/useTauriEvent.ts:8-14` |
| 4 | expect("no child") panic：supervisor 取 child 时若已被 take（竞态）直接 panic | `process/supervisor.rs:56,106` |
| 5 | Windows 找不到 opencode/bun/npx：Command::new 不自动查找 .cmd/.bat/.ps1 扩展名 | `process/manager.rs:105,155,159` |
| 6 | lock().unwrap() 传播 panic：任一处持有锁时 panic 会 poison Mutex | 全局 |

### P1 — 用户体验/数据健壮性

| # | 问题 | 位置 |
|---|------|------|
| 7 | 配置非原子写入：fs::write 写一半崩溃/断电 → config.json 损坏 | `config/store.rs:220-225` |
| 8 | 配置加载无容错：config.json 损坏直接报错，不备份不恢复 | `config/store.rs:208-218` |
| 9 | toast 显示 [object Object]：e 是 {kind,message} 对象，toString() 不可读 | `pages/Dashboard.tsx:15-17` 等 |
| 10 | refresh 静默吞错：.catch(() => {}) 隐藏所有状态刷新失败 | `hooks/useProcessState.tsx:23` |
| 11 | read_stream 静默吞 IO 错误：while let Ok(...) 遇 Err 直接退出循环，不记录 | `process/supervisor.rs:24` |
| 12 | 托盘 start_all 同步阻塞主线程：与 stop_all/restart_all 不一致 | `lib.rs:103-106` |
| 13 | git clone 阻塞命令线程：bridge install 是同步 Command::status() | `bridge/installer.rs`, `commands.rs` |
| 14 | quit 阻塞主线程：block_on + 5s 超时 ×2，最多卡 10s | `lib.rs:97-101` |
| 15 | 无 React Error Boundary：任意组件抛错 → 白屏 | `src/main.tsx` |

### P2 — 架构/一致性

| # | 问题 | 位置 |
|---|------|------|
| 16 | 健康检查用 std::thread+sleep 而非已有 tokio runtime | `lib.rs:58-71` |
| 17 | restart_async 内 ConfigStore::new() 而非复用 AppState 的（实现时机随 #2） | `manager.rs:259` |
| 18 | 健康检查每轮 load_config()，配置损坏时静默回退默认值 | `lib.rs:64` |

## 3. 详细设计

### 3.1 P0 修复

#### 3.1.1 supervisor 不覆盖主动停止的状态（#1, #4）

**当前**：`stop_async` take 了 child，supervisor 的 `child.take()` 返回 `None`，但仍写 `Failed`。`mp.child.as_mut().expect("no child")` 在竞态下 panic。

**方案**：
- `ManagedProcess` 增加 `stopping: bool` 字段（默认 false）。
- `stop_async` 在置 `Stopping` 状态时设 `stopping = true`。
- supervisor 退出处理改为：
  ```rust
  let child = { mp.child.take() }; // 不再 expect
  // 取 child 为 None 说明被 stop_async take 走，或已退出
  let exit_code = match child {
      Some(mut c) => c.wait().await.ok().and_then(|s| s.code()),
      None => None,
  };
  let next_state = if mp.stopping {
      mp.stopping = false;
      ProcessStateKind::Stopped
  } else {
      ProcessStateKind::Failed
  };
  mp.state = ProcessState { state: next_state, ... };
  ```
- `stop_async` 成功停止后也置 `stopping = false`（双重保险）。

**测试**：新增集成测试 `stop_then_supervisor_exits_marks_stopped`——启动短命进程，调 stop_async，确认最终状态为 Stopped 而非 Failed。

#### 3.1.2 restart bridge 重写配置（#2）

**当前**：`restart_async` 的 Bridge 分支只调 `start_bridge`，不写配置文件。

**方案**：`restart_async` 的 Bridge 分支改为与 `commands::start_process` 一致——先 `renderer::write_bridge_files(&cfg, bridge_dir)`，再 `start_bridge`。

由于 #17 会改 `restart_async` 签名（接收 `bridge_dir`），此处一并调整：`restart_async` 接收 `bridge_dir: &Path` 参数，调用方负责计算。

#### 3.1.3 前端事件监听不泄漏（#3）

**当前**：
```ts
useEffect(() => {
  let unlisten: UnlistenFn | undefined
  listen<T>(event, ...).then((fn) => { unlisten = fn })
  return () => { unlisten?.() }
}, [event])
```
effect 在 `listen()` resolve 前 cleanup 时 `unlisten` 仍为 undefined，监听永远不取消。

**方案**：
```ts
useEffect(() => {
  let unlisten: UnlistenFn | undefined
  let cancelled = false
  listen<T>(event, (e) => handlerRef.current(e.payload)).then((fn) => {
    if (cancelled) { fn() } // 已 cleanup，立即取消
    else { unlisten = fn }
  })
  return () => {
    cancelled = true
    unlisten?.()
  }
}, [event])
```

#### 3.1.4 Windows 命令查找（#5）

**当前**：`Command::new("opencode")` 在 Windows 不解析 `.cmd`/`.bat`/`.exe` 扩展名（`opencode`、`bun` 常以 `.cmd` 安装）。

**方案**：新增 `which` crate（跨平台 PATH 解析，~5KB，无依赖）到 Cargo.toml。封装工具函数：
```rust
fn resolve_command(name: &str) -> AppResult<std::process::Command> {
    let path = which::which(name)
        .map_err(|_| AppError::EnvNotFound(name.to_string()))?;
    Ok(std::process::Command::new(path))
}
```
`start_server`、`start_bridge`、`env_check::which`、`installer` 的 git 命令统一改用此函数。`env_check` 的 `which` 函数也改用 `which::which` 实现（删除手写的 `which`/`where` 分支）。

#### 3.1.5 消除 lock().unwrap() panic（#6）

**当前**：全局 `lock().unwrap()`，poison 锁会传播 panic。

**方案**：在 `process/mod.rs` 加辅助 trait/方法：
```rust
pub(crate) fn lock_or_recover<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}
```
全局 `lock().unwrap()` 替换为 `lock_or_recover(...)`（约 12 处：manager.rs、supervisor.rs、lib.rs、commands.rs 的 log_buffer 锁）。

### 3.2 P1 修复

#### 3.2.1 配置原子写入 + 容错（#7, #8）

**save**：写临时文件 `config.json.tmp`（同目录，保证同文件系统），`fs::rename` 原子替换 `config.json`。

**load**：解析失败时：
1. 备份损坏文件为 `config.json.corrupt-{unix_ts}`。
2. 写入默认配置。
3. 返回默认值。

```rust
pub fn load(&self) -> AppResult<AppConfig> {
    let path = self.config_path();
    if !path.exists() { /* 写默认 */ return Ok(default); }
    let content = std::fs::read_to_string(&path)?;
    match serde_json::from_str::<AppConfig>(&content) {
        Ok(cfg) => Ok(cfg),
        Err(e) => {
            // 备份损坏文件
            let backup = path.with_extension(format!("json.corrupt-{}", now_ts()));
            let _ = std::fs::rename(&path, &backup);
            let cfg = Self::default_config();
            let _ = self.save(&cfg);
            Ok(cfg)
        }
    }
}
```

#### 3.2.2 toast 显示可读错误（#9）

**方案**：在 `src/lib/utils.ts` 加 `formatError`：
```ts
export function formatError(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) {
    return String((e as { message: unknown }).message)
  }
  return String(e)
}
```
全局 `.catch((e) => toast.error(\`...: ${formatError(e)}\`))` 替换。涉及：Dashboard、Processes、Bridge、Channels、Config。

#### 3.2.3 refresh 不静默吞错（#10）

`useProcessState` 的 refresh catch 改为 `console.error("[refresh]", e)`，不 toast（避免刷屏），保留不抛错。

#### 3.2.4 read_stream 记录 IO 错误（#11）

**当前**：`while let Ok(Some(line)) = lines.next_line().await` 遇 Err 静默退出。

**方案**：
```rust
loop {
    match lines.next_line().await {
        Ok(Some(line)) => on_log(LogEntry { ... line }),
        Ok(None) => break,
        Err(e) => {
            on_log(LogEntry { ts: now_ts(), source: source.clone(), level: "error",
                line: format!("stream read error: {}", e) });
            break;
        }
    }
}
```

#### 3.2.5 托盘 start_all 改异步（#12）

`lib.rs` 的 `start_all` 菜单事件改用 `tauri::async_runtime::spawn`（与 stop_all/restart_all 一致）。`do_start_all` 改 `async fn`（因 #13 git 操作改异步，见下）。

#### 3.2.6 bridge install 不阻塞（#13）

**当前**：`start_process`/`start_all` 是同步 `#[tauri::command]`，内部 `installer.install()?` 调同步 `Command::status()`。

**方案**：
- `BridgeInstaller` 的 `install`/`update`/`reinstall`/`check_update` 改用 `tokio::process::Command`，签名改 `async fn`。
- `start_process`/`start_all` 改 `async fn`（Tauri 支持 async command）。
- `do_start_all` 改 `async fn`。
- 托盘 `start_all` 菜单事件改 `tauri::async_runtime::spawn`。
- `restart_process`/`restart_all` 已是 async，调用链自然适配。

#### 3.2.7 quit 不阻塞主线程（#14）

**当前**：`lib.rs:97-101` 同步 `stop` ×2（各最多 5s），主线程最多卡 10s。

**方案**：托盘"退出"改 `tauri::async_runtime::spawn` 异步停两进程，完成后 `app.exit(0)`：
```rust
"quit" => {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = handle.state::<state::AppState>();
        let _ = state.process_manager.stop_async(Bridge).await;
        let _ = state.process_manager.stop_async(Server).await;
        handle.exit(0);
    });
}
```

#### 3.2.8 React Error Boundary（#15）

**方案**：新增 `src/components/ErrorBoundary.tsx`（class 组件，捕获 render 错误），`main.tsx` 中包裹 `<App/>`：
```tsx
<ErrorBoundary>
  <App/>
</ErrorBoundary>
```
ErrorBoundary 渲染："出错了，请重启应用"+「重置」按钮（`setState({ hasError: false })` 重新渲染）。

### 3.3 P2 修复

#### 3.3.1 健康检查用 tokio（#16）

**当前**：`lib.rs:58-71` 用 `std::thread::spawn` + `sleep`，`HealthChecker` 用 `reqwest::blocking`。

**方案**：
- `HealthChecker::check_once` 改 `async fn`，`reqwest`（async）替代 `reqwest::blocking`。
- Cargo.toml `reqwest` 去掉 `blocking` feature，加默认 async。
- `lib.rs` 健康检查改 `tauri::async_runtime::spawn` + `tokio::time::interval(Duration::from_secs(5))`。
- `reqwest::Client::builder().timeout(...).build()` 复用 client（每轮 check 不新建）。

#### 3.3.2 restart_async 复用 ConfigStore（#17）

**当前**：`manager.rs:259` `ConfigStore::new()` 重新构造。

**方案**：`restart_async` 签名改为接收 `bridge_dir: &Path`（调用方从 `state.config_store.bridge_install_path` 算好传入）。`ProcessManager` 不再依赖 `ConfigStore`。`commands::restart_process` 已有 `state`，统一从这里取。

#### 3.3.3 健康检查不每轮 load_config（#18）

**当前**：每 5s `load_config()`，配置损坏时静默回退默认值（#8 修复后仍有回退开销）。

**方案**：健康检查任务持有 `Arc<AtomicU64>` 版本号。`save_config` 成功后递增版本号。检查任务每轮比对版本号——变化时才调 `state.load_config()` 更新 `opencode_server_url`。

实现：
- `AppState` 增加 `config_version: Arc<std::sync::atomic::AtomicU64>`。
- `save_config` 成功后 `config_version.fetch_add(1, ...)`。
- 健康检查任务闭包捕获 `config_version` 克隆，维护本地 `last_version`，每轮比对。

## 4. 实现顺序

按依赖关系分组实现：

1. **P0 基础设施**：#6（lock 辅助）、#5（which crate + resolve_command）——后续修复依赖这些。
2. **P0 状态机**：#1+#4（supervisor stopping 标志 + 不 panic）、#2（restart 重写配置，含 #17 签名调整）。
3. **P0 前端**：#3（useTauriEvent 不泄漏）。
4. **P1 配置**：#7+#8（原子写 + 容错）。
5. **P1 错误展示**：#9（formatError）、#10（refresh 不吞错）、#11（read_stream 记录错）。
6. **P1 异步化**：#13（bridge install 异步，牵动 start_process/start_all 改 async）、#12（托盘 start_all 异步）、#14（quit 异步）。
7. **P1 前端**：#15（ErrorBoundary）。
8. **P2**：#16（健康检查 tokio）、#18（配置版本号）。#17 已在 #2 时一并完成。

## 5. 测试策略

### 5.1 Rust 单元测试

- **supervisor stopping 标志**：`stop_async` 后状态为 Stopped 而非 Failed（用短命假进程）。
- **restart 重写配置**：mock 或断言 `write_bridge_files` 被调用（可改为返回 Path 验证文件存在）。
- **配置原子写入**：`save` 后 `config.json.tmp` 不残留。
- **配置容错**：写损坏 JSON 到 config path，`load()` 返回默认值且备份文件存在。

### 5.2 前端测试

当前项目无前端测试框架（package.json 无 vitest）。本设计不新增测试框架（YAGNI，MVP 阶段）。useTauriEvent、formatError 的正确性靠代码审查 + 手动验证。

### 5.3 验证命令

- `cargo build`（src-tauri）——编译通过。
- `cargo test`（src-tauri）——单元测试通过。
- `cargo clippy`（src-tauri）——无 warning。
- `npm run build`（前端）——tsc + vite build 通过。

## 6. 非目标

- 不新增功能（崩溃自动重启、开机自启等仍不做）。
- 不重构整体架构（只针对性修复）。
- 不引入前端测试框架。
- 不改变 Tauri Command/Event 契约（仅内部实现变化，前端 invoke 调用不变）。
