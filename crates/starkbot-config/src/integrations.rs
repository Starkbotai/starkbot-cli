use serde::{Deserialize, Serialize};
use std::path::Path;

/// Manifest for a single integration preset (read from manifest.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationManifest {
    pub name: String,
    pub description: String,
    pub icon: String,
    #[serde(default)]
    pub requires: IntegrationRequires,
    #[serde(default)]
    pub skills: Vec<String>,
}

/// What an integration requires to function.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntegrationRequires {
    pub api_key: Option<String>,
}

/// A single installed integration entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledIntegration {
    pub preset_id: String,
    pub installed_at: String,
}

/// Central registry of installed integrations (integrations.json).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntegrationRegistry {
    pub installed: Vec<InstalledIntegration>,
}

impl IntegrationRegistry {
    pub fn load(path: &Path) -> Self {
        if path.exists() {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize integrations: {}", e))?;
        std::fs::write(path, json)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))
    }

    pub fn is_installed(&self, preset_id: &str) -> bool {
        self.installed.iter().any(|i| i.preset_id == preset_id)
    }

    pub fn install(&mut self, preset_id: &str) {
        if !self.is_installed(preset_id) {
            self.installed.push(InstalledIntegration {
                preset_id: preset_id.to_string(),
                installed_at: chrono::Local::now().to_rfc3339(),
            });
        }
    }

    pub fn uninstall(&mut self, preset_id: &str) {
        self.installed.retain(|i| i.preset_id != preset_id);
    }
}

/// Load a manifest from a preset directory.
pub fn load_manifest(preset_dir: &Path) -> Option<IntegrationManifest> {
    let manifest_path = preset_dir.join("manifest.json");
    if !manifest_path.exists() {
        return None;
    }
    std::fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

/// Scan a presets directory and return all (id, manifest) pairs.
pub fn list_presets(presets_dir: &Path) -> Vec<(String, IntegrationManifest)> {
    let mut result = Vec::new();
    let entries = match std::fs::read_dir(presets_dir) {
        Ok(e) => e,
        Err(_) => return result,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(manifest) = load_manifest(&path) {
                    result.push((name.to_string(), manifest));
                }
            }
        }
    }
    result.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_install_uninstall() {
        let mut reg = IntegrationRegistry::default();
        assert!(!reg.is_installed("cloudflare"));

        reg.install("cloudflare");
        assert!(reg.is_installed("cloudflare"));

        // Idempotent
        reg.install("cloudflare");
        assert_eq!(reg.installed.len(), 1);

        reg.uninstall("cloudflare");
        assert!(!reg.is_installed("cloudflare"));
        assert!(reg.installed.is_empty());
    }

    #[test]
    fn test_registry_roundtrip() {
        let tmp = std::env::temp_dir().join("starkbot-integrations-test.json");
        let mut reg = IntegrationRegistry::default();
        reg.install("github");
        reg.save(&tmp).unwrap();

        let loaded = IntegrationRegistry::load(&tmp);
        assert!(loaded.is_installed("github"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_list_presets() {
        let tmp = std::env::temp_dir().join("starkbot-presets-test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("test-preset")).unwrap();

        let manifest = r#"{"name":"Test","description":"A test","icon":"star","requires":{},"skills":[]}"#;
        std::fs::write(tmp.join("test-preset/manifest.json"), manifest).unwrap();

        let presets = list_presets(&tmp);
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].0, "test-preset");
        assert_eq!(presets[0].1.name, "Test");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
