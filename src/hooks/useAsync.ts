import { useCallback, useEffect, useRef, useState } from "react"
import type { AppError } from "@/lib/types"

interface UseAsyncResult<T> {
  data: T | null
  loading: boolean
  error: AppError | null
  reload: () => void
}

export function useAsync<T>(fn: () => Promise<T>, deps: unknown[]): UseAsyncResult<T> {
  const [data, setData] = useState<T | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<AppError | null>(null)
  const fnRef = useRef(fn)
  fnRef.current = fn
  const reqIdRef = useRef(0)

  const reload = useCallback(() => {
    const myId = ++reqIdRef.current
    setLoading(true)
    setError(null)
    fnRef.current()
      .then((result) => {
        if (myId !== reqIdRef.current) return
        setData(result)
        setLoading(false)
      })
      .catch((e: unknown) => {
        if (myId !== reqIdRef.current) return
        const err: AppError = e && typeof e === "object" && "kind" in e
          ? (e as AppError)
          : { kind: "Process", message: String(e) }
        setError(err)
        setLoading(false)
      })
  }, [])

  useEffect(() => {
    reload()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps)

  return { data, loading, error, reload }
}
