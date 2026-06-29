# UX 与配置编辑优化实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复日志渲染错位、统一加载/错误态、改善配置编辑体验（dirty/端口校验/删除确认/保存联动），保持 1000 条日志上限不变。

**Architecture:** 前端抽 `useAsync` + `ConfigProvider` 两个轻量 hook 收敛加载/错误/dirty 模式；后端给 `LogEntry` 加 `seq: u64` 字段提供稳定 key；Config/Bridge/Channels/Processes 四页改用 `useConfig`，LogView 改用 `seq` 作 key + `useMemo` 过滤。

**Tech Stack:** React 19 + TypeScript strict + Tauri 2 + shadcn/ui (new-york) + tailwind v4 + Rust/tokio

## Global Constraints

- TS strict + `noUnusedLocals`/`noUnusedParameters` — 未使用导入/变量导致 `npm run build` 失败
- `npm run build` (`tsc && vite build`) 是唯一类型检查，无 lint/test 前端脚本
- `cargo test` 在 `src-tauri/` 内运行 Rust 测试
- `@/*` 路径别名 → `./src/*`（tsconfig.json + vite.config.ts）
- Rust 跨边界结构体用 `#[serde(rename_all = "camelCase")]`
- shadcn 组件用 CLI (`npx shadcn@latest add <name>`) 添加，不手写
- Vite 端口 1420 为 `strictPort: true`，不改
- AGENTS.md：「不存在前端测试」「不配置 ESLint/Prettier」→ 前端任务用 `npm run build` 替代测试循环；后端任务保持 TDD
- `ConfigStore::load` 会备份损坏 config.json，不加额外 fallback

## 文件结构

### 新增
- `src/hooks/useAsync.ts` — 通用异步状态 hook（loading/error/data/reload，竞态取消）
- `src/hooks/useConfig.tsx` — ConfigProvider + useConfig（全局单例配置 + dirty + save/reset）
- `src/components/StateView.tsx` — 统一加载/错误/内容三态渲染
- `src/components/ui/alert-dialog.tsx` — shadcn CLI 生成（dirty 阻断切页 + 删除确认复用）

### 修改
- `src-tauri/src/monitor/log_buffer.rs` — LogEntry 加 seq 字段
- `src-tauri/src/process/supervisor.rs` — 生成 LogEntry 时用 AtomicU64 递增 seq
- `src/lib/types.ts` — LogEntry 加 seq 字段
- `src/App.tsx` — 包 ConfigProvider + dirty 阻断切页 AlertDialog
- `src/pages/Processes.tsx` — 用 useConfig 替代本地 config state，移除 onConfigUpdate 链
- `src/pages/Config.tsx` — 用 useConfig + 端口校验 + 删除确认
- `src/pages/Bridge.tsx` — 用 useConfig + useAsync(checkDeps)
- `src/pages/Channels.tsx` — 用 useConfig
- `src/components/LogView.tsx` — key={e.seq} + useMemo 过滤 + 暂停提示
- `src/components/ProcessCard.tsx` — 移除 onConfigUpdate prop

---

### Task 1: 后端 LogEntry 加 seq 字段

**Files:**
- Modify: `src-tauri/src/monitor/log_buffer.rs`
- Modify: `src-tauri/src/process/supervisor.rs`
- Test: `cargo test`（现有 log_buffer 测试需更新构造 LogEntry）

**Interfaces:**
- Produces: `LogEntry { ts, source, level, line, seq: u64 }`（Rust 侧 + serde camelCase → `seq`）
- 后续任务 7 在前端 types.ts 加对应 `seq: number`

- [ ] **Step 1: 写失败测试 — 更新 log_buffer 测试构造 LogEntry 带 seq**

修改 `src-tauri/src/monitor/log_buffer.rs` 的 `tests` 模块辅助函数：

```rust
fn entry(src: &str, n: i64) -> LogEntry {
    LogEntry { ts: n, source: src.to_string(), level: "info".to_string(), line: format!("line {}", n), seq: 0 }
}
```

同时在 `evicts_oldest_when_over_capacity` 测试里加断言验证 seq 存在：

```rust
#[test]
fn evicts_oldest_when_over_capacity() {
    let mut buf = LogBuffer::new(3);
    for i in 0..5 { buf.push(entry("server", i)); }
    let recent = buf.recent("server", 10);
    assert_eq!(recent.len(), 3);
    assert_eq!(recent[0].line, "line 2");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test`（在 `src-tauri/` 内）
Expected: 编译错误 — `LogEntry` 缺少 `seq` 字段

- [ ] **Step 3: 实现 — 给 LogEntry 加 seq 字段**

修改 `src-tauri/src/monitor/log_buffer.rs`：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub ts: i64,
    pub source: String,
    pub level: String,
    pub line: String,
    pub seq: u64,
}
```

- [ ] **Step 4: 实现 — supervisor 生成时递增 seq**

修改 `src-tauri/src/process/supervisor.rs`。在文件顶部 `use` 之后加一个进程内全局计数器（AtomicU64），并在 `read_stream` 与 `read_stream_with_qr` 生成 LogEntry 时调用：

```rust
use std::sync::atomic::{AtomicU64, Ordering};

static LOG_SEQ: AtomicU64 = AtomicU64::new(0);

