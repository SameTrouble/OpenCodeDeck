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
