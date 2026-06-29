import { useState } from "react"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import {
  Collapsible, CollapsibleContent, CollapsibleTrigger,
} from "@/components/ui/collapsible"
import { Button } from "@/components/ui/button"
import { ChevronDown } from "lucide-react"
import type { ProviderConfig } from "@/lib/opencode-types"

const NPM_OPTIONS = [
  "@ai-sdk/openai",
  "@ai-sdk/openai-compatible",
  "@ai-sdk/anthropic",
  "@ai-sdk/google",
  "@ai-sdk/groq",
  "@ai-sdk/mistral",
  "@ai-sdk/amazon-bedrock",
]

interface Props {
  provider: ProviderConfig
  onChange: (patch: Partial<ProviderConfig>) => void
}

export function ProviderBasicTab({ provider, onChange }: Props) {
  const opts = provider.options ?? {}
  const [advOpen, setAdvOpen] = useState(false)
  const [filterOpen, setFilterOpen] = useState(false)

  const setOption = (key: string, value: unknown) => {
    onChange({ options: { ...opts, [key]: value } })
  }

  const numOrEmpty = (v: number | false | undefined): string =>
    v === undefined || v === false ? "" : String(v)

  const parseTimeout = (s: string): number | false | undefined => {
    if (s === "") return undefined
    const n = Number(s)
    return isNaN(n) ? false : n
  }

  return (
    <div className="space-y-4">
      <div className="space-y-1">
        <Label>显示名称</Label>
        <Input value={provider.name ?? ""} onChange={(e) => onChange({ name: e.target.value })} />
      </div>

      <div className="space-y-1">
        <Label>npm 包</Label>
        <Input
          list="npm-options"
          value={provider.npm ?? ""}
          onChange={(e) => onChange({ npm: e.target.value })}
          placeholder="@ai-sdk/openai"
        />
        <datalist id="npm-options">
          {NPM_OPTIONS.map((o) => <option key={o} value={o} />)}
        </datalist>
      </div>

      <div className="space-y-1">
        <Label>API 类型（可选，留空则继承）</Label>
        <Input value={provider.api ?? ""} onChange={(e) => onChange({ api: e.target.value })} placeholder="openai" />
      </div>

      <div className="border-t pt-4">
        <h4 className="text-sm font-medium mb-3">鉴权与连接</h4>
        <div className="space-y-3">
          <div className="space-y-1">
            <Label>API Key</Label>
            <Input type="password" value={opts.apiKey ?? ""} onChange={(e) => setOption("apiKey", e.target.value)} />
          </div>
          <div className="space-y-1">
            <Label>Base URL</Label>
            <Input value={opts.baseURL ?? ""} onChange={(e) => setOption("baseURL", e.target.value)} placeholder="https://api.example.com/v1" />
          </div>
          <div className="flex items-center justify-between">
            <Label>启用缓存键</Label>
            <Switch checked={opts.setCacheKey ?? false} onCheckedChange={(v) => setOption("setCacheKey", v)} />
          </div>
        </div>
      </div>

      <Collapsible open={advOpen} onOpenChange={setAdvOpen}>
        <CollapsibleTrigger asChild>
          <Button variant="ghost" size="sm" className="w-full justify-between">
            超时（高级） <ChevronDown className={`h-4 w-4 transition-transform ${advOpen ? "rotate-180" : ""}`} />
          </Button>
        </CollapsibleTrigger>
        <CollapsibleContent className="space-y-3 pt-3">
          <div className="space-y-1">
            <Label>请求超时 (ms)</Label>
            <Input type="number" value={numOrEmpty(opts.timeout as number | false | undefined)}
              onChange={(e) => setOption("timeout", parseTimeout(e.target.value))} placeholder="留空=默认" />
          </div>
          <div className="space-y-1">
            <Label>响应头超时 (ms)</Label>
            <Input type="number" value={numOrEmpty(opts.headerTimeout as number | false | undefined)}
              onChange={(e) => setOption("headerTimeout", parseTimeout(e.target.value))} placeholder="留空=默认" />
          </div>
          <div className="space-y-1">
            <Label>流式块超时 (ms)</Label>
            <Input type="number" value={opts.chunkTimeout !== undefined ? String(opts.chunkTimeout) : ""}
              onChange={(e) => setOption("chunkTimeout", e.target.value === "" ? undefined : Number(e.target.value))} placeholder="留空=默认" />
          </div>
        </CollapsibleContent>
      </Collapsible>

      <Collapsible open={filterOpen} onOpenChange={setFilterOpen}>
        <CollapsibleTrigger asChild>
          <Button variant="ghost" size="sm" className="w-full justify-between">
            模型过滤（高级） <ChevronDown className={`h-4 w-4 transition-transform ${filterOpen ? "rotate-180" : ""}`} />
          </Button>
        </CollapsibleTrigger>
        <CollapsibleContent className="space-y-3 pt-3">
          <div className="space-y-1">
            <Label>白名单（逗号分隔模型 ID）</Label>
            <Input value={(provider.whitelist ?? []).join(",")}
              onChange={(e) => onChange({ whitelist: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} />
          </div>
          <div className="space-y-1">
            <Label>黑名单（逗号分隔模型 ID）</Label>
            <Input value={(provider.blacklist ?? []).join(",")}
              onChange={(e) => onChange({ blacklist: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} />
          </div>
          <div className="space-y-1">
            <Label>环境变量名（逗号分隔）</Label>
            <Input value={(provider.env ?? []).join(",")}
              onChange={(e) => onChange({ env: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} />
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  )
}
