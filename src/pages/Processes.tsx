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
