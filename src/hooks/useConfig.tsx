import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react"
import { getConfig, saveConfig } from "@/lib/tauri"
import type { AppConfig, AppError } from "@/lib/types"

interface ConfigCtx {
  config: AppConfig | null
  baseline: AppConfig | null
  loading: boolean
  error: AppError | null
  reload: () => void
  update: (patch: (draft: AppConfig) => void) => void
  isDirty: boolean
  save: () => Promise<boolean>
  reset: () => void
}

const Ctx = createContext<ConfigCtx>({
  config: null,
  baseline: null,
  loading: true,
  error: null,
  reload: () => {},
  update: () => {},
  isDirty: false,
  save: () => Promise.resolve(false),
  reset: () => {},
})

export function ConfigProvider({ children }: { children: ReactNode }) {
  const [baseline, setBaseline] = useState<AppConfig | null>(null)
  const [draft, setDraft] = useState<AppConfig | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<AppError | null>(null)
  const reqIdRef = useRef(0)

  const reload = useCallback(() => {
    const myId = ++reqIdRef.current
    setLoading(true)
    setError(null)
    getConfig()
      .then((cfg) => {
        if (myId !== reqIdRef.current) return
        setBaseline(cfg)
        setDraft(cfg)
        setLoading(false)
      })
      .catch((e: unknown) => {
        if (myId !== reqIdRef.current) return
        const err: AppError = e && typeof e === "object" && "kind" in e
          ? (e as AppError)
          : { kind: "Config", message: String(e) }
        setError(err)
        setLoading(false)
      })
  }, [])

  useEffect(() => { reload() }, [reload])

  const update = useCallback((patch: (draft: AppConfig) => void) => {
    setDraft((prev) => {
      if (!prev) return prev
      const next = structuredClone(prev)
      patch(next)
      return next
    })
  }, [])

  const isDirty = baseline !== null && draft !== null
    && JSON.stringify(draft) !== JSON.stringify(baseline)

  const save = useCallback(async () => {
    if (!draft) return false
    try {
      await saveConfig(draft)
      setBaseline(draft)
      return true
    } catch (e: unknown) {
      const err: AppError = e && typeof e === "object" && "kind" in e
        ? (e as AppError)
        : { kind: "Config", message: String(e) }
      setError(err)
      return false
    }
  }, [draft])

  const reset = useCallback(() => {
    setDraft(baseline)
  }, [baseline])

  return (
    <Ctx.Provider value={{ config: draft, baseline, loading, error, reload, update, isDirty, save, reset }}>
      {children}
    </Ctx.Provider>
  )
}

export function useConfig() {
  return useContext(Ctx)
}
