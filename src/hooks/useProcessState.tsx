import { createContext, useContext, useState, useCallback, type ReactNode } from "react"
import { useTauriEvent } from "./useTauriEvent"
import { getState } from "../lib/tauri"
import type { FullState, ProcessTarget } from "../lib/types"

const initial: FullState = {
  server: { state: "Stopped", pid: null, startedAt: null, uptimeSec: null, exitCode: null, healthy: null },
  bridge: { state: "Stopped", pid: null, startedAt: null, uptimeSec: null, exitCode: null, healthy: null },
}

const ProcessStateContext = createContext<{ state: FullState; refresh: () => void }>({
  state: initial,
  refresh: () => {},
})

export function ProcessStateProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<FullState>(initial)

  useTauriEvent<{ target: ProcessTarget; state: FullState["server"] }>("state://update", ({ target, state: ps }) => {
    setState((prev) => ({ ...prev, [target]: ps }))
  })

  const refresh = useCallback(() => { getState().then(setState).catch(() => {}) }, [])

  return (
    <ProcessStateContext.Provider value={{ state, refresh }}>
      {children}
    </ProcessStateContext.Provider>
  )
}

export function useProcessState() {
  return useContext(ProcessStateContext)
}
