export type Modality = "text" | "audio" | "image" | "video" | "pdf"
export type ModelStatus = "alpha" | "beta" | "deprecated" | "active"

export interface ProviderOptions {
  apiKey?: string
  baseURL?: string
  setCacheKey?: boolean
  timeout?: number | false
  headerTimeout?: number | false
  chunkTimeout?: number
  enterpriseUrl?: string
  [key: string]: unknown
}

export interface ModelLimit {
  context: number
  input?: number
  output: number
}

export interface ModelCost {
  input: number
  output: number
  cache_read?: number
  cache_write?: number
}

export interface ModelModalities {
  input?: Modality[]
  output?: Modality[]
}

export interface ModelConfig {
  id?: string
  name?: string
  family?: string
  release_date?: string
  attachment?: boolean
  reasoning?: boolean
  temperature?: boolean
  tool_call?: boolean
  experimental?: boolean
  interleaved?: boolean
  status?: ModelStatus
  limit?: ModelLimit
  cost?: ModelCost
  modalities?: ModelModalities
  [key: string]: unknown
}

export interface ProviderConfig {
  api?: string
  name?: string
  npm?: string
  env?: string[]
  whitelist?: string[]
  blacklist?: string[]
  options?: ProviderOptions
  models?: Record<string, ModelConfig>
  [key: string]: unknown
}

export interface OpenCodeConfig {
  $schema?: string
  provider?: Record<string, ProviderConfig>
  [key: string]: unknown
}
