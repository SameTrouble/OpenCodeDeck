import { invoke } from "@tauri-apps/api/core"
import type { AppConfig, DepStatus, FullState, LogEntry, ProcessState, ProcessTarget } from "./types"

export const getState = () => invoke<FullState>("get_state")
export const startProcess = (target: ProcessTarget) => invoke<ProcessState>("start_process", { target })
export const stopProcess = (target: ProcessTarget) => invoke<void>("stop_process", { target })
export const restartProcess = (target: ProcessTarget) => invoke<ProcessState>("restart_process", { target })
export const startAll = () => invoke<void>("start_all")
export const stopAll = () => invoke<void>("stop_all")
export const restartAll = () => invoke<void>("restart_all")
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
