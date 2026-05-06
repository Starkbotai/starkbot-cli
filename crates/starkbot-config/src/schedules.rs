use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
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

// --- Flow graph traversal ---

/// Walk a flow graph starting from the Entry node, following edges in BFS order,
/// and collect the prompt text from each Prompt node encountered.
/// Returns prompt texts in traversal order. Non-prompt nodes are skipped.
pub fn walk_flow_prompts(flow: &FlowDefinition) -> Vec<String> {
    // Build adjacency list: source -> [target, ...]
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in &flow.edges {
        adj.entry(edge.source.as_str()).or_default().push(edge.target.as_str());
    }

    // Index nodes by id
    let node_map: HashMap<&str, &FlowNode> = flow.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // Find entry node
    let entry = flow.nodes.iter().find(|n| matches!(n.node_type, FlowNodeType::Entry));
    let entry = match entry {
        Some(e) => e,
        None => return vec![],
    };

    // BFS from entry
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    queue.push_back(entry.id.as_str());
    visited.insert(entry.id.as_str());

    let mut prompts = Vec::new();

    while let Some(current_id) = queue.pop_front() {
        if let Some(node) = node_map.get(current_id) {
            if matches!(node.node_type, FlowNodeType::Prompt) {
                // Extract prompt text from node data
                let text = node.data.get("prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !text.is_empty() {
                    prompts.push(text);
                }
            }
        }

        if let Some(targets) = adj.get(current_id) {
            for &target in targets {
                if visited.insert(target) {
                    queue.push_back(target);
                }
            }
        }
    }

    prompts
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
    pub run_id: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_node(id: &str, node_type: FlowNodeType, data: serde_json::Value) -> FlowNode {
        FlowNode { id: id.to_string(), node_type, data, position: [0.0, 0.0] }
    }

    fn make_edge(id: &str, source: &str, target: &str) -> FlowEdge {
        FlowEdge {
            id: id.to_string(),
            source: source.to_string(),
            target: target.to_string(),
            source_handle: None,
            target_handle: None,
        }
    }

    #[test]
    fn walk_flow_prompts_empty_flow() {
        let flow = FlowDefinition { nodes: vec![], edges: vec![] };
        assert!(walk_flow_prompts(&flow).is_empty());
    }

    #[test]
    fn walk_flow_prompts_no_entry_node() {
        let flow = FlowDefinition {
            nodes: vec![make_node("p1", FlowNodeType::Prompt, json!({"prompt": "hello"}))],
            edges: vec![],
        };
        assert!(walk_flow_prompts(&flow).is_empty());
    }

    #[test]
    fn walk_flow_prompts_entry_only() {
        let flow = FlowDefinition {
            nodes: vec![make_node("entry", FlowNodeType::Entry, json!({}))],
            edges: vec![],
        };
        assert!(walk_flow_prompts(&flow).is_empty());
    }

    #[test]
    fn walk_flow_prompts_linear_chain() {
        let flow = FlowDefinition {
            nodes: vec![
                make_node("entry", FlowNodeType::Entry, json!({})),
                make_node("p1", FlowNodeType::Prompt, json!({"prompt": "first prompt"})),
                make_node("p2", FlowNodeType::Prompt, json!({"prompt": "second prompt"})),
            ],
            edges: vec![
                make_edge("e1", "entry", "p1"),
                make_edge("e2", "p1", "p2"),
            ],
        };
        let result = walk_flow_prompts(&flow);
        assert_eq!(result, vec!["first prompt", "second prompt"]);
    }

    #[test]
    fn walk_flow_prompts_skips_empty_prompt() {
        let flow = FlowDefinition {
            nodes: vec![
                make_node("entry", FlowNodeType::Entry, json!({})),
                make_node("p1", FlowNodeType::Prompt, json!({"prompt": ""})),
                make_node("p2", FlowNodeType::Prompt, json!({"prompt": "real prompt"})),
            ],
            edges: vec![
                make_edge("e1", "entry", "p1"),
                make_edge("e2", "p1", "p2"),
            ],
        };
        let result = walk_flow_prompts(&flow);
        assert_eq!(result, vec!["real prompt"]);
    }

    #[test]
    fn walk_flow_prompts_skips_branch_nodes() {
        let flow = FlowDefinition {
            nodes: vec![
                make_node("entry", FlowNodeType::Entry, json!({})),
                make_node("b1", FlowNodeType::Branch, json!({})),
                make_node("p1", FlowNodeType::Prompt, json!({"prompt": "after branch"})),
            ],
            edges: vec![
                make_edge("e1", "entry", "b1"),
                make_edge("e2", "b1", "p1"),
            ],
        };
        let result = walk_flow_prompts(&flow);
        assert_eq!(result, vec!["after branch"]);
    }

    #[test]
    fn walk_flow_prompts_disconnected_prompt_ignored() {
        let flow = FlowDefinition {
            nodes: vec![
                make_node("entry", FlowNodeType::Entry, json!({})),
                make_node("p1", FlowNodeType::Prompt, json!({"prompt": "connected"})),
                make_node("p2", FlowNodeType::Prompt, json!({"prompt": "disconnected"})),
            ],
            edges: vec![
                make_edge("e1", "entry", "p1"),
                // p2 has no edge from the graph
            ],
        };
        let result = walk_flow_prompts(&flow);
        assert_eq!(result, vec!["connected"]);
    }

    #[test]
    fn walk_flow_prompts_no_cycle_loop() {
        // Entry -> p1 -> p2 -> p1 (cycle) — should not infinite loop
        let flow = FlowDefinition {
            nodes: vec![
                make_node("entry", FlowNodeType::Entry, json!({})),
                make_node("p1", FlowNodeType::Prompt, json!({"prompt": "one"})),
                make_node("p2", FlowNodeType::Prompt, json!({"prompt": "two"})),
            ],
            edges: vec![
                make_edge("e1", "entry", "p1"),
                make_edge("e2", "p1", "p2"),
                make_edge("e3", "p2", "p1"), // cycle
            ],
        };
        let result = walk_flow_prompts(&flow);
        assert_eq!(result, vec!["one", "two"]);
    }

    #[test]
    fn walk_flow_prompts_missing_prompt_field() {
        let flow = FlowDefinition {
            nodes: vec![
                make_node("entry", FlowNodeType::Entry, json!({})),
                make_node("p1", FlowNodeType::Prompt, json!({"label": "no prompt field"})),
            ],
            edges: vec![make_edge("e1", "entry", "p1")],
        };
        assert!(walk_flow_prompts(&flow).is_empty());
    }
}