fn next_seq() -> u64 {
    LOG_SEQ.fetch_add(1, Ordering::Relaxed)
}
```

在 `read_stream` 函数里（`on_log(LogEntry { ... })` 两处，正常行与错误行）加 `seq: next_seq()`：

```rust
Ok(Some(line)) => on_log(LogEntry { ts: now_ts(), source: source.clone(), level: level.clone(), line, seq: next_seq() }),
```

```rust
on_log(LogEntry {
    ts: now_ts(),
    source: source.clone(),
    level: "error".to_string(),
    line: format!("stream read error: {}", e),
    seq: next_seq(),
}),
```

在 `read_stream_with_qr` 函数里同样加：

```rust
on_log(LogEntry { ts: now_ts(), source: source.clone(), level: level.clone(), line, seq: next_seq() });
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test`（在 `src-tauri/` 内）
Expected: 全部通过，包括更新后的 log_buffer 测试

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/monitor/log_buffer.rs src-tauri/src/process/supervisor.rs
git commit -m "feat: add seq field to LogEntry for stable log keys"
```

---

### Task 2: 前端 types.ts 同步 LogEntry.seq

**Files:**
- Modify: `src/lib/types.ts:102-107`

**Interfaces:**
- Produces: 前端 `LogEntry` 类型带 `seq: number`，供 Task 8（LogView）使用
- Consumes: Task 1 的后端 `LogEntry.seq`（camelCase 序列化为 `seq`）

- [ ] **Step 1: 实现 — 修改 LogEntry 接口加 seq**

修改 `src/lib/types.ts`：

```ts
export interface LogEntry {
  ts: number
  source: "server" | "bridge"
  level: "info" | "error"
  line: string
  seq: number
}
```

- [ ] **Step 2: 验证类型检查通过**

Run: `npm run build`
Expected: 构建成功（LogView 当前用 `key={i}`，不引用 seq，加字段不影响编译；TS 接口加字段是可选消费）

- [ ] **Step 3: 提交**

```bash
git add src/lib/types.ts
git commit -m "feat: sync TS LogEntry with Rust seq field"
```

---

### Task 3: useAsync hook

**Files:**
- Create: `src/hooks/useAsync.ts`

**Interfaces:**
- Produces: `useAsync<T>(fn: () => Promise<T>, deps: unknown[]): { data: T | null, loading: boolean, error: AppError | null, reload: () => void }`
- 后续 Task 5（Bridge）用 `useAsync(checkDeps, [])`；LogView 不用此 hook（它有事件流追加逻辑，不适用此模式）

- [ ] **Step 1: 实现 — 创建 useAsync hook**

创建 `src/hooks/useAsync.ts`：

```ts
import { useCallback, useEffect, useRef, useState } from "react"
import type { AppError } from "@/lib/types"

interface UseAsyncResult<T> {
  data: T | null
  loading: boolean
  error: AppError | null
  reload: () => void
}

export function useAsync<T>(fn: () => Promise<T>, deps: unknown[]): UseAsyncResult<T> {
  const [data, setData] = useState<T | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<AppError | null>(null)
  const fnRef = useRef(fn)
  fnRef.current = fn
  const reqIdRef = useRef(0)

  const reload = useCallback(() => {
    const myId = ++reqIdRef.current
    setLoading(true)
    setError(null)
    fnRef.current()
      .then((result) => {
        if (myId !== reqIdRef.current) return
        setData(result)
        setLoading(false)
      })
      .catch((e: unknown) => {
        if (myId !== reqIdRef.current) return
        const err: AppError = e && typeof e === "object" && "kind" in e
          ? (e as AppError)
          : { kind: "Process", message: String(e) }
        setError(err)
        setLoading(false)
      })
  }, [])

  useEffect(() => {
    reload()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps)

  return { data, loading, error, reload }
}
```

- [ ] **Step 2: 验证类型检查通过**

Run: `npm run build`
Expected: 构建成功（hook 未被使用时 noUnusedLocals 不会报错，因为 export 的符号算被使用）

- [ ] **Step 3: 提交**

```bash
git add src/hooks/useAsync.ts
git commit -m "feat: add useAsync hook for loading/error/reload state"
```

---

### Task 4: ConfigProvider + useConfig

**Files:**
- Create: `src/hooks/useConfig.tsx`

**Interfaces:**
- Consumes: `getConfig`, `saveConfig` from `@/lib/tauri`；`AppConfig` from `@/lib/types`
- Produces: `<ConfigProvider>` 组件 + `useConfig()` hook，返回 `{ config, baseline, loading, error, reload, update, isDirty, save, reset }`
- 后续 Task 6-9 四页 + App.tsx 使用

- [ ] **Step 1: 实现 — 创建 ConfigProvider**

创建 `src/hooks/useConfig.tsx`：

```tsx
import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react"
import { getConfig, saveConfig } from "@/lib/tauri"
import type { AppConfig, AppError } from "@/lib/types"

interface ConfigCtx {
  config: AppConfig | null
  baseline: AppConfig | null
  loading: boolean
  error: AppError | null
  reload: () => void
  update: (patch: (draft: AppConfig) => void) => void
  isDirty: boolean
  save: () => Promise<boolean>
  reset: () => void
}

const Ctx = createContext<ConfigCtx>({
  config: null,
  baseline: null,
  loading: true,
  error: null,
  reload: () => {},
  update: () => {},
  isDirty: false,
  save: () => Promise.resolve(false),
  reset: () => {},
})

export function ConfigProvider({ children }: { children: ReactNode }) {
  const [baseline, setBaseline] = useState<AppConfig | null>(null)
  const [draft, setDraft] = useState<AppConfig | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<AppError | null>(null)
  const reqIdRef = useRef(0)

  const reload = useCallback(() => {
    const myId = ++reqIdRef.current
    setLoading(true)
    setError(null)
    getConfig()
      .then((cfg) => {
        if (myId !== reqIdRef.current) return
        setBaseline(cfg)
        setDraft(cfg)
        setLoading(false)
      })
      .catch((e: unknown) => {
        if (myId !== reqIdRef.current) return
        const err: AppError = e && typeof e === "object" && "kind" in e
          ? (e as AppError)
          : { kind: "Config", message: String(e) }
        setError(err)
        setLoading(false)
      })
  }, [])

  useEffect(() => { reload() }, [reload])

  const update = useCallback((patch: (draft: AppConfig) => void) => {
    setDraft((prev) => {
      if (!prev) return prev
      const next = structuredClone(prev)
      patch(next)
      return next
    })
  }, [])

  const isDirty = baseline !== null && draft !== null
    && JSON.stringify(draft) !== JSON.stringify(baseline)

  const save = useCallback(async () => {
    if (!draft) return false
    try {
      await saveConfig(draft)
      setBaseline(draft)
      return true
    } catch (e: unknown) {
      const err: AppError = e && typeof e === "object" && "kind" in e
        ? (e as AppError)
        : { kind: "Config", message: String(e) }
      setError(err)
      return false
    }
  }, [draft])

  const reset = useCallback(() => {
    setDraft(baseline)
  }, [baseline])

  return (
    <Ctx.Provider value={{ config: draft, baseline, loading, error, reload, update, isDirty, save, reset }}>
      {children}
    </Ctx.Provider>
  )
}

export function useConfig() {
  return useContext(Ctx)
}
```

