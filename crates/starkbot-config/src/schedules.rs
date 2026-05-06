use serde::{Deserialize, Serialize};
use std::path::Path;

/// Validate that an ID is safe for use as a filename (UUID-like: alphanumeric + hyphens).
fn is_safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNode {
    pub id: String,
    pub node_type: FlowNodeType,
    pub data: serde_json::Value,
    #[serde(default)]
    pub position: [f64; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowNodeType {
    Entry,
    Prompt,
    Branch,
    BranchTool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub source_handle: Option<String>,
    #[serde(default)]
    pub target_handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowDefinition {
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedFlow {
    pub id: String,
    pub name: String,
    pub flow: FlowDefinition,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowSummary {
    pub id: String,
    pub name: String,
    pub node_count: usize,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub enabled: bool,
}

// --- Flow CRUD ---

pub fn save_flow(dir: &Path, flow: &SavedFlow) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create flows dir: {}", e))?;
    let path = dir.join(format!("{}.json", flow.id));
    let json = serde_json::to_string_pretty(flow)
        .map_err(|e| format!("Failed to serialize flow: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write flow {}: {}", path.display(), e))
}

pub fn load_flow(dir: &Path, id: &str) -> Option<SavedFlow> {
    if !is_safe_id(id) { return None; }
    let path = dir.join(format!("{}.json", id));
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn list_flows(dir: &Path) -> Vec<FlowSummary> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };
    let mut summaries: Vec<FlowSummary> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .filter_map(|e| {
            let data = std::fs::read_to_string(e.path()).ok()?;
            let flow: SavedFlow = serde_json::from_str(&data).ok()?;
            Some(FlowSummary {
                id: flow.id,
                name: flow.name,
                node_count: flow.flow.nodes.len(),
                created_at: flow.created_at,
                updated_at: flow.updated_at,
                enabled: flow.enabled,
            })
        })
        .collect();
    summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    summaries
}

pub fn delete_flow(dir: &Path, id: &str) -> bool {
    if !is_safe_id(id) { return false; }
    let path = dir.join(format!("{}.json", id));
    std::fs::remove_file(path).is_ok()
}

// --- Flow Logs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowLogEntry {
    pub timestamp: String,
    pub flow_id: String,
    pub flow_name: String,
    pub action: String,
    pub detail: String,
}

pub fn append_flow_log(log_path: &Path, entry: &FlowLogEntry) {
    let mut entries = load_flow_logs(log_path);
    entries.push(entry.clone());
    // Keep last 500 entries
    if entries.len() > 500 {
        entries = entries.split_off(entries.len() - 500);
    }
    if let Ok(json) = serde_json::to_string_pretty(&entries) {
        let _ = std::fs::write(log_path, json);
    }
}

pub fn load_flow_logs(log_path: &Path) -> Vec<FlowLogEntry> {
    if !log_path.exists() {
        return vec![];
    }
    std::fs::read_to_string(log_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
