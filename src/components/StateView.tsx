import type { ReactNode } from "react"
import type { AppError } from "@/lib/types"
import { formatError } from "@/lib/utils"

interface StateViewProps {
  loading: boolean
  error: AppError | null
  onRetry?: () => void
  children: ReactNode
}

export function StateView({ loading, error, onRetry, children }: StateViewProps) {
  if (loading) {
    return <div className="text-muted-foreground text-sm">加载中…</div>
  }
  if (error) {
    return (
      <div className="flex flex-col items-center gap-2 py-8 text-center">
        <p className="text-sm text-destructive">{formatError(error)}</p>
        {onRetry && (
          <button className="rounded bg-primary px-3 py-1 text-xs text-primary-foreground" onClick={onRetry}>
            重试
          </button>
        )}
      </div>
    )
  }
  return <>{children}</>
}
