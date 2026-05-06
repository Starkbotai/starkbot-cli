use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// JSON-backed API key storage. Replaces the SQLite external_api_keys table.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeyStore {
    #[serde(flatten)]
    keys: HashMap<String, String>,
}

impl KeyStore {
    /// Load from a JSON file. Returns empty store if file doesn't exist.
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

    /// Save to a JSON file (atomic write via temp + rename).
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(&self)
            .map_err(|e| format!("Failed to serialize keys: {}", e))?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create dir {}: {}", parent.display(), e))?;
        }

        // Atomic write: write to temp file, then rename
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &content)
            .map_err(|e| format!("Failed to write {}: {}", tmp_path.display(), e))?;
        std::fs::rename(&tmp_path, path)
            .map_err(|e| format!("Failed to rename {} -> {}: {}", tmp_path.display(), path.display(), e))?;

        Ok(())
    }

    /// Get an API key by service name.
    pub fn get(&self, service_name: &str) -> Option<&str> {
        self.keys.get(service_name).map(|s| s.as_str())
    }

    /// Insert or update an API key.
    pub fn upsert(&mut self, service_name: &str, api_key: &str) {
        self.keys.insert(service_name.to_string(), api_key.to_string());
    }

    /// Delete an API key. Returns true if it existed.
    pub fn delete(&mut self, service_name: &str) -> bool {
        self.keys.remove(service_name).is_some()
    }

    /// List all service names (sorted).
    pub fn list_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.keys.keys().cloned().collect();
        names.sort();
        names
    }

    /// List all keys as (name, masked_key) tuples, sorted by name.
    pub fn list_masked(&self) -> Vec<(String, String)> {
        let mut entries: Vec<(String, String)> = self.keys.iter()
            .map(|(name, key)| (name.clone(), mask_key(key)))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }

    /// Check if a key exists.
    pub fn contains(&self, service_name: &str) -> bool {
        self.keys.contains_key(service_name)
    }

    /// Number of stored keys.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

/// Mask an API key, showing first 4 and last 4 chars.
pub fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    format!("{}...{}", &key[..4], &key[key.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keystore_crud() {
        let mut store = KeyStore::default();
        store.upsert("CLOUDFLARE_API_TOKEN", "cf-abc123def456ghi789");
        store.upsert("GITHUB_TOKEN", "ghp_xxxxxxxxxxxx1234");

        assert_eq!(store.get("CLOUDFLARE_API_TOKEN"), Some("cf-abc123def456ghi789"));
        assert_eq!(store.get("NONEXISTENT"), None);
        assert_eq!(store.len(), 2);

        let names = store.list_names();
        assert_eq!(names, vec!["CLOUDFLARE_API_TOKEN", "GITHUB_TOKEN"]);

        assert!(store.delete("GITHUB_TOKEN"));
        assert!(!store.delete("NONEXISTENT"));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_keystore_save_load() {
        let tmp = std::env::temp_dir().join("starkbot-keystore-test.json");
        let _ = std::fs::remove_file(&tmp);

        let mut store = KeyStore::default();
        store.upsert("TEST_KEY", "sk-12345");
        store.save(&tmp).unwrap();

        let loaded = KeyStore::load(&tmp).unwrap();
        assert_eq!(loaded.get("TEST_KEY"), Some("sk-12345"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_keystore_load_missing() {
        let store = KeyStore::load(Path::new("/tmp/nonexistent-keystore.json")).unwrap();
        assert!(store.is_empty());
    }

    #[test]
    fn test_mask_key() {
        assert_eq!(mask_key("abcdefghijklmnop"), "abcd...mnop");
        assert_eq!(mask_key("short"), "*****");
        assert_eq!(mask_key("12345678"), "********");
        assert_eq!(mask_key("123456789"), "1234...6789");
    }

    #[test]
    fn test_list_masked() {
        let mut store = KeyStore::default();
        store.upsert("A_KEY", "abcdefghijklmnop");
        store.upsert("B_KEY", "short");
        let masked = store.list_masked();
        assert_eq!(masked[0], ("A_KEY".to_string(), "abcd...mnop".to_string()));
        assert_eq!(masked[1], ("B_KEY".to_string(), "*****".to_string()));
    }
}
