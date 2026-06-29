import { invoke } from "@tauri-apps/api/core"
import type { AppConfig, DepStatus, FullState, LogEntry, ProcessState, ProcessTarget } from "./types"
import type { OpenCodeConfig } from "./opencode-types"

export const getState = () => invoke<FullState>("get_state")
export const startProcess = (target: ProcessTarget, serverId?: string) =>
  invoke<ProcessState>("start_process", { target, serverId: serverId ?? null })
export const stopProcess = (target: ProcessTarget, serverId?: string) =>
  invoke<void>("stop_process", { target, serverId: serverId ?? null })
export const restartProcess = (target: ProcessTarget, serverId?: string) =>
  invoke<ProcessState>("restart_process", { target, serverId: serverId ?? null })
export const bindBridge = (serverId: string) =>
  invoke<void>("bind_bridge", { serverId })
export const getConfig = () => invoke<AppConfig>("get_config")
export const saveConfig = (config: AppConfig) => invoke<void>("save_config", { config })
export const checkBridgeUpdate = () => invoke<boolean>("check_bridge_update")
export const updateBridge = () => invoke<void>("update_bridge")
export const reinstallBridge = () => invoke<void>("reinstall_bridge")
export const getLogHistory = (source: "server" | "bridge" | "all", limit: number) =>
  invoke<LogEntry[]>("get_log_history", { source, limit })
export const clearLogs = (source: string) => invoke<void>("clear_logs", { source })
export const exportLogs = (source: string) => invoke<string>("export_logs", { source })
export const checkDeps = () => invoke<DepStatus>("check_deps")
export const getOpencodeConfig = () => invoke<OpenCodeConfig>("get_opencode_config")
export const saveOpencodeConfig = (config: OpenCodeConfig) => invoke<void>("save_opencode_config", { config })
