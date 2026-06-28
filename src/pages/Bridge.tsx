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
