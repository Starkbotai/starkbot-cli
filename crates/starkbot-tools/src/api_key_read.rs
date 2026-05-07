use async_trait::async_trait;
use std::path::PathBuf;
use starkbot_config::keys::KeyStore;

pub struct ApiKeyReadTool {
    keys_path: PathBuf,
}

impl ApiKeyReadTool {
    pub fn new(keys_path: PathBuf) -> Self {
        Self { keys_path }
    }
}

#[async_trait]
impl metalcraft::Tool for ApiKeyReadTool {
    fn name(&self) -> &str { "api_key_read" }
    fn description(&self) -> &str {
        "Read an API key or credential value from the keystore by name. Returns the actual value so you can use it in API calls."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The key name to read (e.g. DOTA_API_KEY, CLOUDFLARE_API_TOKEN)"
                }
            },
            "required": ["name"]
        })
    }
    async fn call(&self, args: serde_json::Value) -> metalcraft::Result<serde_json::Value> {
        let name = args["name"].as_str().ok_or_else(|| metalcraft::GraphError::ToolCallFailed {
            tool: "api_key_read".into(),
            message: "Missing required parameter: name".into(),
        })?;

        let store = KeyStore::load(&self.keys_path).map_err(|e| metalcraft::GraphError::ToolCallFailed {
            tool: "api_key_read".into(),
            message: format!("Failed to load keys: {}", e),
        })?;

        match store.get(name) {
            Some(value) => Ok(serde_json::json!({
                "name": name,
                "value": value,
            })),
            None => Ok(serde_json::json!({
                "name": name,
                "value": null,
                "error": format!("{} is not configured in the keystore", name),
            })),
        }
    }
}
