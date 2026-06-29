import { useEffect, useState } from "react"
import { ProcessCard } from "@/components/ProcessCard"
import { useProcessState } from "@/hooks/useProcessState"
import { getConfig } from "@/lib/tauri"
import type { AppConfig } from "@/lib/types"

export function Processes() {
  const { state, refresh } = useProcessState()
  const [config, setConfig] = useState<AppConfig | null>(null)

  useEffect(() => { getConfig().then(setConfig).then(refresh) }, [refresh])

  if (!config) return <div>加载中...</div>

  return (
    <div className="grid grid-cols-2 gap-4">
      {state.servers.map((s) => (
        <ProcessCard key={s.id} target="server" state={s.state} serverId={s.id} name={s.name} />
      ))}
      <ProcessCard target="bridge" state={state.bridge} servers={config.servers} boundServerId={config.bridge.boundServerId} onConfigUpdate={() => getConfig().then(setConfig)} />
    </div>
  )
}
