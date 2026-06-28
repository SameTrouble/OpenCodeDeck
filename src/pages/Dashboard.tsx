import { useEffect } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { ProcessCard } from "@/components/ProcessCard"
import { LogView } from "@/components/LogView"
import { useProcessState } from "@/hooks/useProcessState"
import { startAll, stopAll, restartAll } from "@/lib/tauri"
import { toast } from "sonner"

export function Dashboard() {
  const { state, refresh } = useProcessState()

  useEffect(() => { refresh() }, [refresh])

  return (
    <div className="space-y-4">
      <div className="flex gap-2">
        <Button onClick={() => startAll().catch((e) => toast.error(`启动失败: ${e}`))}>启动全部</Button>
        <Button variant="outline" onClick={() => stopAll().catch((e) => toast.error(`停止失败: ${e}`))}>停止全部</Button>
        <Button variant="outline" onClick={() => restartAll().catch((e) => toast.error(`重启失败: ${e}`))}>重启全部</Button>
      </div>
      <div className="grid grid-cols-2 gap-4">
        <ProcessCard target="server" state={state.server} />
        <ProcessCard target="bridge" state={state.bridge} />
      </div>
      <Card>
        <CardHeader><CardTitle className="text-sm">最近日志</CardTitle></CardHeader>
        <CardContent><LogView height="200px" /></CardContent>
      </Card>
    </div>
  )
}
