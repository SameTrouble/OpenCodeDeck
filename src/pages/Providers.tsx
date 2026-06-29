import { useState } from "react"
import { Plus, Pencil, Trash2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import {
  AlertDialog, AlertDialogAction, AlertDialogCancel,
  AlertDialogContent, AlertDialogDescription, AlertDialogFooter,
  AlertDialogHeader, AlertDialogTitle,
} from "@/components/ui/alert-dialog"
import { StateView } from "@/components/StateView"
import { useOpencodeConfig } from "@/hooks/useOpencodeConfig"
import { toast } from "sonner"
import type { ProviderConfig } from "@/lib/opencode-types"
import { ProviderDialog } from "@/components/provider-editor/ProviderDialog"

function genId() {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8)
}

export function Providers() {
  const { config, loading, error, reload, update, isDirty, save, reset } = useOpencodeConfig()
  const [deleteId, setDeleteId] = useState<string | null>(null)
  const [editId, setEditId] = useState<string | null>(null)

  if (loading || !config) return <StateView loading={loading} error={error} onRetry={reload}>{null}</StateView>

  const providers = config.provider ?? {}
  const entries = Object.entries(providers)

  const addProvider = () => {
    const id = genId()
    update((draft) => {
      if (!draft.provider) draft.provider = {}
      draft.provider[id] = { name: "新供应商", npm: "@ai-sdk/openai", options: {}, models: {} }
    })
  }

  const removeProvider = (id: string) => {
    update((draft) => {
      if (draft.provider) delete draft.provider[id]
    })
    setDeleteId(null)
  }

  const confirmEdit = (patch: ProviderConfig) => {
    if (!editId) return
    update((draft) => {
      if (draft.provider && draft.provider[editId]) {
        draft.provider[editId] = patch
      }
    })
    setEditId(null)
  }

  const handleSave = () => {
    save().then((ok) => {
      if (ok) toast.success("已保存")
      else toast.error("保存失败")
    })
  }

  return (
    <div className="space-y-4">
      <StateView loading={false} error={error} onRetry={reload}>
        {isDirty && (
          <div className="flex items-center justify-between rounded-md border border-yellow-500/50 bg-yellow-500/10 px-3 py-2 text-xs text-yellow-700 dark:text-yellow-400">
            <span>未保存的修改</span>
            <Button size="sm" variant="ghost" onClick={reset}>放弃修改</Button>
          </div>
        )}
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold">模型供应商</h2>
          <Button variant="outline" size="sm" onClick={addProvider}>
            <Plus className="h-4 w-4 mr-1" /> 添加供应商
          </Button>
        </div>
        {entries.length === 0 ? (
          <Card>
            <CardContent className="py-12 text-center text-sm text-muted-foreground">
              暂无供应商，点击「添加供应商」创建第一个
            </CardContent>
          </Card>
        ) : (
          <div className="space-y-3">
            {entries.map(([id, p]: [string, ProviderConfig]) => {
              const modelCount = p.models ? Object.keys(p.models).length : 0
              return (
                <Card key={id}>
                  <CardContent className="flex items-center justify-between py-4">
                    <div className="space-y-1">
                      <div className="font-medium">{p.name || "(未命名)"}</div>
                      <div className="text-xs text-muted-foreground">
                        <span className="font-mono">{id}</span>
                        {p.npm && <span className="ml-2">{p.npm}</span>}
                        <span className="ml-2">{modelCount} 个模型</span>
                      </div>
                    </div>
                    <div className="flex gap-2">
                      <Button variant="outline" size="sm" onClick={() => setEditId(id)}>
                        <Pencil className="h-4 w-4 mr-1" /> 编辑
                      </Button>
                      <AlertDialog open={deleteId === id} onOpenChange={(o) => setDeleteId(o ? id : null)}>
                        <Button variant="destructive" size="sm" onClick={() => setDeleteId(id)}>
                          <Trash2 className="h-4 w-4 mr-1" /> 删除
                        </Button>
                        <AlertDialogContent>
                          <AlertDialogHeader>
                            <AlertDialogTitle>删除该供应商？</AlertDialogTitle>
                            <AlertDialogDescription>此操作不可撤销。</AlertDialogDescription>
                          </AlertDialogHeader>
                          <AlertDialogFooter>
                            <AlertDialogCancel>取消</AlertDialogCancel>
                            <AlertDialogAction onClick={() => removeProvider(id)}>确认删除</AlertDialogAction>
                          </AlertDialogFooter>
                        </AlertDialogContent>
                      </AlertDialog>
                    </div>
                  </CardContent>
                </Card>
              )
            })}
          </div>
        )}
        <Button onClick={handleSave} disabled={!isDirty}>保存</Button>
        {editId && config.provider?.[editId] && (
          <ProviderDialog
            providerId={editId}
            provider={config.provider[editId]}
            open={editId !== null}
            onOpenChange={(o) => { if (!o) setEditId(null) }}
            onConfirm={confirmEdit}
          />
        )}
      </StateView>
    </div>
  )
}
