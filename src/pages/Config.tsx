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
