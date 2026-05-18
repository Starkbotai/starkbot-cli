use async_trait::async_trait;
use std::path::PathBuf;

/// Tool to list available skill test suites.
pub struct ListSkillTestsTool {
    skill_tests_dir: PathBuf,
}

impl ListSkillTestsTool {
    pub fn new(skill_tests_dir: PathBuf) -> Self {
        Self { skill_tests_dir }
    }
}

#[async_trait]
impl metalcraft::Tool for ListSkillTestsTool {
    fn name(&self) -> &str { "list_skill_tests" }
    fn description(&self) -> &str {
        "List all available skill test suites. Returns each suite's ID, name, and test count."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
    async fn call(&self, _args: serde_json::Value) -> metalcraft::Result<serde_json::Value> {
        let dir = &self.skill_tests_dir;
        if !dir.is_dir() {
            return Ok(serde_json::json!({ "tests": [] }));
        }
        let mut suites = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("ron") {
                    continue;
                }
                let id = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let (name, test_count) = match ron::from_str::<RonSuiteDef>(&content) {
                        Ok(suite) => (suite.name.clone(), suite.tests.len()),
                        Err(_) => (id.clone(), 0),
                    };
                    suites.push(serde_json::json!({
                        "id": id,
                        "name": name,
                        "test_count": test_count,
                    }));
                }
            }
        }
        Ok(serde_json::json!({ "tests": suites }))
    }
}

/// Tool to create a new skill test suite from JSON parameters.
pub struct CreateSkillTestTool {
    skill_tests_dir: PathBuf,
}

impl CreateSkillTestTool {
    pub fn new(skill_tests_dir: PathBuf) -> Self {
        Self { skill_tests_dir }
    }
}

#[async_trait]
impl metalcraft::Tool for CreateSkillTestTool {
    fn name(&self) -> &str { "create_skill_test" }
    fn description(&self) -> &str {
        "Create a new skill test suite. Provide a name and list of test definitions. The suite will be saved as a RON file."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the test suite"
                },
                "tests": {
                    "type": "array",
                    "description": "Array of test definitions",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "Unique test ID (slug)" },
                            "name": { "type": "string", "description": "Human-readable test name" },
                            "prompt": { "type": "string", "description": "The prompt to send to the agent" },
                            "expect_tools": { "type": "array", "items": { "type": "string" }, "description": "Tools expected to be called" },
                            "forbid_tools": { "type": "array", "items": { "type": "string" }, "description": "Tools that must NOT be called" },
                            "expect_no_error": { "type": "boolean", "description": "Whether to expect no error" },
                            "retries": { "type": "integer", "description": "Number of retries" },
                            "tags": { "type": "array", "items": { "type": "string" }, "description": "Tags for filtering" },
                            "min_response_length": { "type": "integer", "description": "Minimum response length" }
                        },
                        "required": ["id", "name", "prompt"]
                    }
                }
            },
            "required": ["name", "tests"]
        })
    }
    async fn call(&self, args: serde_json::Value) -> metalcraft::Result<serde_json::Value> {
        let name = args["name"].as_str().ok_or_else(|| metalcraft::GraphError::ToolCallFailed {
            tool: "create_skill_test".into(), message: "Missing required parameter: name".into(),
        })?;

        let tests_val = args.get("tests").ok_or_else(|| metalcraft::GraphError::ToolCallFailed {
            tool: "create_skill_test".into(), message: "Missing required parameter: tests".into(),
        })?;

        let tests: Vec<RonTestDef> = serde_json::from_value(tests_val.clone()).map_err(|e| {
            metalcraft::GraphError::ToolCallFailed {
                tool: "create_skill_test".into(),
                message: format!("Invalid tests format: {}", e),
            }
        })?;

        let suite = RonSuiteDef {
            name: name.to_string(),
            tests,
        };

        let ron_str = ron::ser::to_string_pretty(&suite, ron::ser::PrettyConfig::default())
            .map_err(|e| metalcraft::GraphError::ToolCallFailed {
                tool: "create_skill_test".into(),
                message: format!("RON serialize error: {}", e),
            })?;

        let slug: String = name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        let slug = if slug.is_empty() { format!("test-{}", chrono::Utc::now().timestamp()) } else { slug };

        let _ = std::fs::create_dir_all(&self.skill_tests_dir);
        let path = self.skill_tests_dir.join(format!("{}.ron", slug));
        std::fs::write(&path, &ron_str).map_err(|e| metalcraft::GraphError::ToolCallFailed {
            tool: "create_skill_test".into(),
            message: format!("Failed to write: {}", e),
        })?;

        Ok(serde_json::json!({
            "created": true,
            "id": slug,
            "path": path.display().to_string(),
            "test_count": suite.tests.len(),
        }))
    }
}

/// Tool to edit an existing skill test suite by replacing its RON content.
pub struct EditSkillTestTool {
    skill_tests_dir: PathBuf,
}

impl EditSkillTestTool {
    pub fn new(skill_tests_dir: PathBuf) -> Self {
        Self { skill_tests_dir }
    }
}

#[async_trait]
impl metalcraft::Tool for EditSkillTestTool {
    fn name(&self) -> &str { "edit_skill_test" }
    fn description(&self) -> &str {
        "Edit an existing skill test suite by replacing its RON content. The content is validated before saving."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The test suite ID (filename stem)"
                },
                "content": {
                    "type": "string",
                    "description": "The full RON content to save"
                }
            },
            "required": ["id", "content"]
        })
    }
    async fn call(&self, args: serde_json::Value) -> metalcraft::Result<serde_json::Value> {
        let id = args["id"].as_str().ok_or_else(|| metalcraft::GraphError::ToolCallFailed {
            tool: "edit_skill_test".into(), message: "Missing required parameter: id".into(),
        })?;
        let content = args["content"].as_str().ok_or_else(|| metalcraft::GraphError::ToolCallFailed {
            tool: "edit_skill_test".into(), message: "Missing required parameter: content".into(),
        })?;

        // Validate RON parse
        let _suite: RonSuiteDef = ron::from_str(content).map_err(|e| {
            metalcraft::GraphError::ToolCallFailed {
                tool: "edit_skill_test".into(),
                message: format!("Invalid RON: {}", e),
            }
        })?;

        let path = self.skill_tests_dir.join(format!("{}.ron", id));
        if !path.exists() {
            return Ok(serde_json::json!({
                "error": format!("Test suite '{}' not found", id)
            }));
        }

        std::fs::write(&path, content).map_err(|e| metalcraft::GraphError::ToolCallFailed {
            tool: "edit_skill_test".into(),
            message: format!("Failed to write: {}", e),
        })?;

        Ok(serde_json::json!({
            "updated": true,
            "id": id,
        }))
    }
}

// --- Internal RON types (mirroring starkbot-api::ron_defs but local to avoid dep cycle) ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RonTestDef {
    id: String,
    name: String,
    prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expect_tools: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    forbid_tools: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expect_no_error: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    retries: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    config: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    min_response_length: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RonSuiteDef {
    name: String,
    tests: Vec<RonTestDef>,
}
