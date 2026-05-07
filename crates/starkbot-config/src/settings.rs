use serde::{Deserialize, Serialize};
use std::path::Path;

/// Application settings stored in settings.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// LLM model name (e.g. "gpt-5.4").
    #[serde(default = "default_model")]
    pub model: String,

    /// Auto-approve tool calls without prompting.
    #[serde(default)]
    pub auto_approve: bool,

    /// Extension server URL for fetching hyperpacks.
    #[serde(default = "default_extension_server")]
    pub extension_server: String,
}

fn default_model() -> String {
    "gpt-5.4".to_string()
}

fn default_extension_server() -> String {
    "https://hyperpacks.org".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: default_model(),
            auto_approve: false,
            extension_server: default_extension_server(),
        }
    }
}

impl Settings {
    /// Load from a JSON file. Returns defaults if file doesn't exist.
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        if content.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
    }

    /// Save to a JSON file (atomic write).
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create dir {}: {}", parent.display(), e))?;
        }

        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &content)
            .map_err(|e| format!("Failed to write {}: {}", tmp_path.display(), e))?;
        std::fs::rename(&tmp_path, path)
            .map_err(|e| format!("Failed to rename {} -> {}: {}", tmp_path.display(), path.display(), e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_defaults() {
        let s = Settings::default();
        assert_eq!(s.model, "gpt-5.4");
        assert!(!s.auto_approve);
        assert_eq!(s.extension_server, "https://hyperpacks.org");
    }

    #[test]
    fn test_settings_save_load() {
        let tmp = std::env::temp_dir().join("starkbot-settings-test.json");
        let _ = std::fs::remove_file(&tmp);

        let mut s = Settings::default();
        s.model = "gpt-5.5".to_string();
        s.save(&tmp).unwrap();

        let loaded = Settings::load(&tmp).unwrap();
        assert_eq!(loaded.model, "gpt-5.5");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_settings_load_missing() {
        let s = Settings::load(Path::new("/tmp/nonexistent-settings.json")).unwrap();
        assert_eq!(s.model, "gpt-5.4");
    }
}
