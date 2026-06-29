import { Plus } from "lucide-react"
import { Button } from "@/components/ui/button"
import { ModelEditor } from "./ModelEditor"
import type { ProviderConfig, ModelConfig } from "@/lib/opencode-types"

function genId() {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8)
}

interface Props {
  provider: ProviderConfig
  onChange: (patch: Partial<ProviderConfig>) => void
}

export function ProviderModelsTab({ provider, onChange }: Props) {
  const models = provider.models ?? {}

  const updateModel = (modelId: string, patch: Partial<ModelConfig>) => {
    const next = { ...models }
    next[modelId] = { ...next[modelId], ...patch }
    onChange({ models: next })
  }

  const addModel = () => {
    const id = `model-${genId()}`
    onChange({
      models: {
        ...models,
        [id]: { name: id, limit: { context: 128000, output: 4096 } },
      },
    })
  }

  const removeModel = (modelId: string) => {
    const next = { ...models }
    delete next[modelId]
    onChange({ models: next })
  }

  return (
    <div className="space-y-3">
      {Object.entries(models).map(([id, m]) => (
        <ModelEditor
          key={id}
          modelId={id}
          model={m}
          onChange={(patch) => updateModel(id, patch)}
          onDelete={() => removeModel(id)}
        />
      ))}
      <Button variant="outline" size="sm" onClick={addModel}>
        <Plus className="h-4 w-4 mr-1" /> 添加模型
      </Button>
    </div>
  )
}
