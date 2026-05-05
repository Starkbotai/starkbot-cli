use async_trait::async_trait;
use std::path::PathBuf;
use starkbot_db::{Database, mask_key};

pub struct InstallApiKeyTool {
    db_path: PathBuf,
}

impl InstallApiKeyTool {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }
}

fn is_upper_snake_case(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        && s.chars().next().map_or(false, |c| c.is_ascii_uppercase())
}

#[async_trait]
impl metalcraft::Tool for InstallApiKeyTool {
    fn name(&self) -> &str { "install_api_key" }
    fn description(&self) -> &str {
        "Store an API key for an external service. The key name must be UPPER_SNAKE_CASE (e.g. CLOUDFLARE_API_TOKEN, GITHUB_TOKEN). Keys are stored securely in the local database."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "service_name": {
                    "type": "string",
                    "description": "Service name in UPPER_SNAKE_CASE (e.g. CLOUDFLARE_API_TOKEN)"
                },
                "api_key": {
                    "type": "string",
                    "description": "The API key value to store"
                }
            },
            "required": ["service_name", "api_key"]
        })
    }
    async fn call(&self, args: serde_json::Value) -> metalcraft::Result<serde_json::Value> {
        let service_name = args["service_name"].as_str().ok_or_else(|| metalcraft::GraphError::ToolCallFailed {
            tool: "install_api_key".into(),
            message: "Missing required parameter: service_name".into(),
        })?;
        let api_key = args["api_key"].as_str().ok_or_else(|| metalcraft::GraphError::ToolCallFailed {
            tool: "install_api_key".into(),
            message: "Missing required parameter: api_key".into(),
        })?;

        if !is_upper_snake_case(service_name) {
            return Ok(serde_json::json!({
                "error": format!("Invalid service name '{}'. Must be UPPER_SNAKE_CASE (e.g. CLOUDFLARE_API_TOKEN)", service_name)
            }));
        }

        if api_key.is_empty() {
            return Ok(serde_json::json!({
                "error": "API key cannot be empty"
            }));
        }

        let db = Database::open(&self.db_path).map_err(|e| metalcraft::GraphError::ToolCallFailed {
            tool: "install_api_key".into(),
            message: format!("Failed to open database: {}", e),
        })?;

        db.upsert_api_key(service_name, api_key).map_err(|e| metalcraft::GraphError::ToolCallFailed {
            tool: "install_api_key".into(),
            message: format!("Failed to store key: {}", e),
        })?;

        Ok(serde_json::json!({
            "success": true,
            "service_name": service_name,
            "masked_key": mask_key(api_key),
            "message": format!("API key for {} installed successfully ({})", service_name, mask_key(api_key))
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upper_snake_case() {
        assert!(is_upper_snake_case("CLOUDFLARE_API_TOKEN"));
        assert!(is_upper_snake_case("GITHUB_TOKEN"));
        assert!(is_upper_snake_case("API_KEY_V2"));
        assert!(!is_upper_snake_case("lowercase"));
        assert!(!is_upper_snake_case("Mixed_Case"));
        assert!(!is_upper_snake_case(""));
        assert!(!is_upper_snake_case("_LEADING_UNDERSCORE"));
    }
}
