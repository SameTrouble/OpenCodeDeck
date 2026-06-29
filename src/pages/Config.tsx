import { useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { getConfig, saveConfig } from "@/lib/tauri"
import type { AppConfig, ServerConfig } from "@/lib/types"
import { toast } from "sonner"
import { formatError } from "@/lib/utils"

function genId() {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8)
}

export function Config() {
  const [config, setConfig] = useState<AppConfig | null>(null)

  useEffect(() => { getConfig().then(setConfig).catch((e) => toast.error(`加载配置失败: ${formatError(e)}`)) }, [])

  if (!config) return <div>加载中...</div>

  const updateServer = (id: string, patch: Partial<ServerConfig>) => {
    setConfig({
      ...config,
      servers: config.servers.map((s) => s.id === id ? { ...s, ...patch } : s),
    })
  }
  const addServer = () => {
    const newServer: ServerConfig = { id: genId(), name: "新 server", url: "http://127.0.0.1:4097", cwd: "", extraEnv: {} }
    setConfig({ ...config, servers: [...config.servers, newServer] })
  }
  const removeServer = (id: string) => {
    setConfig({ ...config, servers: config.servers.filter((s) => s.id !== id) })
  }
  const save = () => saveConfig(config).then(() => toast.success("已保存")).catch((e) => toast.error(`保存失败: ${formatError(e)}`))

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader><CardTitle>opencode servers</CardTitle></CardHeader>
        <CardContent className="space-y-4">
          {config.servers.map((s) => (
            <div key={s.id} className="space-y-2 border-b pb-4">
              <div className="flex items-center justify-between">
                <Label className="text-xs text-muted-foreground">ID: {s.id}</Label>
                <Button size="sm" variant="destructive" onClick={() => removeServer(s.id)}>删除</Button>
              </div>
              <div className="grid grid-cols-2 gap-2">
                <div className="space-y-1">
                  <Label>名称</Label>
                  <Input value={s.name} onChange={(e) => updateServer(s.id, { name: e.target.value })} />
                </div>
                <div className="space-y-1">
                  <Label>URL</Label>
                  <Input value={s.url} onChange={(e) => updateServer(s.id, { url: e.target.value })} />
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
      <Button onClick={save}>保存</Button>
    </div>
  )
}
