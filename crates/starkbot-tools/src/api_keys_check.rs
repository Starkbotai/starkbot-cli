use async_trait::async_trait;
use std::path::PathBuf;
use starkbot_config::keys::KeyStore;

pub struct ApiKeysCheckTool {
    keys_path: PathBuf,
}

impl ApiKeysCheckTool {
    pub fn new(keys_path: PathBuf) -> Self {
        Self { keys_path }
    }
}

#[async_trait]
impl metalcraft::Tool for ApiKeysCheckTool {
    fn name(&self) -> &str { "api_keys_check" }
    fn description(&self) -> &str {
        "Check which API keys are configured. Returns a list of configured service names (never reveals actual key values). Use this before making API calls to verify required keys exist."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "service_name": {
                    "type": "string",
                    "description": "Check for a specific service name (e.g. CLOUDFLARE_API_TOKEN). Use empty string to list all configured keys."
                }
            },
            "required": ["service_name"]
        })
    }
    async fn call(&self, args: serde_json::Value) -> metalcraft::Result<serde_json::Value> {
        let store = KeyStore::load(&self.keys_path).map_err(|e| metalcraft::GraphError::ToolCallFailed {
            tool: "api_keys_check".into(),
            message: format!("Failed to load keys: {}", e),
        })?;

        let service_name = args["service_name"].as_str().unwrap_or("");
        if !service_name.is_empty() {
            let exists = store.contains(service_name);
            Ok(serde_json::json!({
                "service_name": service_name,
                "configured": exists,
                "message": if exists {
                    format!("{} is configured", service_name)
                } else {
                    format!("{} is NOT configured. Use install_api_key to add it.", service_name)
                }
            }))
        } else {
            let names = store.list_names();
            Ok(serde_json::json!({
                "configured_keys": names,
                "count": names.len(),
                "message": if names.is_empty() {
                    "No API keys configured. Use install_api_key to add keys.".to_string()
                } else {
                    format!("{} API key(s) configured: {}", names.len(), names.join(", "))
                }
            }))
        }
    }
}
