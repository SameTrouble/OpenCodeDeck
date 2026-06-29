import { useEffect, useState } from "react"
import { Cpu, Settings, Boxes, Radio, ScrollText, Layers } from "lucide-react"
import { Processes } from "@/pages/Processes"
import { Config } from "@/pages/Config"
import { Bridge } from "@/pages/Bridge"
import { Channels } from "@/pages/Channels"
import { Logs } from "@/pages/Logs"
import { Providers } from "@/pages/Providers"
import { WechatQrDialog } from "@/components/WechatQrDialog"
import {
  AlertDialog, AlertDialogAction, AlertDialogCancel,
  AlertDialogContent, AlertDialogDescription, AlertDialogFooter,
  AlertDialogHeader, AlertDialogTitle,
} from "@/components/ui/alert-dialog"
import { Toaster } from "@/components/ui/sonner"
import { ProcessStateProvider, useProcessState } from "@/hooks/useProcessState"
import { ConfigProvider, useConfig } from "@/hooks/useConfig"
import { OpencodeConfigProvider, useOpencodeConfig } from "@/hooks/useOpencodeConfig"
import { toast } from "sonner"
import { cn } from "@/lib/utils"

type Page = "processes" | "config" | "providers" | "bridge" | "channels" | "logs"

const navItems: { id: Page; label: string; icon: React.ReactNode }[] = [
  { id: "processes", label: "进程", icon: <Cpu className="h-4 w-4" /> },
  { id: "config", label: "配置", icon: <Settings className="h-4 w-4" /> },
  { id: "providers", label: "供应商", icon: <Layers className="h-4 w-4" /> },
  { id: "bridge", label: "Bridge", icon: <Boxes className="h-4 w-4" /> },
  { id: "channels", label: "渠道", icon: <Radio className="h-4 w-4" /> },
  { id: "logs", label: "日志", icon: <ScrollText className="h-4 w-4" /> },
]

export default function App() {
  return (
    <ConfigProvider>
      <OpencodeConfigProvider>
        <ProcessStateProvider>
          <AppInner />
        </ProcessStateProvider>
      </OpencodeConfigProvider>
    </ConfigProvider>
  )
}

function AppInner() {
  const [page, setPage] = useState<Page>("processes")
  const { refresh } = useProcessState()

  useEffect(() => { refresh() }, [refresh])

  return <AppContent page={page} setPage={setPage} />
}

function AppContent({ page, setPage }: { page: Page; setPage: (p: Page) => void }) {
  const { isDirty: configDirty, save: saveConfig, reset: resetConfig } = useConfig()
  const { isDirty: opencodeDirty, save: saveOpencode, reset: resetOpencode } = useOpencodeConfig()
  const [pendingPage, setPendingPage] = useState<Page | null>(null)

  const trySetPage = (next: Page) => {
    if (configDirty || opencodeDirty) setPendingPage(next)
    else setPage(next)
  }

  const handleSaveAndLeave = () => {
    Promise.all([saveConfig(), saveOpencode()]).then(([ok1, ok2]) => {
      if (ok1 && ok2 && pendingPage) setPage(pendingPage)
      else if (!ok1 || !ok2) toast.error("保存失败")
      setPendingPage(null)
    })
  }

  const handleDiscardAndLeave = () => {
    resetConfig()
    resetOpencode()
    if (pendingPage) setPage(pendingPage)
    setPendingPage(null)
  }

  return (
    <div className="flex h-screen">
      <nav className="w-16 border-r bg-muted/30 flex flex-col items-center py-4 gap-2">
        {navItems.map((item) => (
          <button key={item.id} onClick={() => trySetPage(item.id)}
            className={cn("flex flex-col items-center gap-1 rounded-md p-2 text-xs transition-colors w-14",
              page === item.id ? "bg-primary text-primary-foreground" : "hover:bg-muted text-muted-foreground")}>
            {item.icon}
            <span>{item.label}</span>
          </button>
        ))}
      </nav>
      <main className="flex-1 overflow-auto p-6">
        {page === "processes" && <Processes />}
        {page === "config" && <Config />}
        {page === "providers" && <Providers />}
        {page === "bridge" && <Bridge />}
        {page === "channels" && <Channels />}
        {page === "logs" && <Logs />}
      </main>
      <WechatQrDialog />
      <Toaster />
      <AlertDialog open={pendingPage !== null} onOpenChange={(o) => { if (!o) setPendingPage(null) }}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>有未保存的修改</AlertDialogTitle>
            <AlertDialogDescription>是否保存当前配置后再离开？</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={() => setPendingPage(null)}>取消</AlertDialogCancel>
            <AlertDialogAction onClick={handleDiscardAndLeave}>不保存</AlertDialogAction>
            <AlertDialogAction onClick={handleSaveAndLeave}>保存</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
