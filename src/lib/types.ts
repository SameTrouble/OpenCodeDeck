export type ProcessStateKind = "Stopped" | "Starting" | "Running" | "Stopping" | "Failed"

export interface ProcessState {
  state: ProcessStateKind
  pid: number | null
  startedAt: number | null
  uptimeSec: number | null
  exitCode: number | null
  healthy: boolean | null
}

export interface FullState {
  server: ProcessState
  bridge: ProcessState
}

export type ProcessTarget = "server" | "bridge"

export interface ServerConfig {
  port: number
  opencodeServerUrl: string
  cwd: string
  extraEnv: Record<string, string>
}

export interface ProgressConfig {
  debounceMs: number
  maxDebounceMs: number
}

export interface LauncherConfig {
  enabled: boolean
  autoStartServer: boolean
  serverCommand: string
  serverStartTimeoutMs: number
  probeTimeoutMs: number
}

export interface BridgeConfig {
  installPath: string | null
  defaultAgent: string
  dataDir: string
  progress: ProgressConfig
  launcher: LauncherConfig
}

export interface FeishuConfig {
  enabled: boolean
  appId: string
  appSecret: string
  verificationToken: string
  webhookPort: number
  encryptKey: string
}

export interface QqConfig {
  enabled: boolean
  appId: string
  secret: string
}

export interface TelegramConfig {
  enabled: boolean
  botToken: string
  allowedChatIds: string[]
}

export interface DiscordConfig {
  enabled: boolean
  botToken: string
  allowedChannelIds: string[]
}

export interface WechatConfig {
  enabled: boolean
}

export interface ChannelsConfig {
  feishu: FeishuConfig
  qq: QqConfig
  telegram: TelegramConfig
  discord: DiscordConfig
  wechat: WechatConfig
}

export interface AppConfig {
  version: number
  server: ServerConfig
  bridge: BridgeConfig
  channels: ChannelsConfig
}

export interface LogEntry {
  ts: number
  source: "server" | "bridge"
  level: "info" | "error"
  line: string
}

export type QrKind = "ascii" | "url"

export interface WechatQrEvent {
  kind: QrKind
  data: string
}

export interface DepStatus {
  opencode: boolean
  bun: boolean
  node: boolean
  npm: boolean
  git: boolean
}

export type AppError = { kind: "Io" | "Config" | "Process" | "BridgeInstall" | "EnvNotFound"; message: string }
