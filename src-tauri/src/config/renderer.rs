use std::path::Path;
use crate::error::AppResult;
use crate::config::store::AppConfig;

/// Derive the server URL from the configured URL and port.
///
/// The bridge connects to `OPENCODE_SERVER_URL`, while OpenCodeDeck starts
/// `opencode serve --port <port>`. If these two disagree (e.g. the user
/// changed the port but not the URL), the bridge POSTs to one instance but
/// subscribes to SSE on another, so replies never arrive.
///
/// We keep the scheme and host from `opencode_server_url` but always
/// substitute `port`, guaranteeing the bridge and the launched server agree.
fn derive_server_url(configured_url: &str, port: u16) -> String {
    if let Some((scheme, rest)) = configured_url.split_once("://") {
        let after_host = rest.find(['/', ':']).map(|i| &rest[i..]).unwrap_or("");
        let host_end = after_host.len();
        let host = &rest[..rest.len() - host_end];
        return format!("{}://{}:{}", scheme, host, port);
    }
    format!("http://127.0.0.1:{}", port)
}

pub fn render_env(config: &AppConfig) -> String {
    let mut lines = Vec::new();

    let server_url = derive_server_url(&config.server.opencode_server_url, config.server.port);
    lines.push(format!("OPENCODE_SERVER_URL={}", server_url));
    if !config.server.cwd.is_empty() {
        lines.push(format!("OPENCODE_CWD={}", config.server.cwd));
    }

    let f = &config.channels.feishu;
    if f.enabled {
        lines.push(format!("FEISHU_APP_ID={}", f.app_id));
        lines.push(format!("FEISHU_APP_SECRET={}", f.app_secret));
        if !f.verification_token.is_empty() {
            lines.push(format!("FEISHU_VERIFICATION_TOKEN={}", f.verification_token));
        }
        lines.push(format!("FEISHU_WEBHOOK_PORT={}", f.webhook_port));
        if !f.encrypt_key.is_empty() {
            lines.push(format!("FEISHU_ENCRYPT_KEY={}", f.encrypt_key));
        }
    }

    let q = &config.channels.qq;
    if q.enabled {
        lines.push(format!("QQ_APP_ID={}", q.app_id));
        lines.push(format!("QQ_SECRET={}", q.secret));
    }

    let t = &config.channels.telegram;
    if t.enabled {
        lines.push(format!("TELEGRAM_BOT_TOKEN={}", t.bot_token));
        if !t.allowed_chat_ids.is_empty() {
            lines.push(format!("TELEGRAM_ALLOWED_CHAT_IDS={}", t.allowed_chat_ids.join(",")));
        }
    }

    let d = &config.channels.discord;
    if d.enabled {
        lines.push(format!("DISCORD_BOT_TOKEN={}", d.bot_token));
        if !d.allowed_channel_ids.is_empty() {
            lines.push(format!("DISCORD_ALLOWED_CHANNEL_IDS={}", d.allowed_channel_ids.join(",")));
        }
    }

    let w = &config.channels.wechat;
    if w.enabled {
        lines.push("WECHAT_ENABLED=true".to_string());
    }

    lines.push(format!("OPENCODE_SERVER_PORT={}", config.server.port));

    lines.join("\n") + "\n"
}

pub fn render_jsonc(config: &AppConfig) -> String {
    let f = &config.channels.feishu;
    let mut s = String::new();
    s.push_str("{\n");

    if f.enabled {
        s.push_str("  \"feishu\": {\n");
        s.push_str(&format!("    \"appId\": \"{}\",\n", f.app_id));
        s.push_str(&format!("    \"appSecret\": \"{}\",\n", f.app_secret));
        s.push_str(&format!("    \"verificationToken\": \"{}\",\n", f.verification_token));
        s.push_str(&format!("    \"webhookPort\": {},\n", f.webhook_port));
        s.push_str(&format!("    \"encryptKey\": \"{}\"\n", f.encrypt_key));
        s.push_str("  },\n");
    }

    s.push_str(&format!("  \"defaultAgent\": \"{}\",\n", config.bridge.default_agent));
    s.push_str(&format!("  \"dataDir\": \"{}\",\n", config.bridge.data_dir));

    s.push_str("  \"progress\": {\n");
    s.push_str(&format!("    \"debounceMs\": {},\n", config.bridge.progress.debounce_ms));
    s.push_str(&format!("    \"maxDebounceMs\": {}\n", config.bridge.progress.max_debounce_ms));
    s.push_str("  },\n");

    let l = &config.bridge.launcher;
    s.push_str("  \"launcher\": {\n");
    s.push_str(&format!("    \"enabled\": {},\n", l.enabled));
    s.push_str(&format!("    \"autoStartServer\": {},\n", l.auto_start_server));
    s.push_str(&format!("    \"serverCommand\": \"{}\",\n", l.server_command));
    s.push_str(&format!("    \"serverStartTimeoutMs\": {},\n", l.server_start_timeout_ms));
    s.push_str(&format!("    \"probeTimeoutMs\": {}\n", l.probe_timeout_ms));
    s.push_str("  }\n");

    s.push_str("}\n");
    s
}

