use async_trait::async_trait;
use std::path::PathBuf;
use starkbot_config::integrations::{self, IntegrationRegistry};
use starkbot_config::keys::{self, KeyStore};

pub struct InstallIntegrationTool {
    data_root: PathBuf,
}

impl InstallIntegrationTool {
    pub fn new(data_root: PathBuf) -> Self {
        Self { data_root }
    }

    fn keys_path(&self) -> PathBuf {
        self.data_root.join("keys.json")
    }

    fn skills_dir(&self) -> PathBuf {
        self.data_root.join("skills")
    }

    fn custom_dir(&self) -> PathBuf {
        self.data_root.join("custom")
    }

    fn integrations_path(&self) -> PathBuf {
        self.data_root.join("integrations.json")
    }

    /// Resolve preset directory: bundled integration_presets first, then packs.
    fn resolve_preset_dir(&self, preset_id: &str) -> Option<PathBuf> {
        let bundled = self.data_root.join("integration_presets").join(preset_id);
        if bundled.join("manifest.json").exists() {
            return Some(bundled);
        }
        let pack = self.data_root.join("packs").join(preset_id);
        if pack.join("manifest.json").exists() {
            return Some(pack);
        }
        None
    }
}

fn tool_err(msg: impl Into<String>) -> metalcraft::GraphError {
    metalcraft::GraphError::ToolCallFailed {
        tool: "install_integration".into(),
        message: msg.into(),
    }
}

#[async_trait]
impl metalcraft::Tool for InstallIntegrationTool {
    fn name(&self) -> &str { "install_integration" }

    fn description(&self) -> &str {
        "Install an integration: stores API keys, copies skill files, and registers the integration. Use this instead of manually saving keys."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "preset_id": {
                    "type": "string",
                    "description": "Integration identifier (e.g. \"linear\", \"aws\", \"cloudflare\", \"github\", \"railway\", \"digitalocean\", \"notion\")"
                },
                "api_keys": {
                    "type": "array",
                    "description": "Array of API keys to store. For Linear: [{\"name\": \"LINEAR_API_KEY\", \"value\": \"lin_api_...\"}]. For AWS: [{\"name\": \"AWS_ACCESS_KEY_ID\", \"value\": \"...\"}, {\"name\": \"AWS_SECRET_ACCESS_KEY\", \"value\": \"...\"}]",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Key name in UPPER_SNAKE_CASE (e.g. LINEAR_API_KEY)"
                            },
                            "value": {
                                "type": "string",
                                "description": "The API key value"
                            }
                        },
                        "required": ["name", "value"]
                    }
                }
            },
            "required": ["preset_id", "api_keys"]
        })
    }

    async fn call(&self, args: serde_json::Value) -> metalcraft::Result<serde_json::Value> {
        let preset_id = args["preset_id"].as_str().ok_or_else(|| tool_err("Missing required parameter: preset_id"))?;
        let api_keys_val = args.get("api_keys").ok_or_else(|| tool_err("Missing required parameter: api_keys"))?;

        // Parse api_keys array into a name→value map
        let api_keys_arr = api_keys_val.as_array().ok_or_else(|| tool_err("api_keys must be an array"))?;
        let mut api_keys_map: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
        for item in api_keys_arr {
            let name = item["name"].as_str().ok_or_else(|| tool_err("Each api_keys entry must have a 'name' string"))?;
            let value = item["value"].as_str().ok_or_else(|| tool_err("Each api_keys entry must have a 'value' string"))?;
            api_keys_map.insert(name.to_string(), serde_json::Value::String(value.to_string()));
        }
        let api_keys_obj = &api_keys_map;

        // 1. Resolve preset directory
        let preset_dir = match self.resolve_preset_dir(preset_id) {
            Some(dir) => dir,
            None => {
                return Ok(serde_json::json!({
                    "error": format!("Integration preset '{}' not found. Check available integrations.", preset_id)
                }));
            }
        };

        // 2. Load manifest
        let manifest = match integrations::load_manifest(&preset_dir) {
            Some(m) => m,
            None => {
                return Ok(serde_json::json!({
                    "error": format!("Failed to load manifest for preset '{}'", preset_id)
                }));
            }
        };

        // 3. Validate required keys are provided
        let required_key_names: Vec<String> = if !manifest.requires.api_keys.is_empty() {
            manifest.requires.api_keys.iter().map(|k| k.name.clone()).collect()
        } else if let Some(ref key_name) = manifest.requires.api_key {
            vec![key_name.clone()]
        } else {
            vec![]
        };

        let mut missing: Vec<&str> = Vec::new();
        for required in &required_key_names {
            if !api_keys_obj.contains_key(required.as_str()) {
                missing.push(required);
            }
        }
        if !missing.is_empty() {
            return Ok(serde_json::json!({
                "error": format!("Missing required API keys: {}. The {} integration requires: {}",
                    missing.join(", "), preset_id, required_key_names.join(", "))
            }));
        }

        // 4. Store API keys
        let mut store = KeyStore::load(&self.keys_path()).map_err(|e| tool_err(format!("Failed to load keys: {}", e)))?;
        let mut stored_keys = Vec::new();
        for (key_name, key_value) in api_keys_obj {
            let value = key_value.as_str().ok_or_else(|| tool_err(format!("Value for '{}' must be a string", key_name)))?;
            if value.is_empty() {
                return Ok(serde_json::json!({
                    "error": format!("API key value for '{}' cannot be empty", key_name)
                }));
            }
            store.upsert(key_name, value);
            stored_keys.push(serde_json::json!({
                "name": key_name,
                "masked": keys::mask_key(value),
            }));
        }
        store.save(&self.keys_path()).map_err(|e| tool_err(format!("Failed to save keys: {}", e)))?;

        // 5. Copy skill files from preset → skills dir
        let skills_dir = self.skills_dir();
        let _ = std::fs::create_dir_all(&skills_dir);
        let mut installed_skills = Vec::new();
        for skill_file in &manifest.skills {
            let src = preset_dir.join(skill_file);
            let dst = skills_dir.join(skill_file);
            if src.exists() && !dst.exists() {
                let _ = std::fs::copy(&src, &dst);
            }
            installed_skills.push(skill_file.clone());
        }

        // 6. Copy custom configs → custom/{preset_id}/
        if !manifest.custom_configs.is_empty() {
            let custom_dest = self.custom_dir().join(preset_id);
            let _ = std::fs::create_dir_all(&custom_dest);
            for config_path in &manifest.custom_configs {
                let src = preset_dir.join(config_path);
                if src.exists() {
                    let filename = std::path::Path::new(config_path)
                        .file_name()
                        .unwrap_or_default();
                    let dst = custom_dest.join(filename);
                    if !dst.exists() {
                        let _ = std::fs::copy(&src, &dst);
                    }
                }
            }
        }

        // 7. Register in IntegrationRegistry
        let mut registry = IntegrationRegistry::load(&self.integrations_path());
        registry.install(preset_id);
        let _ = registry.save(&self.integrations_path());

        // 8. Return success
        Ok(serde_json::json!({
            "success": true,
            "preset_id": preset_id,
            "name": manifest.name,
            "keys_stored": stored_keys,
            "skills_installed": installed_skills,
            "message": format!("Integration '{}' installed successfully. {} key(s) stored, {} skill(s) installed.",
                manifest.name, stored_keys.len(), installed_skills.len())
        }))
    }
}