- [ ] **Step 2: 验证类型检查通过**

Run: `npm run build`
Expected: 构建成功

- [ ] **Step 3: 提交**

```bash
git add src/hooks/useConfig.tsx
git commit -m "feat: add ConfigProvider for shared config state with dirty tracking"
```

---

### Task 5: StateView 组件

**Files:**
- Create: `src/components/StateView.tsx`

**Interfaces:**
- Consumes: `formatError` from `@/lib/utils`；`AppError` from `@/lib/types`
- Produces: `<StateView loading error onRetry children>`，供各页统一渲染加载/错误/内容

- [ ] **Step 1: 实现 — 创建 StateView**

创建 `src/components/StateView.tsx`：

```tsx
import type { ReactNode } from "react"
import type { AppError } from "@/lib/types"
import { formatError } from "@/lib/utils"

interface StateViewProps {
  loading: boolean
  error: AppError | null
  onRetry?: () => void
  children: ReactNode
}

export function StateView({ loading, error, onRetry, children }: StateViewProps) {
  if (loading) {
    return <div className="text-muted-foreground text-sm">加载中…</div>
  }
  if (error) {
    return (
      <div className="flex flex-col items-center gap-2 py-8 text-center">
        <p className="text-sm text-destructive">{formatError(error)}</p>
        {onRetry && (
          <button className="rounded bg-primary px-3 py-1 text-xs text-primary-foreground" onClick={onRetry}>
            重试
          </button>
        )}
      </div>
    )
  }
  return <>{children}</>
}
```

- [ ] **Step 2: 验证类型检查通过**

Run: `npm run build`
Expected: 构建成功

- [ ] **Step 3: 提交**

```bash
git add src/components/StateView.tsx
git commit -m "feat: add StateView for unified loading/error display"
```

---

### Task 6: 添加 shadcn alert-dialog 组件

**Files:**
- Create: `src/components/ui/alert-dialog.tsx`（shadcn CLI 生成）
- Create: `src/components/ui/alert-dialog.tsx` 依赖 `@radix-ui/react-alert-dialog`（CLI 自动安装）

**Interfaces:**
- Produces: shadcn AlertDialog 系列组件（AlertDialog / AlertDialogTrigger / AlertDialogContent / AlertDialogHeader / AlertDialogFooter / AlertDialogTitle / AlertDialogDescription / AlertDialogAction / AlertDialogCancel）
- 后续 Task 10（App.tsx dirty 阻断切页）与 Task 8（Config 删除确认）使用

- [ ] **Step 1: 用 shadcn CLI 添加 alert-dialog**

Run: `npx shadcn@latest add alert-dialog`
Expected: 安装 `@radix-ui/react-alert-dialog` 依赖，生成 `src/components/ui/alert-dialog.tsx`

- [ ] **Step 2: 验证类型检查通过**

Run: `npm run build`
Expected: 构建成功（新组件未被使用也不报错，因为 shadcn 组件都是 export 的）

- [ ] **Step 3: 提交**

```bash
git add src/components/ui/alert-dialog.tsx package.json package-lock.json
git commit -m "feat: add shadcn alert-dialog component"
```

---

### Task 7: LogView 改用 seq 作 key + useMemo 过滤 + 暂停提示

**Files:**
- Modify: `src/components/LogView.tsx`

**Interfaces:**
- Consumes: Task 2 的 `LogEntry.seq`

- [ ] **Step 1: 实现 — 改 LogView**

修改 `src/components/LogView.tsx` 全文为：

