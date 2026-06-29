import { useState, useEffect } from "react"
import {
  Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs"
import { ProviderBasicTab } from "./ProviderBasicTab"
import type { ProviderConfig } from "@/lib/opencode-types"

interface Props {
  providerId: string
  provider: ProviderConfig
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: (patch: ProviderConfig) => void
}

export function ProviderDialog({ providerId, provider, open, onOpenChange, onConfirm }: Props) {
  const [local, setLocal] = useState<ProviderConfig>(provider)

  useEffect(() => {
    if (open) setLocal(provider)
  }, [open, provider])

  const handleChange = (patch: Partial<ProviderConfig>) => {
    setLocal((prev) => ({ ...prev, ...patch }))
  }

  const handleConfirm = () => {
    onConfirm(local)
    onOpenChange(false)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>编辑供应商: {providerId}</DialogTitle>
        </DialogHeader>
        <Tabs defaultValue="basic">
          <TabsList>
            <TabsTrigger value="basic">基础</TabsTrigger>
            <TabsTrigger value="models">模型</TabsTrigger>
          </TabsList>
          <TabsContent value="basic" className="py-4">
            <ProviderBasicTab provider={local} onChange={handleChange} />
          </TabsContent>
          <TabsContent value="models" className="py-4">
            <p className="text-sm text-muted-foreground">模型编辑将在后续任务实现</p>
          </TabsContent>
        </Tabs>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={handleConfirm}>确认</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
