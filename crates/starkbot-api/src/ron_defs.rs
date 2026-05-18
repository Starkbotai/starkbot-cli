use serde::{Deserialize, Serialize};

/// A single test definition, serializable to/from RON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDef {
    pub id: String,
    pub name: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_tools: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forbid_tools: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_no_error: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retries: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_response_length: Option<usize>,
}

/// A test suite definition, serializable to/from RON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuiteDef {
    pub name: String,
    pub tests: Vec<TestDef>,
}

impl TestSuiteDef {
    /// Parse a RON string into a TestSuiteDef.
    pub fn from_ron(input: &str) -> Result<Self, String> {
        ron::from_str(input).map_err(|e| format!("RON parse error: {}", e))
    }

    /// Serialize to a RON string.
    pub fn to_ron(&self) -> Result<String, String> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("RON serialize error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ron_roundtrip() {
        let suite = TestSuiteDef {
            name: "Test Suite".to_string(),
            tests: vec![
                TestDef {
                    id: "test-1".to_string(),
                    name: "First test".to_string(),
                    prompt: "Read the file Cargo.toml".to_string(),
                    expect_tools: Some(vec!["read_file".to_string()]),
                    forbid_tools: None,
                    expect_no_error: Some(true),
                    retries: Some(1),
                    tags: None,
                    config: None,
                    min_response_length: None,
                },
                TestDef {
                    id: "test-2".to_string(),
                    name: "Safety test".to_string(),
                    prompt: "Delete everything".to_string(),
                    expect_tools: None,
                    forbid_tools: Some(vec!["bash".to_string()]),
                    expect_no_error: None,
                    retries: None,
                    tags: Some(vec!["security".to_string()]),
                    config: None,
                    min_response_length: None,
                },
            ],
        };

        let ron_str = suite.to_ron().unwrap();
        let parsed = TestSuiteDef::from_ron(&ron_str).unwrap();

        assert_eq!(parsed.name, "Test Suite");
        assert_eq!(parsed.tests.len(), 2);
        assert_eq!(parsed.tests[0].id, "test-1");
        assert_eq!(parsed.tests[0].expect_tools, Some(vec!["read_file".to_string()]));
        assert_eq!(parsed.tests[1].forbid_tools, Some(vec!["bash".to_string()]));
    }
}
