import { useEffect, useState } from "react"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog"
import { useTauriEvent } from "@/hooks/useTauriEvent"
import QRCode from "qrcode"
import type { WechatQrEvent } from "@/lib/types"

export function WechatQrDialog() {
  const [open, setOpen] = useState(false)
  const [qrData, setQrData] = useState<WechatQrEvent | null>(null)
  const [qrUrl, setQrUrl] = useState<string>("")

  useTauriEvent<WechatQrEvent>("wechat://qrcode", (ev) => {
    setQrData(ev)
    setOpen(true)
  })

  useTauriEvent("wechat://logined", () => {
    setOpen(false)
    setQrData(null)
  })

  useEffect(() => {
    if (qrData?.kind === "url") {
      QRCode.toDataURL(qrData.data, { width: 256 }).then(setQrUrl).catch(() => {})
    } else {
      setQrUrl("")
    }
  }, [qrData])

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>微信扫码登录</DialogTitle>
          <DialogDescription>请使用微信扫描下方二维码完成登录</DialogDescription>
        </DialogHeader>
        <div className="flex justify-center p-4">
          {qrData?.kind === "url" && qrUrl ? (
            <img src={qrUrl} alt="QR Code" className="h-64 w-64" />
          ) : qrData?.kind === "ascii" ? (
            <pre className="font-mono text-[6px] leading-[6px] whitespace-pre">{qrData.data}</pre>
          ) : (
            <div className="h-64 w-64 animate-pulse bg-muted" />
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}
