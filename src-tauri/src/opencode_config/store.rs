use std::path::PathBuf;
use crate::error::AppResult;

pub struct OpencodeConfigStore {
    config_dir: PathBuf,
}

impl OpencodeConfigStore {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("opencode");
        Self { config_dir }
    }

    pub fn config_dir(&self) -> &std::path::Path { &self.config_dir }

    pub fn config_path(&self) -> PathBuf {
        let json = self.config_dir.join("opencode.json");
        if json.exists() { return json; }
        let jsonc = self.config_dir.join("opencode.jsonc");
        if jsonc.exists() { return jsonc; }
        json
    }

    pub fn load(&self) -> AppResult<serde_json::Value> {
        let path = self.config_path();
        if !path.exists() {
            return Ok(serde_json::json!({}));
        }
        let content = std::fs::read_to_string(&path)?;
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(v) => Ok(v),
            Err(_) => {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let backup = path.with_extension(format!("json.corrupt-{}", ts));
                let _ = std::fs::rename(&path, &backup);
                Ok(serde_json::json!({}))
            }
        }
    }

    pub fn save(&self, config: &serde_json::Value) -> AppResult<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        let content = serde_json::to_string_pretty(config)?;
        let tmp = self.config_path().with_extension("json.tmp");
        std::fs::write(&tmp, &content)?;
        std::fs::rename(&tmp, self.config_path())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (OpencodeConfigStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = OpencodeConfigStore { config_dir: dir.path().to_path_buf() };
        (store, dir)
    }

    #[test]
    fn load_returns_empty_object_when_file_missing() {
        let (store, _dir) = temp_store();
        let val = store.load().unwrap();
        assert_eq!(val, serde_json::json!({}));
    }

    #[test]
    fn load_returns_value_with_unknown_fields_preserved() {
        let (store, _dir) = temp_store();
        std::fs::create_dir_all(store.config_dir()).unwrap();
        let content = r#"{
            "$schema": "https://opencode.ai/config.json",
            "permission": { "external_directory": { "*": "allow" } },
            "plugin": ["superpowers"],
            "provider": {
                "deepseek": {
                    "npm": "@ai-sdk/openai-compatible",
                    "options": { "apiKey": "sk-xxx", "baseURL": "https://api.deepseek.com/v1" }
                }
            }
        }"#;
        std::fs::write(store.config_path(), content).unwrap();

        let val = store.load().unwrap();
        assert_eq!(val["$schema"], "https://opencode.ai/config.json");
        assert_eq!(val["permission"]["external_directory"]["*"], "allow");
        assert_eq!(val["plugin"][0], "superpowers");
        assert_eq!(val["provider"]["deepseek"]["npm"], "@ai-sdk/openai-compatible");
    }

    #[test]
    fn load_backs_up_corrupt_file_and_returns_empty() {
        let (store, _dir) = temp_store();
        std::fs::create_dir_all(store.config_dir()).unwrap();
        std::fs::write(store.config_path(), "{ not valid json").unwrap();

        let val = store.load().unwrap();
        assert_eq!(val, serde_json::json!({}));

        let entries: Vec<_> = std::fs::read_dir(store.config_dir()).unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(entries.iter().any(|n| n.starts_with("opencode.json.corrupt-")),
            "expected a corrupt backup, got: {:?}", entries);
    }

    #[test]
    fn save_writes_file_and_preserves_all_fields() {
        let (store, _dir) = temp_store();
        let val = serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "permission": { "external_directory": { "*": "allow" } },
            "provider": {
                "test": { "npm": "@ai-sdk/openai", "options": { "apiKey": "sk-xxx" } }
            }
        });
        store.save(&val).unwrap();

        let written = std::fs::read_to_string(store.config_path()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(parsed, val);
    }

    #[test]
    fn save_is_atomic_no_tmp_residue() {
        let (store, _dir) = temp_store();
        store.save(&serde_json::json!({})).unwrap();
        let entries: Vec<_> = std::fs::read_dir(store.config_dir()).unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(!entries.iter().any(|n| n == "opencode.json.tmp"),
            "tmp file should not remain after save, got: {:?}", entries);
        assert!(entries.iter().any(|n| n == "opencode.json"));
    }

    #[test]
    fn config_path_prefers_json_over_jsonc() {
        let (store, _dir) = temp_store();
        std::fs::create_dir_all(store.config_dir()).unwrap();
        std::fs::write(store.config_dir().join("opencode.jsonc"), "{}").unwrap();
        std::fs::write(store.config_dir().join("opencode.json"), "{}").unwrap();
        assert!(store.config_path().ends_with("opencode.json"));
    }

    #[test]
    fn config_path_falls_back_to_jsonc() {
        let (store, _dir) = temp_store();
        std::fs::create_dir_all(store.config_dir()).unwrap();
        std::fs::write(store.config_dir().join("opencode.jsonc"), "{}").unwrap();
        assert!(store.config_path().ends_with("opencode.jsonc"));
    }

    #[test]
    fn new_resolves_to_home_config_opencode() {
        let dir = tempfile::tempdir().unwrap();
        let fake_home = dir.path().to_path_buf();
        let original = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", &fake_home);
        }
        let store = OpencodeConfigStore::new();
        unsafe {
            match original {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
        }
        let expected = fake_home.join(".config").join("opencode");
        assert_eq!(store.config_dir(), expected);
    }
}