pub fn write_bridge_files(config: &AppConfig, bridge_dir: &Path) -> AppResult<()> {
    let env_content = render_env(config);
    let jsonc_content = render_jsonc(config);
    std::fs::write(bridge_dir.join(".env"), env_content)?;
    std::fs::write(bridge_dir.join("opencode-im.jsonc"), jsonc_content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::store::*;

    fn sample_config() -> AppConfig {
        let mut cfg = ConfigStore::default_config();
        cfg.channels.feishu.enabled = true;
        cfg.channels.feishu.app_id = "cli_abc".to_string();
        cfg.channels.feishu.app_secret = "secret123".to_string();
        cfg.channels.feishu.verification_token = "tok".to_string();
        cfg.channels.feishu.encrypt_key = "key".to_string();
        cfg.channels.wechat.enabled = true;
        cfg
    }

    #[test]
    fn render_env_includes_enabled_channels() {
        let cfg = sample_config();
        let env = render_env(&cfg);
        assert!(env.contains("FEISHU_APP_ID=cli_abc"));
        assert!(env.contains("FEISHU_APP_SECRET=secret123"));
        assert!(env.contains("FEISHU_VERIFICATION_TOKEN=tok"));
        assert!(env.contains("FEISHU_ENCRYPT_KEY=key"));
        assert!(env.contains("WECHAT_ENABLED=true"));
        assert!(env.contains("OPENCODE_SERVER_URL=http://127.0.0.1:4097"));
    }

    #[test]
    fn render_env_excludes_disabled_channels() {
        let cfg = ConfigStore::default_config();
        let env = render_env(&cfg);
        assert!(!env.contains("FEISHU_APP_ID="));
        assert!(!env.contains("WECHAT_ENABLED="));
    }

    #[test]
    fn render_jsonc_has_required_fields() {
        let cfg = sample_config();
        let jsonc = render_jsonc(&cfg);
        assert!(jsonc.contains("\"defaultAgent\": \"build\""));
        assert!(jsonc.contains("\"appId\": \"cli_abc\""));
        assert!(jsonc.contains("\"webhookPort\": 3001"));
    }

    #[test]
    fn render_env_telegram_joins_chat_ids() {
        let mut cfg = ConfigStore::default_config();
        cfg.channels.telegram.enabled = true;
        cfg.channels.telegram.bot_token = "tok".to_string();
        cfg.channels.telegram.allowed_chat_ids = vec!["111".to_string(), "222".to_string()];
        let env = render_env(&cfg);
        assert!(env.contains("TELEGRAM_BOT_TOKEN=tok"));
        assert!(env.contains("TELEGRAM_ALLOWED_CHAT_IDS=111,222"));
    }

    #[test]
    fn render_env_server_url_derived_from_port() {
        let mut cfg = ConfigStore::default_config();
        cfg.server.port = 4092;
        cfg.server.opencode_server_url = "http://127.0.0.1:4097".to_string();
        let env = render_env(&cfg);
        assert!(
            env.contains("OPENCODE_SERVER_URL=http://127.0.0.1:4092"),
            "OPENCODE_SERVER_URL must be derived from port, got: {}",
            env
        );
    }

    #[test]
    fn render_env_server_url_preserves_custom_host() {
        let mut cfg = ConfigStore::default_config();
        cfg.server.port = 4092;
        cfg.server.opencode_server_url = "http://opencode.local:4097".to_string();
        let env = render_env(&cfg);
        assert!(
            env.contains("OPENCODE_SERVER_URL=http://opencode.local:4092"),
            "custom host should be preserved with derived port, got: {}",
            env
        );
    }
}