```tsx
import { useEffect, useMemo, useRef, useState } from "react"
import { Button } from "@/components/ui/button"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Pause, Play, Trash2, Download } from "lucide-react"
import { useTauriEvent } from "@/hooks/useTauriEvent"
import { getLogHistory, clearLogs, exportLogs } from "@/lib/tauri"
import type { LogEntry } from "@/lib/types"
import { toast } from "sonner"
import { formatError } from "@/lib/utils"

export function LogView({ height = "400px" }: { height?: string }) {
  const [entries, setEntries] = useState<LogEntry[]>([])
  const [paused, setPaused] = useState(false)
  const [activeTab, setActiveTab] = useState<"all" | "server" | "bridge">("all")
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    getLogHistory("all", 500).then(setEntries).catch((e) => console.error("[load log history]", e))
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

  const filtered = useMemo(
    () => activeTab === "all" ? entries : entries.filter((e) => e.source === activeTab),
    [entries, activeTab],
  )

  const handleClear = () => {
    clearLogs(activeTab).then(() => {
      setEntries((prev) => activeTab === "all" ? [] : prev.filter((e) => e.source !== activeTab))
    }).catch((e) => toast.error(`清空失败: ${formatError(e)}`))
  }

  const handleExport = () => {
    exportLogs(activeTab).then((path) => toast.success(`已导出到: ${path}`)).catch((e) => toast.error(`导出失败: ${formatError(e)}`))
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
        <div className="flex items-center gap-1">
          {paused && <span className="text-xs text-muted-foreground">已暂停</span>}
          <Button size="sm" variant="ghost" onClick={() => setPaused((p) => !p)}>
            {paused ? <Play className="h-3 w-3" /> : <Pause className="h-3 w-3" />}
          </Button>
          <Button size="sm" variant="ghost" onClick={handleClear}><Trash2 className="h-3 w-3" /></Button>
          <Button size="sm" variant="ghost" onClick={handleExport}><Download className="h-3 w-3" /></Button>
        </div>
      </div>
      <div className={`overflow-auto rounded border bg-muted/30 p-2 font-mono text-xs`} style={{ height }}>
        {filtered.map((e) => (
          <div key={e.seq} className={e.level === "error" ? "text-red-500" : "text-foreground"}>
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

- [ ] **Step 2: 验证类型检查通过**

Run: `npm run build`
Expected: 构建成功（`key={e.seq}` 用了新字段；`useMemo` 已 import；移除了未使用的 `i` 索引参数）

- [ ] **Step 3: 提交**

```bash
git add src/components/LogView.tsx
git commit -m "fix: use seq as log key, memoize filter, show pause indicator"
```

---

### Task 8: Config 页改用 useConfig + 端口校验 + 删除确认

**Files:**
- Modify: `src/pages/Config.tsx`

**Interfaces:**
- Consumes: `useConfig` from `@/hooks/useConfig`；`StateView` from `@/components/StateView`；shadcn AlertDialog（Task 6）；`AppConfig`, `ServerConfig` from `@/lib/types`

- [ ] **Step 1: 实现 — 重写 Config 页**

修改 `src/pages/Config.tsx` 全文为：

```tsx
import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  AlertDialog, AlertDialogAction, AlertDialogCancel,
  AlertDialogContent, AlertDialogDescription, AlertDialogFooter,
  AlertDialogHeader, AlertDialogTitle, AlertDialogTrigger,
} from "@/components/ui/alert-dialog"
import { StateView } from "@/components/StateView"
import { useConfig } from "@/hooks/useConfig"
import { toast } from "sonner"
import type { ServerConfig } from "@/lib/types"

function genId() {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8)
}

export function Config() {
  const { config, loading, error, reload, update, isDirty, save, reset } = useConfig()
  const [portErrors, setPortErrors] = useState<Record<string, string>>({})
  const [deleteId, setDeleteId] = useState<string | null>(null)

  if (loading || !config) return <StateView loading={loading} error={error} onRetry={reload}>{null}</StateView>

  const validatePort = (id: string, port: number) => {
    const errs = { ...portErrors }
    if (port < 1 || port > 65535) {
      errs[id] = "端口范围 1-65535"
    } else {
      const dup = config.servers.some((s) => s.id !== id && s.port === port)
      errs[id] = dup ? "端口与其他 server 重复" : ""
    }
    setPortErrors(errs)
  }

  const updateServer = (id: string, patch: Partial<ServerConfig>) => {
    update((draft) => {
      const s = draft.servers.find((x) => x.id === id)
      if (s) Object.assign(s, patch)
    })
    if ("port" in patch) validatePort(id, patch.port ?? 0)
  }

  const addServer = () => {
    update((draft) => {
      draft.servers.push({
        id: genId(), name: "新 server", hostname: "127.0.0.1", port: 4097, cwd: "", extraEnv: {},
      })
    })
  }

  const removeServer = (id: string) => {
    update((draft) => {
      draft.servers = draft.servers.filter((s) => s.id !== id)
    })
    const errs = { ...portErrors }
    delete errs[id]
    setPortErrors(errs)
    setDeleteId(null)
  }

  const hasErrors = Object.values(portErrors).some((e) => e.length > 0)

  const handleSave = () => {
    save().then((ok) => {
      if (ok) toast.success("已保存")
      else toast.error("保存失败")
    })
  }

  return (
    <div className="space-y-4">
      <StateView loading={false} error={error} onRetry={reload}>
        {isDirty && (
          <div className="flex items-center justify-between rounded-md border border-yellow-500/50 bg-yellow-500/10 px-3 py-2 text-xs text-yellow-700 dark:text-yellow-400">
            <span>未保存的修改</span>
            <Button size="sm" variant="ghost" onClick={reset}>放弃修改</Button>
          </div>
        )}
        <Card>
          <CardHeader><CardTitle>opencode servers</CardTitle></CardHeader>
          <CardContent className="space-y-4">
            {config.servers.map((s) => (
              <div key={s.id} className="space-y-2 border-b pb-4">
                <div className="flex items-center justify-between">
                  <Label className="text-xs text-muted-foreground">ID: {s.id}</Label>
                  <AlertDialog open={deleteId === s.id} onOpenChange={(o) => setDeleteId(o ? s.id : null)}>
                    <AlertDialogTrigger asChild>
                      <Button size="sm" variant="destructive">删除</Button>
                    </AlertDialogTrigger>
                    <AlertDialogContent>
                      <AlertDialogHeader>
                        <AlertDialogTitle>删除该 server 配置？</AlertDialogTitle>
                        <AlertDialogDescription>此操作不可撤销。</AlertDialogDescription>
                      </AlertDialogHeader>
                      <AlertDialogFooter>
                        <AlertDialogCancel>取消</AlertDialogCancel>
                        <AlertDialogAction onClick={() => removeServer(s.id)}>确认删除</AlertDialogAction>
                      </AlertDialogFooter>
                    </AlertDialogContent>
                  </AlertDialog>
                </div>
                <div className="grid grid-cols-3 gap-2">
                  <div className="space-y-1">
                    <Label>名称</Label>
                    <Input value={s.name} onChange={(e) => updateServer(s.id, { name: e.target.value })} />
                  </div>
                  <div className="space-y-1">
                    <Label>hostname</Label>
                    <Input value={s.hostname} onChange={(e) => updateServer(s.id, { hostname: e.target.value })} />
                  </div>
                  <div className="space-y-1">
                    <Label>port</Label>
                    <Input type="number" min={1} max={65535} value={s.port}
                      onChange={(e) => updateServer(s.id, { port: Number(e.target.value) })}
                      onBlur={() => validatePort(s.id, s.port)} />
                    {portErrors[s.id] && <p className="text-xs text-destructive">{portErrors[s.id]}</p>}
                  </div>
                </div>
                <div className="space-y-1">
                  <Label>工作目录 (cwd)</Label>
                  <Input value={s.cwd} onChange={(e) => updateServer(s.id, { cwd: e.target.value })} />
                </div>
              </div>
            ))}
            <Button variant="outline" onClick={addServer}>添加 server</Button>
          </CardContent>
        </Card>
        <Button onClick={handleSave} disabled={!isDirty || hasErrors}>
          {hasErrors ? "存在校验错误" : "保存"}
        </Button>
      </StateView>
    </div>
  )
}
```

- [ ] **Step 2: 验证类型检查通过**

Run: `npm run build`
Expected: 构建成功

- [ ] **Step 3: 提交**

```bash
git add src/pages/Config.tsx
git commit -m "feat: Config page uses useConfig with dirty tracking, port validation, delete confirm"
```

---

### Task 9: Bridge 页改用 useConfig + useAsync(checkDeps)

**Files:**
- Modify: `src/pages/Bridge.tsx`

**Interfaces:**
- Consumes: `useConfig` from `@/hooks/useConfig`；`useAsync` from `@/hooks/useAsync`；`StateView` from `@/components/StateView`；`checkDeps`, `checkBridgeUpdate`, `updateBridge`, `reinstallBridge` from `@/lib/tauri`

- [ ] **Step 1: 实现 — 重写 Bridge 页**

修改 `src/pages/Bridge.tsx` 全文为：

```tsx
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import { StateView } from "@/components/StateView"
import { useConfig } from "@/hooks/useConfig"
import { useAsync } from "@/hooks/useAsync"
import { checkBridgeUpdate, updateBridge, reinstallBridge, checkDeps } from "@/lib/tauri"
import { toast } from "sonner"
import { formatError } from "@/lib/utils"

