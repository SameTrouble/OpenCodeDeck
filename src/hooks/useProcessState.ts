import { useState, useCallback } from "react"
import { useTauriEvent } from "./useTauriEvent"
import { getState } from "../lib/tauri"
import type { FullState, ProcessTarget } from "../lib/types"

export function useProcessState() {
  const [state, setState] = useState<FullState>({ server: { state: "Stopped", pid: null, startedAt: null, uptimeSec: null, exitCode: null, healthy: null }, bridge: { state: "Stopped", pid: null, startedAt: null, uptimeSec: null, exitCode: null, healthy: null } })

  useTauriEvent<{ target: ProcessTarget; state: FullState["server"] }>("state://update", ({ target, state: ps }) => {
    setState((prev) => ({ ...prev, [target]: ps }))
  })

  const refresh = useCallback(() => { getState().then(setState).catch(() => {}) }, [])

  return { state, refresh }
}
