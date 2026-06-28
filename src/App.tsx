import { useState } from "react"
import { LayoutDashboard, Cpu, Settings, Boxes, Radio, ScrollText } from "lucide-react"
import { Dashboard } from "@/pages/Dashboard"
import { Processes } from "@/pages/Processes"
import { Config } from "@/pages/Config"
import { Bridge } from "@/pages/Bridge"
import { Channels } from "@/pages/Channels"
import { Logs } from "@/pages/Logs"
import { WechatQrDialog } from "@/components/WechatQrDialog"
import { Toaster } from "@/components/ui/sonner"
import { cn } from "@/lib/utils"

type Page = "dashboard" | "processes" | "config" | "bridge" | "channels" | "logs"

const navItems: { id: Page; label: string; icon: React.ReactNode }[] = [
  { id: "dashboard", label: "仪表盘", icon: <LayoutDashboard className="h-4 w-4" /> },
  { id: "processes", label: "进程", icon: <Cpu className="h-4 w-4" /> },
  { id: "config", label: "配置", icon: <Settings className="h-4 w-4" /> },
  { id: "bridge", label: "Bridge", icon: <Boxes className="h-4 w-4" /> },
  { id: "channels", label: "渠道", icon: <Radio className="h-4 w-4" /> },
  { id: "logs", label: "日志", icon: <ScrollText className="h-4 w-4" /> },
]

export default function App() {
  const [page, setPage] = useState<Page>("dashboard")

  return (
    <div className="flex h-screen">
      <nav className="w-16 border-r bg-muted/30 flex flex-col items-center py-4 gap-2">
        {navItems.map((item) => (
          <button key={item.id} onClick={() => setPage(item.id)}
            className={cn("flex flex-col items-center gap-1 rounded-md p-2 text-xs transition-colors w-14",
              page === item.id ? "bg-primary text-primary-foreground" : "hover:bg-muted text-muted-foreground")}>
            {item.icon}
            <span>{item.label}</span>
          </button>
        ))}
      </nav>
      <main className="flex-1 overflow-auto p-6">
        {page === "dashboard" && <Dashboard />}
        {page === "processes" && <Processes />}
        {page === "config" && <Config />}
        {page === "bridge" && <Bridge />}
        {page === "channels" && <Channels />}
        {page === "logs" && <Logs />}
      </main>
      <WechatQrDialog />
      <Toaster />
    </div>
  )
}
