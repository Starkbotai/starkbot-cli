use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_CONTENT_CHARS: usize = 50_000;

pub struct WebFetchTool;

#[async_trait]
impl metalcraft::Tool for WebFetchTool {
    fn name(&self) -> &str { "web_fetch" }
    fn description(&self) -> &str {
        "Fetch content from a URL and return it as markdown text. Works for public web pages."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "The URL to fetch" },
                "timeout_secs": { "type": "integer", "description": "Maximum time in seconds (default 30)" }
            },
            "required": ["url"]
        })
    }
    async fn call(&self, args: serde_json::Value) -> metalcraft::Result<serde_json::Value> {
        let url = args["url"].as_str().ok_or_else(|| metalcraft::GraphError::ToolCallFailed {
            tool: "web_fetch".into(), message: "Missing required parameter: url".into(),
        })?;
        let timeout = args["timeout_secs"].as_u64().unwrap_or(DEFAULT_TIMEOUT_SECS).min(60);

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .user_agent("starkbot-cli/0.1 (web_fetch tool)")
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| metalcraft::GraphError::ToolCallFailed {
                tool: "web_fetch".into(), message: format!("Failed to create HTTP client: {}", e),
            })?;

        let response = client.get(url).send().await.map_err(|e| metalcraft::GraphError::ToolCallFailed {
            tool: "web_fetch".into(), message: format!("Request failed: {}", e),
        })?;

        let status = response.status();
        let final_url = response.url().to_string();

        if !status.is_success() {
            return Ok(serde_json::json!({ "error": format!("HTTP {}", status), "url": final_url }));
        }

        let content_type = response.headers().get("content-type")
            .and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        let body = response.text().await.map_err(|e| metalcraft::GraphError::ToolCallFailed {
            tool: "web_fetch".into(), message: format!("Failed to read response body: {}", e),
        })?;

        let markdown = if content_type.contains("text/html") {
            html_to_markdown(&body)
        } else {
            body
        };

        let markdown = crate::truncate_output(&markdown, MAX_CONTENT_CHARS);
        Ok(serde_json::json!({ "url": final_url, "content": markdown }))
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