export function Bridge() {
  const { config, loading, error, reload, update, isDirty, save, reset } = useConfig()
  const { data: deps, loading: depsLoading, error: depsError, reload: depsReload } = useAsync(checkDeps, [])

  if (loading || !config) return <StateView loading={loading} error={error} onRetry={reload}>{null}</StateView>

  const handleSave = () => {
    save().then((ok) => { if (ok) toast.success("已保存"); else toast.error("保存失败") })
  }

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader><CardTitle>依赖检测</CardTitle></CardHeader>
        <CardContent>
          <StateView loading={depsLoading} error={depsError} onRetry={depsReload}>
            {deps && (
              <div className="flex flex-wrap gap-2">
                {Object.entries(deps).map(([k, v]) => (
                  <Badge key={k} variant={v ? "default" : "destructive"}>{k}: {v ? "已安装" : "缺失"}</Badge>
                ))}
              </div>
            )}
          </StateView>
        </CardContent>
      </Card>
      <Card>
        <CardHeader><CardTitle>Bridge 配置</CardTitle></CardHeader>
        <CardContent className="space-y-3">
          <div className="space-y-1">
            <Label>安装路径（留空用默认）</Label>
            <Input value={config.bridge.installPath ?? ""}
              onChange={(e) => update((d) => { d.bridge.installPath = e.target.value || null })} />
          </div>
          <div className="space-y-1">
            <Label>defaultAgent</Label>
            <Input value={config.bridge.defaultAgent}
              onChange={(e) => update((d) => { d.bridge.defaultAgent = e.target.value })} />
          </div>
          <div className="space-y-1">
            <Label>dataDir</Label>
            <Input value={config.bridge.dataDir}
              onChange={(e) => update((d) => { d.bridge.dataDir = e.target.value })} />
          </div>
        </CardContent>
      </Card>
      {isDirty && (
        <div className="flex items-center justify-between rounded-md border border-yellow-500/50 bg-yellow-500/10 px-3 py-2 text-xs text-yellow-700 dark:text-yellow-400">
          <span>未保存的修改</span>
          <Button size="sm" variant="ghost" onClick={reset}>放弃修改</Button>
        </div>
      )}
      <div className="flex gap-2">
        <Button variant="outline" onClick={() => checkBridgeUpdate().then((u) => toast.info(u ? "已是最新" : "有更新可用")).catch((e) => toast.error(`检查失败: ${formatError(e)}`))}>检查更新</Button>
        <Button variant="outline" onClick={() => updateBridge().then(() => toast.success("已更新")).catch((e) => toast.error(`更新失败: ${formatError(e)}`))}>更新</Button>
        <Button variant="outline" onClick={() => reinstallBridge().then(() => toast.success("已重装")).catch((e) => toast.error(`重装失败: ${formatError(e)}`))}>重新安装</Button>
      </div>
      <Button onClick={handleSave} disabled={!isDirty}>保存</Button>
    </div>
  )
}
```

- [ ] **Step 2: 验证类型检查通过**

Run: `npm run build`
Expected: 构建成功

- [ ] **Step 3: 提交**

```bash
git add src/pages/Bridge.tsx
git commit -m "feat: Bridge page uses useConfig and useAsync for deps"
```

---

### Task 10: Channels 页改用 useConfig

**Files:**
- Modify: `src/pages/Channels.tsx`

**Interfaces:**
- Consumes: `useConfig` from `@/hooks/useConfig`；`StateView` from `@/components/StateView`

- [ ] **Step 1: 实现 — 重写 Channels 页**

修改 `src/pages/Channels.tsx` 全文为：

```tsx
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { StateView } from "@/components/StateView"
import { useConfig } from "@/hooks/useConfig"
import { toast } from "sonner"

