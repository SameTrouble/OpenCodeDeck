import { ProcessCard } from "@/components/ProcessCard"
import { useProcessState } from "@/hooks/useProcessState"

export function Processes() {
  const { state } = useProcessState()
  return (
    <div className="grid grid-cols-2 gap-4">
      <ProcessCard target="server" state={state.server} />
      <ProcessCard target="bridge" state={state.bridge} />
    </div>
  )
}
