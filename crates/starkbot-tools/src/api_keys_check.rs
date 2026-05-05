use async_trait::async_trait;
use std::path::PathBuf;
use starkbot_db::Database;

pub struct ApiKeysCheckTool {
    db_path: PathBuf,
}

impl ApiKeysCheckTool {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
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
        let db = Database::open(&self.db_path).map_err(|e| metalcraft::GraphError::ToolCallFailed {
            tool: "api_keys_check".into(),
            message: format!("Failed to open database: {}", e),
        })?;

        let service_name = args["service_name"].as_str().unwrap_or("");
        if !service_name.is_empty() {
            let name = service_name;
            let exists = db.get_api_key(name)
                .map_err(|e| metalcraft::GraphError::ToolCallFailed {
                    tool: "api_keys_check".into(),
                    message: format!("DB error: {}", e),
                })?
                .is_some();
            Ok(serde_json::json!({
                "service_name": name,
                "configured": exists,
                "message": if exists {
                    format!("{} is configured", name)
                } else {
                    format!("{} is NOT configured. Use install_api_key to add it.", name)
                }
            }))
        } else {
            let names = db.get_configured_key_names().map_err(|e| metalcraft::GraphError::ToolCallFailed {
                tool: "api_keys_check".into(),
                message: format!("DB error: {}", e),
            })?;
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
