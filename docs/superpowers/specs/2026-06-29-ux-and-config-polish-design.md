# 体验与配置编辑优化设计

- 日期：2026-06-29
- 范围：前端为主，后端最小改动（LogEntry 加 seq）
- 目标：日志渲染与性能、加载/错误态一致、配置编辑体验

## 目标与非目标

### 目标
1. **日志渲染与性能**：修复 `key={i}` 导致的过滤切换错位；`filtered` 用 `useMemo`；暂停时提示。
2. **加载/错误态一致**：抽 `useAsync` + `ConfigProvider`，三页统一「加载中 / 错误+重试 / 内容」三态，消除各页独立 `getConfig()` 与散落 toast。
3. **配置编辑体验**：dirty 跟踪 + 未保存提示（切页 AlertDialog 阻断）、端口输入校验、删除确认、保存后联动刷新。

### 非目标
- 不做日志虚拟化（保持 1000 条上限）。
- 不重构 `render_jsonc` 手拼 JSON（独立问题，本次范围外）。
- 不接入 `LauncherConfig.autoStartServer`（独立功能，本次范围外）。
- 不引入状态管理库 / 表单库（zustand/react-hook-form 等）。
- 不改进程状态事件流（`state://update` / `health://update` 保持不变）。

## 架构

### 数据流

```
App
└─ ConfigProvider (mount 时 loadConfig 一次，持有 baseline + draft)
   ├─ Processes   → useConfig().config + useProcessState()
   ├─ Config     → useConfig() 读写 + 端口校验 + 删除确认
   ├─ Bridge     → useConfig() + useAsync(checkDeps)
   ├─ Channels   → useConfig()
   └─ Logs       → useAsync + 稳定 key/memo（与 config 无关）
```

1. `ConfigProvider` mount → `loadConfig` → `{ config: draft, baseline, loading, error }`
2. 配置页编辑 → `update(patch)` 改 draft，`isDirty = JSON.stringify(draft) !== JSON.stringify(baseline)`
3. `save()` → `saveConfig(draft)` → 成功后 `baseline = draft`、bump `config_version` → 后端 health loop 重建 tray/checkers；前端各页经 `useConfig` 自动拿到新配置
4. 进程状态仍走现有 `state://update` / `health://update` 事件，不变

### 为什么是 Provider 而非普通 hook

当前各页独立 `getConfig()` 导致：(a) 切页卸载丢失未保存草稿、(b) Processes 需额外 refresh 链、(c) 保存后其他页不知道。Provider 一次加载全局共享，三问题一并解决。

## 组件与 hooks

### 新增文件
- `src/hooks/useAsync.ts` — 通用异步状态
- `src/hooks/useConfig.tsx` — ConfigProvider + useConfig（替代各页独立 getConfig）
- `src/components/StateView.tsx` — 统一加载/错误渲染

### useAsync

```ts
interface UseAsyncResult<T> {
  data: T | null
  loading: boolean
  error: AppError | null
  reload: () => void
}
function useAsync<T>(fn: () => Promise<T>, deps: unknown[]): UseAsyncResult<T>
```

- `fn` 用 ref 持有，`deps` 决定重跑（语义同 useEffect）
- 不抛异常到调用方，用 error 字段返回
- 取消竞态：用 `reqId` 计数器，`reload` 时递增，旧请求回来后丢弃（若 reqId 不匹配则忽略）

### ConfigProvider / useConfig

```ts
interface ConfigCtx {
  config: AppConfig | null      // 当前 draft（编辑中的值）
  baseline: AppConfig | null    // 上次保存的值
  loading: boolean
  error: AppError | null
  reload: () => void
  update: (patch: (draft: AppConfig) => void) => void   // 不可变更新
  isDirty: boolean                                        // draft !== baseline
  save: () => Promise<boolean>                            // 成功返回 true
  reset: () => void                                       // 丢弃 draft 回到 baseline
}
```

- mount 时 `loadConfig`，set baseline + draft 同值
- `update(patch)` 用 `structuredClone(draft)` + `patch` 生成新 draft（深拷贝保证不可变）
- `isDirty` 用 `JSON.stringify(draft) !== JSON.stringify(baseline)` 计算（配置体量小，可接受）
- `save()` 成功后 `baseline = draft`、bump version；失败时保留 draft，toast 提示，返回 false
- `reset()` 恢复到 baseline

### StateView

```tsx
function StateView({ loading, error, onRetry, children }: {
  loading: boolean
  error: AppError | null
  onRetry?: () => void
  children: ReactNode
})
```

- loading → `<div className="text-muted-foreground text-sm">加载中…</div>`
- error → 显示 `formatError(error)` + 「重试」按钮（触发 onRetry）
- 否则 children

### 页面改造映射

| 页面 | 改造 |
|------|------|
| `Processes.tsx` | 用 `useConfig().config`，删除本地 config state 与 `onConfigUpdate` 回调链；保留 `useProcessState` |
| `Config.tsx` | 用 `useConfig()`，删除本地 state + `getConfig` + `saveConfig`；新增端口校验 + 删除确认 |
| `Bridge.tsx` | 用 `useConfig()` + `useAsync(checkDeps)`；依赖检测区域独立 loading/error |
| `Channels.tsx` | 用 `useConfig()`，移除本地 `getConfig` |
| `Logs.tsx` | 不接 ConfigProvider，独立用 useAsync 拉历史 |

