import { useEffect, useRef, useState } from "react"
import { Button } from "@/components/ui/button"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Pause, Play, Trash2, Download } from "lucide-react"
import { useTauriEvent } from "@/hooks/useTauriEvent"
import { getLogHistory, clearLogs, exportLogs } from "@/lib/tauri"
import type { LogEntry } from "@/lib/types"
import { toast } from "sonner"
import { formatError } from "@/lib/utils"

export function LogView({ height = "400px" }: { height?: string }) {
  const [entries, setEntries] = useState<LogEntry[]>([])
  const [paused, setPaused] = useState(false)
  const [activeTab, setActiveTab] = useState<"all" | "server" | "bridge">("all")
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    getLogHistory("all", 500).then(setEntries).catch((e) => console.error("[load log history]", e))
  }, [])

  useTauriEvent<LogEntry>("log://entry", (entry) => {
    if (paused) return
    setEntries((prev) => {
      const next = [...prev, entry]
      return next.length > 1000 ? next.slice(-1000) : next
    })
  })

  useEffect(() => {
    if (!paused) bottomRef.current?.scrollIntoView({ behavior: "smooth" })
  }, [entries, paused])

  const filtered = activeTab === "all" ? entries : entries.filter((e) => e.source === activeTab)

  const handleClear = () => {
    clearLogs(activeTab).then(() => {
      setEntries((prev) => activeTab === "all" ? [] : prev.filter((e) => e.source !== activeTab))
    }).catch((e) => toast.error(`清空失败: ${formatError(e)}`))
  }

  const handleExport = () => {
    exportLogs(activeTab).then((path) => toast.success(`已导出到: ${path}`)).catch((e) => toast.error(`导出失败: ${formatError(e)}`))
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as typeof activeTab)}>
          <TabsList>
            <TabsTrigger value="all">全部</TabsTrigger>
            <TabsTrigger value="server">Server</TabsTrigger>
            <TabsTrigger value="bridge">Bridge</TabsTrigger>
          </TabsList>
        </Tabs>
        <div className="flex gap-1">
          <Button size="sm" variant="ghost" onClick={() => setPaused((p) => !p)}>
            {paused ? <Play className="h-3 w-3" /> : <Pause className="h-3 w-3" />}
          </Button>
          <Button size="sm" variant="ghost" onClick={handleClear}><Trash2 className="h-3 w-3" /></Button>
          <Button size="sm" variant="ghost" onClick={handleExport}><Download className="h-3 w-3" /></Button>
        </div>
      </div>
      <div className={`overflow-auto rounded border bg-muted/30 p-2 font-mono text-xs`} style={{ height }}>
        {filtered.map((e, i) => (
          <div key={i} className={e.level === "error" ? "text-red-500" : "text-foreground"}>
            <span className="text-muted-foreground">[{new Date(e.ts * 1000).toLocaleTimeString()}]</span>{" "}
            <span className="text-muted-foreground">[{e.source}]</span>{" "}
            {e.line}
          </div>
        ))}
        <div ref={bottomRef} />
      </div>
    </div>
  )
}