export function Channels() {
  const { config, loading, error, reload, update, isDirty, save, reset } = useConfig()

  if (loading || !config) return <StateView loading={loading} error={error} onRetry={reload}>{null}</StateView>

  const updateChannel = <K extends keyof typeof config.channels>(
    channel: K,
    patch: Partial<typeof config.channels[K]>,
  ) => update((d) => { Object.assign(d.channels[channel], patch) })

  const handleSave = () => {
    save().then((ok) => { if (ok) toast.success("已保存"); else toast.error("保存失败") })
  }

  return (
    <div className="space-y-4">
      {isDirty && (
        <div className="flex items-center justify-between rounded-md border border-yellow-500/50 bg-yellow-500/10 px-3 py-2 text-xs text-yellow-700 dark:text-yellow-400">
          <span>未保存的修改</span>
          <Button size="sm" variant="ghost" onClick={reset}>放弃修改</Button>
        </div>
      )}
      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>飞书</CardTitle>
          <Switch checked={config.channels.feishu.enabled} onCheckedChange={(v) => updateChannel("feishu", { enabled: v })} />
        </CardHeader>
        {config.channels.feishu.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>App ID</Label><Input value={config.channels.feishu.appId} onChange={(e) => updateChannel("feishu", { appId: e.target.value })} /></div>
            <div className="space-y-1"><Label>App Secret</Label><Input type="password" value={config.channels.feishu.appSecret} onChange={(e) => updateChannel("feishu", { appSecret: e.target.value })} /></div>
            <div className="space-y-1"><Label>Verification Token</Label><Input value={config.channels.feishu.verificationToken} onChange={(e) => updateChannel("feishu", { verificationToken: e.target.value })} /></div>
            <div className="space-y-1"><Label>Webhook Port</Label><Input type="number" value={config.channels.feishu.webhookPort} onChange={(e) => updateChannel("feishu", { webhookPort: Number(e.target.value) })} /></div>
            <div className="space-y-1"><Label>Encrypt Key</Label><Input value={config.channels.feishu.encryptKey} onChange={(e) => updateChannel("feishu", { encryptKey: e.target.value })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>QQ</CardTitle>
          <Switch checked={config.channels.qq.enabled} onCheckedChange={(v) => updateChannel("qq", { enabled: v })} />
        </CardHeader>
        {config.channels.qq.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>App ID</Label><Input value={config.channels.qq.appId} onChange={(e) => updateChannel("qq", { appId: e.target.value })} /></div>
            <div className="space-y-1"><Label>Secret</Label><Input type="password" value={config.channels.qq.secret} onChange={(e) => updateChannel("qq", { secret: e.target.value })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>Telegram</CardTitle>
          <Switch checked={config.channels.telegram.enabled} onCheckedChange={(v) => updateChannel("telegram", { enabled: v })} />
        </CardHeader>
        {config.channels.telegram.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>Bot Token</Label><Input type="password" value={config.channels.telegram.botToken} onChange={(e) => updateChannel("telegram", { botToken: e.target.value })} /></div>
            <div className="space-y-1"><Label>Allowed Chat IDs（逗号分隔）</Label><Input value={config.channels.telegram.allowedChatIds.join(",")} onChange={(e) => updateChannel("telegram", { allowedChatIds: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>Discord</CardTitle>
          <Switch checked={config.channels.discord.enabled} onCheckedChange={(v) => updateChannel("discord", { enabled: v })} />
        </CardHeader>
        {config.channels.discord.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>Bot Token</Label><Input type="password" value={config.channels.discord.botToken} onChange={(e) => updateChannel("discord", { botToken: e.target.value })} /></div>
            <div className="space-y-1"><Label>Allowed Channel IDs（逗号分隔）</Label><Input value={config.channels.discord.allowedChannelIds.join(",")} onChange={(e) => updateChannel("discord", { allowedChannelIds: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>微信</CardTitle>
          <Switch checked={config.channels.wechat.enabled} onCheckedChange={(v) => updateChannel("wechat", { enabled: v })} />
        </CardHeader>
        {config.channels.wechat.enabled && (
          <CardContent><p className="text-sm text-muted-foreground">微信使用扫码登录，启动后请在弹窗中扫码。</p></CardContent>
        )}
      </Card>

      <Button onClick={handleSave} disabled={!isDirty}>保存</Button>
    </div>
  )
}
```

- [ ] **Step 2: 验证类型检查通过**

Run: `npm run build`
Expected: 构建成功

- [ ] **Step 3: 提交**

```bash
git add src/pages/Channels.tsx
git commit -m "feat: Channels page uses useConfig with dirty tracking"
```

---

### Task 11: Processes 页 + ProcessCard 改用 useConfig

**Files:**
- Modify: `src/pages/Processes.tsx`
- Modify: `src/components/ProcessCard.tsx`

**Interfaces:**
- Consumes: `useConfig` from `@/hooks/useConfig`；`StateView` from `@/components/StateView`

- [ ] **Step 1: 实现 — 重写 Processes 页**

修改 `src/pages/Processes.tsx` 全文为：

```tsx
import { ProcessCard } from "@/components/ProcessCard"
import { StateView } from "@/components/StateView"
import { useProcessState } from "@/hooks/useProcessState"
import { useConfig } from "@/hooks/useConfig"

export function Processes() {
  const { state } = useProcessState()
  const { config, loading, error, reload } = useConfig()

  if (loading || !config) return <StateView loading={loading} error={error} onRetry={reload}>{null}</StateView>

  return (
    <div className="grid grid-cols-2 gap-4">
      {state.servers.map((s) => (
        <ProcessCard key={s.id} target="server" state={s.state} serverId={s.id} name={s.name} />
      ))}
      <ProcessCard target="bridge" state={state.bridge} servers={config.servers} boundServerId={config.bridge.boundServerId} />
    </div>
  )
}
```

- [ ] **Step 2: 实现 — ProcessCard 移除 onConfigUpdate**

修改 `src/components/ProcessCard.tsx`。删除 `onConfigUpdate` prop 及其在 `handleBind` 中的调用：

```tsx
import { Switch } from "@/components/ui/switch"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { AlertTriangle } from "lucide-react"
import type { ProcessState, ProcessTarget, ServerConfig } from "@/lib/types"
import { startProcess, stopProcess, bindBridge } from "@/lib/tauri"
import { toast } from "sonner"
import { formatError } from "@/lib/utils"

const stateColor: Record<string, string> = {
  Running: "bg-green-500",
  Stopped: "bg-gray-400",
  Starting: "bg-yellow-500",
  Stopping: "bg-orange-500",
  Failed: "bg-red-500",
}

interface ProcessCardProps {
  target: ProcessTarget
  state: ProcessState
  serverId?: string
  name?: string
  servers?: ServerConfig[]
  boundServerId?: string
}

export function ProcessCard({ target, state, serverId, name, servers, boundServerId }: ProcessCardProps) {
  const label = target === "server" ? (name ?? "server") : "bridge"
  const isRunning = state.state === "Running"
  const isBusy = state.state === "Starting" || state.state === "Stopping"

  const handleToggle = (checked: boolean) => {
    if (checked) {
      startProcess(target, serverId).catch((e) => toast.error(`启动失败: ${formatError(e)}`))
    } else {
      stopProcess(target, serverId).catch((e) => toast.error(`停止失败: ${formatError(e)}`))
    }
  }
  const handleBind = (newId: string) => bindBridge(newId)
    .catch((e) => toast.error(`绑定失败: ${formatError(e)}`))

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
          {state.pid != null && <div>PID: {state.pid}</div>}
          {state.uptimeSec != null && <div>运行时长: {state.uptimeSec}s</div>}
          {state.healthy != null && <div>健康: {state.healthy ? "正常" : "异常"}</div>}
          {state.exitCode != null && <div>退出码: {state.exitCode}</div>}
        </div>
        {target === "bridge" && servers && (
          <div className="mt-3 space-y-1">
            <span className="text-xs text-muted-foreground">绑定 server</span>
            <select
              className="w-full rounded border bg-transparent px-2 py-1 text-xs"
              value={boundServerId ?? ""}
              onChange={(e) => handleBind(e.target.value)}
            >
              {servers.map((s) => (
                <option key={s.id} value={s.id}>{s.name}</option>
              ))}
            </select>
          </div>
        )}
        <div className="mt-3 flex items-center gap-2">
          <Switch checked={isRunning} disabled={isBusy} onCheckedChange={handleToggle} />
          <span className="text-xs text-muted-foreground">{isRunning ? "运行中" : "已停止"}</span>
        </div>
        {target === "bridge" && (
          <div className="mt-3 flex items-start gap-2 rounded-md border border-yellow-500/50 bg-yellow-500/10 p-2 text-xs text-yellow-700 dark:text-yellow-400">
            <AlertTriangle className="h-3.5 w-3.5 shrink-0 translate-y-0.5" />
            <span>opencode web 和 bridge 建议不要同时使用同一个 serve。</span>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
```

- [ ] **Step 3: 验证类型检查通过**

Run: `npm run build`
Expected: 构建成功（`onConfigUpdate` 及其调用链已移除，无悬空引用）

- [ ] **Step 4: 提交**

```bash
git add src/pages/Processes.tsx src/components/ProcessCard.tsx
git commit -m "feat: Processes page uses useConfig, remove onConfigUpdate chain"
```

---

### Task 12: App.tsx 包 ConfigProvider + dirty 阻断切页

**Files:**
- Modify: `src/App.tsx`

**Interfaces:**
- Consumes: `ConfigProvider`, `useConfig` from `@/hooks/useConfig`；shadcn AlertDialog（Task 6）

- [ ] **Step 1: 实现 — 重写 App.tsx**

修改 `src/App.tsx` 全文为：

```tsx
import { useEffect, useState } from "react"
import { Cpu, Settings, Boxes, Radio, ScrollText } from "lucide-react"
import { Processes } from "@/pages/Processes"
import { Config } from "@/pages/Config"
import { Bridge } from "@/pages/Bridge"
import { Channels } from "@/pages/Channels"
import { Logs } from "@/pages/Logs"
import { WechatQrDialog } from "@/components/WechatQrDialog"
import {
  AlertDialog, AlertDialogAction, AlertDialogCancel,
  AlertDialogContent, AlertDialogDescription, AlertDialogFooter,
  AlertDialogHeader, AlertDialogTitle,
} from "@/components/ui/alert-dialog"
import { Toaster } from "@/components/ui/sonner"
import { ProcessStateProvider, useProcessState } from "@/hooks/useProcessState"
import { ConfigProvider, useConfig } from "@/hooks/useConfig"
import { toast } from "sonner"
import { cn } from "@/lib/utils"

type Page = "processes" | "config" | "bridge" | "channels" | "logs"

const navItems: { id: Page; label: string; icon: React.ReactNode }[] = [
  { id: "processes", label: "进程", icon: <Cpu className="h-4 w-4" /> },
  { id: "config", label: "配置", icon: <Settings className="h-4 w-4" /> },
  { id: "bridge", label: "Bridge", icon: <Boxes className="h-4 w-4" /> },
  { id: "channels", label: "渠道", icon: <Radio className="h-4 w-4" /> },
  { id: "logs", label: "日志", icon: <ScrollText className="h-4 w-4" /> },
]

export default function App() {
  return (
    <ConfigProvider>
      <ProcessStateProvider>
        <AppInner />
      </ProcessStateProvider>
    </ConfigProvider>
  )
}

function AppInner() {
  const [page, setPage] = useState<Page>("processes")
  const { refresh } = useProcessState()

  useEffect(() => { refresh() }, [refresh])

  return <AppContent page={page} setPage={setPage} />
}

function AppContent({ page, setPage }: { page: Page; setPage: (p: Page) => void }) {
  const { isDirty, save, reset } = useConfig()
  const [pendingPage, setPendingPage] = useState<Page | null>(null)

  const trySetPage = (next: Page) => {
    if (isDirty) setPendingPage(next)
    else setPage(next)
  }

  const handleSaveAndLeave = () => {
    save().then((ok) => {
      if (ok && pendingPage) setPage(pendingPage)
      else if (!ok) toast.error("保存失败")
      setPendingPage(null)
    })
  }

  const handleDiscardAndLeave = () => {
    reset()
    if (pendingPage) setPage(pendingPage)
    setPendingPage(null)
  }

  return (
    <div className="flex h-screen">
      <nav className="w-16 border-r bg-muted/30 flex flex-col items-center py-4 gap-2">
        {navItems.map((item) => (
          <button key={item.id} onClick={() => trySetPage(item.id)}
            className={cn("flex flex-col items-center gap-1 rounded-md p-2 text-xs transition-colors w-14",
              page === item.id ? "bg-primary text-primary-foreground" : "hover:bg-muted text-muted-foreground")}>
            {item.icon}
            <span>{item.label}</span>
          </button>
        ))}
      </nav>
      <main className="flex-1 overflow-auto p-6">
        {page === "processes" && <Processes />}
        {page === "config" && <Config />}
        {page === "bridge" && <Bridge />}
        {page === "channels" && <Channels />}
        {page === "logs" && <Logs />}
      </main>
      <WechatQrDialog />
      <Toaster />
      <AlertDialog open={pendingPage !== null} onOpenChange={(o) => { if (!o) setPendingPage(null) }}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>有未保存的修改</AlertDialogTitle>
            <AlertDialogDescription>是否保存当前配置后再离开？</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={() => setPendingPage(null)}>取消</AlertDialogCancel>
            <AlertDialogAction onClick={handleDiscardAndLeave}>不保存</AlertDialogAction>
            <AlertDialogAction onClick={handleSaveAndLeave}>保存</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
```

- [ ] **Step 2: 验证类型检查通过**

Run: `npm run build`
Expected: 构建成功

- [ ] **Step 3: 提交**

```bash
git add src/App.tsx
git commit -m "feat: wrap ConfigProvider, block page switch on dirty with AlertDialog"
```

---

### Task 13: 最终构建验证 + Rust 测试

**Files:** 无（仅验证）

- [ ] **Step 1: 运行 Rust 测试**

Run: `cargo test`（在 `src-tauri/` 内）
Expected: 全部通过（log_buffer 测试已更新带 seq）

- [ ] **Step 2: 运行前端构建**

Run: `npm run build`
Expected: tsc + vite build 成功，无类型错误

- [ ] **Step 3: 手动验证（可选，若在 GUI 环境）**

Run: `npm run tauri dev`
验证：
- 切换到「配置」页，改一个端口，尝试切到「进程」页 → 应弹 AlertDialog
- 改端口为 0 或重复端口 → 失焦应显示红字，保存按钮禁用
- 点「删除」server → 应弹确认 AlertDialog
- 切到「日志」页，暂停 → 应显示「已暂停」标记
- 日志切换 Tab 不再错位（key=seq 稳定）

如无 GUI 环境，Step 1-2 已足够验证正确性。

---

## 自审

**1. 规格覆盖：**
- 日志渲染与性能（key/filtered/暂停提示）→ Task 7 ✓
- 加载/错误态一致（useAsync + ConfigProvider + StateView）→ Task 3, 4, 5, 8, 9, 10, 11 ✓
- dirty 跟踪与未保存提示 → Task 4（isDirty/save/reset）, 8/9/10（横幅）, 12（切页阻断）✓
- 端口输入校验 → Task 8 ✓
- 删除确认 → Task 8 ✓
- 保存后联动刷新 → Task 4（ConfigProvider 单例）, 11（Processes 用 useConfig）✓
- LogEntry.seq 后端 + 前端同步 → Task 1, 2 ✓

**2. 占位符扫描：** 无 TBD/TODO/"添加合适处理"；每步有完整代码或确切命令。✓

**3. 类型一致性：**
- `useConfig` 的 `save()` 在 Task 4 定义为 `Promise<boolean>`，Task 8/9/10/12 调用 `.then((ok) => ...)` 一致 ✓
- `useAsync` 的 `reload` 在 Task 3 定义为 `() => void`，Task 9 调用 `depsReload` 传入 StateView 的 `onRetry` 一致 ✓
- `LogEntry.seq` 在 Task 1（Rust `u64`）与 Task 2（TS `number`）一致，serde camelCase → `seq` ✓
- shadcn AlertDialog 组件名在 Task 6 生成与 Task 8/12 引用一致 ✓

无遗漏，无占位符，类型一致。计划就绪。
