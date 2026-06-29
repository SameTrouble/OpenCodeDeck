import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Play, Square, RotateCcw } from "lucide-react"
import type { ProcessState, ProcessTarget, ServerConfig } from "@/lib/types"
import { startProcess, stopProcess, restartProcess, bindBridge } from "@/lib/tauri"
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

  const handleStart = () => startProcess(target, serverId).catch((e) => toast.error(`启动失败: ${formatError(e)}`))
  const handleStop = () => stopProcess(target, serverId).catch((e) => toast.error(`停止失败: ${formatError(e)}`))
  const handleRestart = () => restartProcess(target, serverId).catch((e) => toast.error(`重启失败: ${formatError(e)}`))
  const handleBind = (newId: string) => bindBridge(newId).catch((e) => toast.error(`绑定失败: ${formatError(e)}`))

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
