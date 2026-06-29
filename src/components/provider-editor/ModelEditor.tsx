import { useState } from "react"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { Checkbox } from "@/components/ui/checkbox"
import { Button } from "@/components/ui/button"
import {
  Select, SelectContent, SelectItem, SelectTrigger, SelectValue,
} from "@/components/ui/select"
import {
  Collapsible, CollapsibleContent, CollapsibleTrigger,
} from "@/components/ui/collapsible"
import { ChevronDown, Trash2 } from "lucide-react"
import type { ModelConfig, Modality, ModelStatus } from "@/lib/opencode-types"

const MODALITIES: Modality[] = ["text", "audio", "image", "video", "pdf"]
const STATUSES: ModelStatus[] = ["active", "beta", "alpha", "deprecated"]

interface Props {
  modelId: string
  model: ModelConfig
  onChange: (patch: Partial<ModelConfig>) => void
  onDelete: () => void
}

export function ModelEditor({ modelId, model, onChange, onDelete }: Props) {
  const [open, setOpen] = useState(true)
  const [costOpen, setCostOpen] = useState(false)

  const limit = model.limit ?? { context: 0, output: 0 }
  const setLimit = (key: "context" | "input" | "output", value: number | undefined) => {
    if (value === undefined) {
      const next = { ...limit }
      delete next.input
      onChange({ limit: next })
    } else {
      onChange({ limit: { ...limit, [key]: value } })
    }
  }

  const cost = model.cost
  const setCost = (key: string, value: string) => {
    const num = value === "" ? undefined : Number(value)
    onChange({ cost: { ...(cost ?? { input: 0, output: 0 }), [key]: num } })
  }

  const toggleModality = (dir: "input" | "output", m: Modality) => {
    const current = model.modalities?.[dir] ?? []
    const has = current.includes(m)
    const next = has ? current.filter((x) => x !== m) : [...current, m]
    onChange({ modalities: { ...(model.modalities ?? {}), [dir]: next } })
  }

  const contextOutput = `${limit.context}/${limit.output}`

  return (
    <Collapsible open={open} onOpenChange={setOpen} className="rounded-md border">
      <div className="flex items-center justify-between px-3 py-2">
        <CollapsibleTrigger asChild>
          <button className="flex items-center gap-2 text-sm font-medium">
            <ChevronDown className={`h-4 w-4 transition-transform ${open ? "rotate-180" : ""}`} />
            <span className="font-mono">{modelId}</span>
            <span className="text-xs text-muted-foreground">{contextOutput}</span>
          </button>
        </CollapsibleTrigger>
        <Button variant="ghost" size="icon" onClick={onDelete} className="h-7 w-7 text-destructive">
          <Trash2 className="h-4 w-4" />
        </Button>
      </div>
      <CollapsibleContent className="space-y-4 px-3 pb-4">
        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-1">
            <Label>显示名称</Label>
            <Input value={model.name ?? ""} onChange={(e) => onChange({ name: e.target.value })} />
          </div>
          <div className="space-y-1">
            <Label>家族</Label>
            <Input value={model.family ?? ""} onChange={(e) => onChange({ family: e.target.value })} />
          </div>
          <div className="space-y-1">
            <Label>发布日期</Label>
            <Input value={model.release_date ?? ""} onChange={(e) => onChange({ release_date: e.target.value })} placeholder="YYYY-MM-DD" />
          </div>
          <div className="space-y-1">
            <Label>状态</Label>
            <Select value={model.status ?? "active"} onValueChange={(v) => onChange({ status: v as ModelStatus })}>
              <SelectTrigger><SelectValue /></SelectTrigger>
              <SelectContent>
                {STATUSES.map((s) => <SelectItem key={s} value={s}>{s}</SelectItem>)}
              </SelectContent>
            </Select>
          </div>
        </div>

        <div className="border-t pt-3">
          <h4 className="text-sm font-medium mb-2">Token 限制（必填）</h4>
          <div className="grid grid-cols-3 gap-3">
            <div className="space-y-1">
              <Label>上下文窗口</Label>
              <Input type="number" min={1} value={limit.context}
                onChange={(e) => setLimit("context", Number(e.target.value))} />
            </div>
            <div className="space-y-1">
              <Label>输出上限</Label>
              <Input type="number" min={1} value={limit.output}
                onChange={(e) => setLimit("output", Number(e.target.value))} />
            </div>
            <div className="space-y-1">
              <Label>输入上限（可选）</Label>
              <Input type="number" min={1} value={limit.input ?? ""}
                onChange={(e) => setLimit("input", e.target.value === "" ? undefined : Number(e.target.value))} />
            </div>
          </div>
        </div>

        <div className="border-t pt-3">
          <h4 className="text-sm font-medium mb-2">能力开关</h4>
          <div className="grid grid-cols-2 gap-x-6 gap-y-2">
            {([
              ["attachment", "支持附件"], ["reasoning", "推理模型"],
              ["temperature", "支持 temperature"], ["tool_call", "支持工具调用"],
              ["experimental", "实验性"], ["interleaved", "流式交错输出"],
            ] as const).map(([key, label]) => (
              <div key={key} className="flex items-center justify-between">
                <Label>{label}</Label>
                <Switch checked={model[key] ?? false} onCheckedChange={(v) => onChange({ [key]: v })} />
              </div>
            ))}
          </div>
        </div>

        <div className="border-t pt-3">
          <h4 className="text-sm font-medium mb-2">模态</h4>
          <div className="space-y-2">
            <div className="flex items-center gap-4">
              <span className="text-xs text-muted-foreground w-10">输入</span>
              {MODALITIES.map((m) => (
                <label key={m} className="flex items-center gap-1 text-sm">
                  <Checkbox checked={(model.modalities?.input ?? []).includes(m)} onCheckedChange={() => toggleModality("input", m)} />
                  {m}
                </label>
              ))}
            </div>
            <div className="flex items-center gap-4">
              <span className="text-xs text-muted-foreground w-10">输出</span>
              {MODALITIES.map((m) => (
                <label key={m} className="flex items-center gap-1 text-sm">
                  <Checkbox checked={(model.modalities?.output ?? []).includes(m)} onCheckedChange={() => toggleModality("output", m)} />
                  {m}
                </label>
              ))}
            </div>
          </div>
        </div>

        <Collapsible open={costOpen} onOpenChange={setCostOpen}>
          <CollapsibleTrigger asChild>
            <Button variant="ghost" size="sm" className="w-full justify-between">
              定价（高级） <ChevronDown className={`h-4 w-4 transition-transform ${costOpen ? "rotate-180" : ""}`} />
            </Button>
          </CollapsibleTrigger>
          <CollapsibleContent className="grid grid-cols-2 gap-3 pt-3">
            <div className="space-y-1">
              <Label>输入 ($/百万 token)</Label>
              <Input type="number" min={0} step="any" value={cost?.input ?? ""}
                onChange={(e) => setCost("input", e.target.value)} />
            </div>
            <div className="space-y-1">
              <Label>输出 ($/百万 token)</Label>
              <Input type="number" min={0} step="any" value={cost?.output ?? ""}
                onChange={(e) => setCost("output", e.target.value)} />
            </div>
            <div className="space-y-1">
              <Label>缓存读</Label>
              <Input type="number" min={0} step="any" value={cost?.cache_read ?? ""}
                onChange={(e) => setCost("cache_read", e.target.value)} />
            </div>
            <div className="space-y-1">
              <Label>缓存写</Label>
              <Input type="number" min={0} step="any" value={cost?.cache_write ?? ""}
                onChange={(e) => setCost("cache_write", e.target.value)} />
            </div>
          </CollapsibleContent>
        </Collapsible>
      </CollapsibleContent>
    </Collapsible>
  )
}