### dirty 阻断切页

在 `AppContent` 监听 `useConfig().isDirty`。点击 navItem 切换时若 dirty，弹 AlertDialog：
- 「保存」→ 调 `save()`，成功后切页
- 「不保存」→ 调 `reset()`，切页
- 「取消」→ 不切页

用 shadcn `alert-dialog` 组件（基于 `@radix-ui/react-dialog`，已安装；需 `npx shadcn@latest add alert-dialog` 添加）。

## LogView 改造

### 当前问题
1. `key={i}` 用数组索引 → 切换 Tab 过滤后条数变化，React 复用错位
2. `setEntries(prev => [...prev, entry].slice(-1000))` 每条都 spread 全数组
3. 切到 server/bridge Tab 时 `entries` 仍是全量，靠 `filtered` 每渲染都重过滤
4. 暂停后再恢复，期间日志被丢弃（`if (paused) return`），无提示

### 改造
1. **后端 LogEntry 加 `seq: u64`**：
   - `src-tauri/src/monitor/log_buffer.rs`：`LogEntry` 结构体加 `pub seq: u64`
   - `src-tauri/src/process/supervisor.rs`：`read_stream` / `read_stream_with_qr` 生成 LogEntry 时用 `AtomicU64` 递增赋 seq
   - `src/lib/types.ts`：`LogEntry` 加 `seq: number`
   - `export_logs` 格式化不改（seq 字段对导出无影响，保持原格式）
2. **前端 key**：`key={e.seq}`
3. **filtered 用 useMemo**：`const filtered = useMemo(() => activeTab === "all" ? entries : entries.filter(e => e.source === activeTab), [entries, activeTab])`
4. **暂停提示**：暂停时顶部 Tabs 旁显示「已暂停，新日志将被丢弃」小标记

## 配置页交互细节

### dirty 跟踪与未保存提示
- `useConfig().isDirty` 驱动保存按钮：dirty 时高亮（`variant="default"`），非 dirty 时置灰（`variant="outline"` disabled）
- Config/Bridge/Channels 三页顶部统一显示「未保存的修改」横幅 + 「放弃修改」按钮（调 `reset()`）
- 切页时若 dirty → 弹 AlertDialog（见上 dirty 阻断切页）

### 端口输入校验
- `<Input type="number" min={1} max={65535}>`
- 失焦时校验：若 `port === 0` 或与同配置内其他 server 重复 → Input 红框 + 下方红字提示
- 用 `portErrors: Record<serverId, string>` 在 Config 页本地 state 管理
- 存在校验错误时保存按钮 disabled（横幅提示「存在校验错误，无法保存」）

### 删除确认
- 删除 server 按钮 → AlertDialog「删除该 server 配置？此操作不可撤销。」「确认删除」「取消」
- 复用新增的 `alert-dialog` shadcn 组件

### 保存后联动刷新
- `useConfig().save()` 成功后 baseline 更新 → `Processes.tsx` 因 `useConfig().config` 变化自动重渲染，拿到新的 servers 列表（含删除/新增的 server）
- 进程状态仍由 `useProcessState` 维护（事件驱动），无需额外 refresh
- `ProcessCard` 的 `onConfigUpdate` prop 移除（不再需要）

## 受影响文件

### 前端
- 新增：`src/hooks/useAsync.ts`、`src/hooks/useConfig.tsx`、`src/components/StateView.tsx`、`src/components/AlertDialog.tsx`（shadcn CLI 生成）
- 修改：`src/App.tsx`（包 ConfigProvider + dirty 阻断切页）、`src/pages/Processes.tsx`、`src/pages/Config.tsx`、`src/pages/Bridge.tsx`、`src/pages/Channels.tsx`、`src/components/LogView.tsx`、`src/components/ProcessCard.tsx`（移除 onConfigUpdate）、`src/lib/types.ts`（LogEntry 加 seq）

### 后端
- 修改：`src-tauri/src/monitor/log_buffer.rs`（LogEntry 加 seq）、`src-tauri/src/process/supervisor.rs`（生成时赋 seq）
- 不改：`src-tauri/src/commands.rs`（export_logs 格式化保持原样）

## 测试

- 无前端测试框架（AGENTS.md 确认）。验证依赖 `npm run build`（tsc + vite build）通过。
- 后端：`cargo test`（log_buffer / supervisor 现有测试需同步更新构造 LogEntry 时加 seq 字段）。

## 验证命令

- `npm run build` — 唯一类型检查
- `cargo test`（在 src-tauri/ 内）— Rust 测试

## 约定遵守

- `@/*` 路径别名 → `./src/*`
- TS strict + noUnusedLocals/noUnusedParameters
- Rust 跨边界结构体用 `#[serde(rename_all = "camelCase")]`（LogEntry 已用，seq 字段自动 camelCase）
- shadcn 组件用 CLI 添加，不手写
- 不改 Vite 端口 1420
