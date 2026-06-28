import { useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { getConfig, saveConfig } from "@/lib/tauri"
import type { AppConfig } from "@/lib/types"
import { toast } from "sonner"
import { formatError } from "@/lib/utils"

export function Config() {
  const [config, setConfig] = useState<AppConfig | null>(null)

  useEffect(() => { getConfig().then(setConfig).catch((e) => toast.error(`加载配置失败: ${formatError(e)}`)) }, [])

  if (!config) return <div>加载中...</div>

  const save = () => saveConfig(config).then(() => toast.success("已保存")).catch((e) => toast.error(`保存失败: ${formatError(e)}`))

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
