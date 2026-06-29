import { createContext, useContext, useState, useCallback, type ReactNode } from "react"
import { useTauriEvent } from "./useTauriEvent"
import { getState } from "../lib/tauri"
import type { FullState, ProcessState, ProcessTarget } from "../lib/types"

const stoppedState: ProcessState = { state: "Stopped", pid: null, startedAt: null, uptimeSec: null, exitCode: null, healthy: null }

const initial: FullState = {
  servers: [],
  bridge: stoppedState,
}

const ProcessStateContext = createContext<{ state: FullState; refresh: () => void }>({
  state: initial,
  refresh: () => {},
})

export function ProcessStateProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<FullState>(initial)

  useTauriEvent<{ target: ProcessTarget; serverId: string | null; state: ProcessState }>("state://update", ({ target, serverId, state: ps }) => {
    setState((prev) => {
      if (target === "bridge") {
        return { ...prev, bridge: ps }
      }
      if (serverId) {
        const servers = prev.servers.map((s) => s.id === serverId ? { ...s, state: ps } : s)
        return { ...prev, servers }
      }
      return prev
    })
  })

  useTauriEvent<{ target: string; serverId: string; healthy: boolean }>("health://update", ({ serverId, healthy }) => {
    setState((prev) => {
      const servers = prev.servers.map((s) => s.id === serverId ? { ...s, state: { ...s.state, healthy } } : s)
      return { ...prev, servers }
    })
  })

  const refresh = useCallback(() => {
    getState().then(setState).catch((e) => console.error("[refresh process state]", e))
  }, [])

  return (
    <ProcessStateContext.Provider value={{ state, refresh }}>
      {children}
    </ProcessStateContext.Provider>
  )
}

export function useProcessState() {
  return useContext(ProcessStateContext)
}
