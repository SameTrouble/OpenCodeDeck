use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub id: String,
    pub name: String,
    #[serde(default = "default_hostname")]
    pub hostname: String,
    #[serde(default)]
    pub port: u16,
    pub cwd: String,
    #[serde(default)]
    pub extra_env: std::collections::HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

fn default_hostname() -> String { "127.0.0.1".to_string() }

/// 解析旧 url 字段回填 hostname/port，迁移后清空 url。
pub fn migrate_server_urls(cfg: &mut AppConfig) {
    for s in &mut cfg.servers {
        if s.port == 0 {
            if let Some(url) = s.url.take() {
                let after_scheme = url.split("://").nth(1).unwrap_or(&url);
                let host_port = after_scheme.split('/').next().unwrap_or(after_scheme);
                let (host, port_str) = match host_port.rsplit_once(':') {
                    Some((h, p)) => (h, p),
                    None => (host_port, ""),
                };
                s.hostname = if host.is_empty() { "127.0.0.1".to_string() } else { host.to_string() };
                if let Ok(p) = port_str.parse::<u16>() {
                    s.port = p;
                    if s.port == 0 {
                        s.port = 4097;
                    }
                } else if s.port == 0 {
                    s.port = 4097;
                }
            } else if s.port == 0 {
                s.port = 4097;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub servers: Vec<ServerConfig>,
    pub bridge: BridgeConfig,
    pub channels: ChannelsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeConfig {
    #[serde(default)]
    pub install_path: Option<String>,
    pub default_agent: String,
    pub data_dir: String,
    #[serde(default)]
    pub progress: ProgressConfig,
    #[serde(default)]
    pub launcher: LauncherConfig,
    #[serde(default = "default_bound_server_id")]
    pub bound_server_id: String,
}

fn default_bound_server_id() -> String { "default".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressConfig {
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    #[serde(default = "default_max_debounce_ms")]
    pub max_debounce_ms: u64,
}

fn default_debounce_ms() -> u64 { 500 }
fn default_max_debounce_ms() -> u64 { 3000 }

impl Default for ProgressConfig {
    fn default() -> Self {
        Self { debounce_ms: 500, max_debounce_ms: 3000 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub auto_start_server: bool,
    #[serde(default = "default_server_command")]
    pub server_command: String,
    #[serde(default = "default_server_start_timeout_ms")]
    pub server_start_timeout_ms: u64,
    #[serde(default = "default_probe_timeout_ms")]
    pub probe_timeout_ms: u64,
}

fn default_true() -> bool { true }
fn default_server_command() -> String { "opencode serve".to_string() }
fn default_server_start_timeout_ms() -> u64 { 30000 }
fn default_probe_timeout_ms() -> u64 { 4000 }

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_start_server: true,
            server_command: "opencode serve".to_string(),
            server_start_timeout_ms: 30000,
            probe_timeout_ms: 4000,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelsConfig {
    #[serde(default)]
    pub feishu: FeishuConfig,
    #[serde(default)]
    pub qq: QqConfig,
    #[serde(default)]
    pub telegram: TelegramConfig,
    #[serde(default)]
    pub discord: DiscordConfig,
    #[serde(default)]
    pub wechat: WechatConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeishuConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub app_secret: String,
    #[serde(default)]
    pub verification_token: String,
    #[serde(default = "default_webhook_port")]
    pub webhook_port: u16,
    #[serde(default)]
    pub encrypt_key: String,
}

fn default_webhook_port() -> u16 { 3001 }

impl Default for FeishuConfig {
    fn default() -> Self {
        Self {
            enabled: false, app_id: String::new(), app_secret: String::new(),
            verification_token: String::new(), webhook_port: 3001, encrypt_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QqConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub secret: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub allowed_chat_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub allowed_channel_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatConfig {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyAppConfig {
    version: u32,
    server: LegacyServerConfig,
    bridge: LegacyBridgeConfig,
    #[serde(default)]
    channels: ChannelsConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyServerConfig {
    port: u16,
    cwd: String,
    #[serde(default)]
    extra_env: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyBridgeConfig {
    #[serde(default)]
    install_path: Option<String>,
    #[serde(default = "default_agent_str")]
    default_agent: String,
    #[serde(default = "default_data_dir")]
    data_dir: String,
    #[serde(default)]
    progress: ProgressConfig,
    #[serde(default)]
    launcher: LauncherConfig,
}

fn default_agent_str() -> String { "build".to_string() }
fn default_data_dir() -> String { "./data".to_string() }

fn migrate_legacy(legacy: LegacyAppConfig) -> AppConfig {
    let server_id = "default".to_string();
    AppConfig {
        version: legacy.version,
        servers: vec![ServerConfig {
            id: server_id.clone(),
            name: "默认".to_string(),
            hostname: "127.0.0.1".to_string(),
            port: legacy.server.port,
            cwd: legacy.server.cwd,
            extra_env: legacy.server.extra_env,
            url: None,
        }],
        bridge: BridgeConfig {
            install_path: legacy.bridge.install_path,
            default_agent: legacy.bridge.default_agent,
            data_dir: legacy.bridge.data_dir,
            progress: legacy.bridge.progress,
            launcher: legacy.bridge.launcher,
            bound_server_id: server_id,
        },
        channels: legacy.channels,
    }
}

pub struct ConfigStore {
    config_dir: PathBuf,
}

impl ConfigStore {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(if cfg!(target_os = "macos") { "OpenCodeDeck" }
                  else if cfg!(target_os = "windows") { "OpenCodeDeck" }
                  else { "opencodedeck" });
        Self { config_dir }
    }

    pub fn config_dir(&self) -> &Path { &self.config_dir }

    pub fn config_path(&self) -> PathBuf { self.config_dir.join("config.json") }

    pub fn default_config() -> AppConfig {
        AppConfig {
            version: 1,
            servers: vec![ServerConfig {
                id: "default".to_string(),
                name: "默认".to_string(),
                hostname: "127.0.0.1".to_string(),
                port: 4097,
                cwd: dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
                extra_env: Default::default(),
                url: None,
            }],
            bridge: BridgeConfig {
                install_path: None,
                default_agent: "build".to_string(),
                data_dir: "./data".to_string(),
                progress: ProgressConfig::default(),
                launcher: LauncherConfig::default(),
                bound_server_id: "default".to_string(),
            },
            channels: ChannelsConfig::default(),
        }
    }

    pub fn load(&self) -> AppResult<AppConfig> {
        let path = self.config_path();
        if !path.exists() {
            let cfg = Self::default_config();
            self.save(&cfg)?;
            return Ok(cfg);
        }
        let content = std::fs::read_to_string(&path)?;
        if let Ok(mut cfg) = serde_json::from_str::<AppConfig>(&content) {
            let needs_migrate = cfg.servers.iter().any(|s| s.port == 0 && s.url.is_some());
            if needs_migrate {
                migrate_server_urls(&mut cfg);
                let _ = self.save(&cfg);
            }
            return Ok(cfg);
        }
        if let Ok(legacy) = serde_json::from_str::<LegacyAppConfig>(&content) {
            let cfg = migrate_legacy(legacy);
            let _ = self.save(&cfg);
            return Ok(cfg);
        }
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let backup = path.with_extension(format!("json.corrupt-{}", ts));
        let _ = std::fs::rename(&path, &backup);
        let cfg = Self::default_config();
        let _ = self.save(&cfg);
        Ok(cfg)
    }

    pub fn save(&self, config: &AppConfig) -> AppResult<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        let content = serde_json::to_string_pretty(config)?;
        let tmp = self.config_path().with_extension("json.tmp");
        std::fs::write(&tmp, &content)?;
        std::fs::rename(&tmp, self.config_path())?;
        Ok(())
    }

    pub fn bridge_install_path(&self, config: &AppConfig) -> PathBuf {
        if let Some(p) = &config.bridge.install_path {
            PathBuf::from(p)
        } else {
            self.config_dir.join("bridges").join("opencode-im-bridge")
        }
    }
}

#[cfg(test)]
mod robustness_tests {
    use super::*;

    fn temp_store() -> (ConfigStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore { config_dir: dir.path().to_path_buf() };
        (store, dir)
    }

    #[test]
    fn load_backs_up_corrupt_file_and_returns_default() {
        let (store, _dir) = temp_store();
        std::fs::create_dir_all(store.config_dir()).unwrap();
        std::fs::write(store.config_path(), "{ not valid json").unwrap();

        let cfg = store.load().unwrap();
        assert_eq!(cfg.servers[0].hostname, "127.0.0.1");
        assert_eq!(cfg.servers[0].port, 4097);

        // corrupt file was backed up
        let mut entries = std::fs::read_dir(store.config_dir()).unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        entries.sort();
        assert!(entries.iter().any(|n| n.starts_with("config.json.corrupt-")),
            "expected a corrupt backup, got: {:?}", entries);
        // config.json now exists and is valid
        assert!(store.config_path().exists());
    }

    #[test]
    fn save_is_atomic_no_tmp_residue() {
        let (store, _dir) = temp_store();
        let cfg = ConfigStore::default_config();
        store.save(&cfg).unwrap();
        let entries: Vec<_> = std::fs::read_dir(store.config_dir()).unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(!entries.iter().any(|n| n == "config.json.tmp"),
            "tmp file should not remain after save, got: {:?}", entries);
        assert!(entries.iter().any(|n| n == "config.json"));
    }

    #[test]
    fn migrates_legacy_single_server_config() {
        let (store, _dir) = temp_store();
        std::fs::create_dir_all(store.config_dir()).unwrap();
        let legacy = serde_json::json!({
            "version": 1,
            "server": {
                "port": 4097,
                "cwd": "/home/user",
                "extraEnv": {}
            },
            "bridge": {
                "installPath": null,
                "defaultAgent": "build",
                "dataDir": "./data",
                "progress": { "debounceMs": 500, "maxDebounceMs": 3000 },
                "launcher": {
                    "enabled": true,
                    "autoStartServer": true,
                    "serverCommand": "opencode serve",
                    "serverStartTimeoutMs": 30000,
                    "probeTimeoutMs": 4000
                }
            },
            "channels": {}
        });
        std::fs::write(store.config_path(), legacy.to_string()).unwrap();

        let cfg = store.load().unwrap();
        assert_eq!(cfg.servers.len(), 1, "legacy server should migrate to one-element servers array");
        assert_eq!(cfg.servers[0].hostname, "127.0.0.1");
        assert_eq!(cfg.servers[0].port, 4097);
        assert_eq!(cfg.servers[0].cwd, "/home/user");
        assert_eq!(cfg.servers[0].id, "default");
        assert_eq!(cfg.bridge.bound_server_id, "default");
    }

    #[test]
    fn default_config_has_empty_servers_with_bound_id() {
        let cfg = ConfigStore::default_config();
        assert!(!cfg.servers.is_empty(), "default config should have at least one server");
        assert!(!cfg.bridge.bound_server_id.is_empty(), "default bound_server_id should be set");
        let default_id = &cfg.servers[0].id;
        assert_eq!(&cfg.bridge.bound_server_id, default_id, "default bound_server_id must point to first server");
    }

    #[test]
    fn migrate_server_urls_parses_legacy_url_field() {
        let (store, _dir) = temp_store();
        std::fs::create_dir_all(store.config_dir()).unwrap();
        let with_url = serde_json::json!({
            "version": 1,
            "servers": [{
                "id": "default",
                "name": "默认",
                "url": "http://0.0.0.0:5050",
                "cwd": "/tmp",
                "extraEnv": {}
            }],
            "bridge": {
                "installPath": null,
                "defaultAgent": "build",
                "dataDir": "./data",
                "progress": { "debounceMs": 500, "maxDebounceMs": 3000 },
                "launcher": {
                    "enabled": true,
                    "autoStartServer": true,
                    "serverCommand": "opencode serve",
                    "serverStartTimeoutMs": 30000,
                    "probeTimeoutMs": 4000
                },
                "boundServerId": "default"
            },
            "channels": {}
        });
        std::fs::write(store.config_path(), with_url.to_string()).unwrap();

        let cfg = store.load().unwrap();
        assert_eq!(cfg.servers[0].hostname, "0.0.0.0");
        assert_eq!(cfg.servers[0].port, 5050);
        assert!(cfg.servers[0].url.is_none(), "url field should be cleared after migration");
    }

    #[test]
    fn migrate_server_urls_treats_parsed_port_zero_as_invalid() {
        let mut cfg = ConfigStore::default_config();
        cfg.servers[0].port = 0;
        cfg.servers[0].url = Some("http://0.0.0.0:0".to_string());
        migrate_server_urls(&mut cfg);
        assert_eq!(cfg.servers[0].hostname, "0.0.0.0");
        assert_eq!(cfg.servers[0].port, 4097, "parsed port 0 must fall back to 4097");
        assert!(cfg.servers[0].url.is_none());
    }

    #[test]
    fn default_config_has_hostname_and_port() {
        let cfg = ConfigStore::default_config();
        assert_eq!(cfg.servers[0].hostname, "127.0.0.1");
        assert_eq!(cfg.servers[0].port, 4097);
        assert!(cfg.servers[0].url.is_none());
    }
}
