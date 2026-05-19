use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_CONTENT_CHARS: usize = 50_000;

pub struct WebFetchTool {
    keys_path: Option<PathBuf>,
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self { keys_path: None }
    }
}

impl WebFetchTool {
    pub fn new(keys_path: PathBuf) -> Self {
        Self { keys_path: Some(keys_path) }
    }

    /// Resolve an auth spec like "bearer:CLOUDFLARE_API_TOKEN" into a header value.
    fn resolve_auth(&self, auth: &str) -> Result<(String, String), String> {
        let (scheme, key_name) = auth.split_once(':')
            .ok_or_else(|| format!("Invalid auth format '{}'. Expected 'bearer:<KEY_NAME>'", auth))?;

        let keys_path = self.keys_path.as_ref()
            .ok_or_else(|| "No keystore configured — cannot resolve auth".to_string())?;

        let store = starkbot_config::keys::KeyStore::load(keys_path)
            .map_err(|e| format!("Failed to load keystore: {}", e))?;

        let value = store.get(key_name)
            .ok_or_else(|| format!("Key '{}' not found in keystore", key_name))?;

        match scheme.to_lowercase().as_str() {
            "bearer" => Ok(("Authorization".to_string(), format!("Bearer {}", value))),
            "basic" => Ok(("Authorization".to_string(), format!("Basic {}", value))),
            "token" => Ok(("Authorization".to_string(), format!("token {}", value))),
            "header" => {
                // Format: "header:KEY_NAME" sets X-API-Key style header
                Ok(("X-API-Key".to_string(), value.to_string()))
            }
            "raw" => {
                // Format: "raw:KEY_NAME" sets Authorization header with just the key value (no prefix)
                Ok(("Authorization".to_string(), value.to_string()))
            }
            other => Err(format!("Unknown auth scheme '{}'. Use bearer, basic, token, raw, or header.", other)),
        }
    }
}

#[async_trait]
impl metalcraft::Tool for WebFetchTool {
    fn name(&self) -> &str { "web_fetch" }
    fn description(&self) -> &str {
        "Fetch content from a URL. Supports GET/POST/PUT/DELETE/PATCH with custom headers and body. Use the 'auth' parameter for authenticated requests — it resolves API keys from the keystore automatically (e.g. \"bearer:CLOUDFLARE_API_TOKEN\")."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "The URL to fetch" },
                "method": { "type": "string", "description": "HTTP method: GET, POST, PUT, DELETE, PATCH (default: GET)", "enum": ["GET", "POST", "PUT", "DELETE", "PATCH"] },
                "headers": { "type": "string", "description": "Custom headers as a JSON string of key-value pairs" },
                "auth": { "type": "string", "description": "Auth spec that resolves a key from the keystore. Format: 'scheme:KEY_NAME'. Schemes: bearer, basic, token, raw, header. Example: 'bearer:CLOUDFLARE_API_TOKEN' or 'raw:LINEAR_API_KEY'" },
                "body": { "type": "string", "description": "Request body for POST/PUT/PATCH requests" },
                "timeout_secs": { "type": "integer", "description": "Maximum time in seconds (default 30)" }
            },
            "required": ["url"]
        })
    }
    async fn call(&self, args: serde_json::Value) -> metalcraft::Result<serde_json::Value> {
        let url = args["url"].as_str().ok_or_else(|| metalcraft::GraphError::ToolCallFailed {
            tool: "web_fetch".into(), message: "Missing required parameter: url".into(),
        })?;
        let method = args["method"].as_str().unwrap_or("GET").to_uppercase();
        let timeout = args["timeout_secs"].as_u64().unwrap_or(DEFAULT_TIMEOUT_SECS).min(60);

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .user_agent("starkbot-cli/0.1 (web_fetch tool)")
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| metalcraft::GraphError::ToolCallFailed {
                tool: "web_fetch".into(), message: format!("Failed to create HTTP client: {}", e),
            })?;

        let mut request = match method.as_str() {
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            "PATCH" => client.patch(url),
            _ => client.get(url),
        };

        // Apply custom headers (accept JSON string or object)
        let parsed_headers: HashMap<String, String> = if let Some(headers_str) = args["headers"].as_str() {
            serde_json::from_str(headers_str).unwrap_or_default()
        } else if let Some(headers_obj) = args["headers"].as_object() {
            headers_obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        } else {
            HashMap::new()
        };
        for (key, value) in &parsed_headers {
            request = request.header(key.as_str(), value.as_str());
        }

        // Resolve auth from keystore if provided
        if let Some(auth_spec) = args["auth"].as_str() {
            let (header_name, header_value) = self.resolve_auth(auth_spec)
                .map_err(|e| metalcraft::GraphError::ToolCallFailed {
                    tool: "web_fetch".into(), message: e,
                })?;
            request = request.header(header_name.as_str(), header_value.as_str());
        }

        // Apply body
        if let Some(body) = args["body"].as_str() {
            request = request.body(body.to_string());
            // Auto-set Content-Type if not already provided
            if !parsed_headers.contains_key("Content-Type")
                && !parsed_headers.contains_key("content-type")
            {
                request = request.header("Content-Type", "application/json");
            }
        }

        let response = request.send().await.map_err(|e| metalcraft::GraphError::ToolCallFailed {
            tool: "web_fetch".into(), message: format!("Request failed: {}", e),
        })?;

        let status = response.status();
        let final_url = response.url().to_string();
        let status_code = status.as_u16();

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            let error_body = crate::truncate_output(&error_body, 2000);
            return Ok(serde_json::json!({
                "error": format!("HTTP {}", status),
                "status_code": status_code,
                "url": final_url,
                "body": error_body
            }));
        }

        let content_type = response.headers().get("content-type")
            .and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        let body = response.text().await.map_err(|e| metalcraft::GraphError::ToolCallFailed {
            tool: "web_fetch".into(), message: format!("Failed to read response body: {}", e),
        })?;

        let content = if content_type.contains("text/html") {
            html_to_markdown(&body)
        } else {
            body
        };

        let content = crate::truncate_output(&content, MAX_CONTENT_CHARS);
        Ok(serde_json::json!({
            "url": final_url,
            "status_code": status_code,
            "content": content
        }))
    }
}

fn html_to_markdown(html: &str) -> String {
    use htmd::HtmlToMarkdown;
    let converter = HtmlToMarkdown::builder()
        .skip_tags(vec!["script", "style", "nav", "footer", "header"])
        .build();
    match converter.convert(html) {
        Ok(md) => md,
        Err(_) => {
            // Fallback: naive tag stripping
            let mut result = String::with_capacity(html.len());
            let mut in_tag = false;
            for ch in html.chars() {
                match ch {
                    '<' => in_tag = true,
                    '>' => in_tag = false,
                    _ if !in_tag => result.push(ch),
                    _ => {}
                }
            }
            result.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
        }
    }
}
