import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { ProcessCard } from "@/components/ProcessCard"
import { LogView } from "@/components/LogView"
import { useProcessState } from "@/hooks/useProcessState"
import { useEffect, useState } from "react"
import { getConfig } from "@/lib/tauri"
import type { AppConfig } from "@/lib/types"

export function Dashboard() {
  const { state, refresh } = useProcessState()
  const [config, setConfig] = useState<AppConfig | null>(null)

  useEffect(() => { getConfig().then(setConfig).then(refresh) }, [refresh])

  if (!config) return <div>加载中...</div>

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-4">
        {state.servers.map((s) => (
          <ProcessCard key={s.id} target="server" state={s.state} serverId={s.id} name={s.name} />
        ))}
        <ProcessCard target="bridge" state={state.bridge} servers={config.servers} boundServerId={config.bridge.boundServerId} />
      </div>
      <Card>
        <CardHeader><CardTitle className="text-sm">最近日志</CardTitle></CardHeader>
        <CardContent><LogView height="200px" /></CardContent>
      </Card>
    </div>
  )
}
