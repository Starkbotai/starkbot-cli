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
    Prompt,
    Branch,
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
#[serde(tag = "type", content = "value")]
pub enum Schedule {
    #[serde(rename = "every_minutes")]
    EveryMinutes(u32),
    #[serde(rename = "every_hours")]
    EveryHours(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub schedule: Schedule,
    pub flow: FlowDefinition,
    pub created_at: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskSummary {
    pub id: String,
    pub name: String,
    pub schedule: Schedule,
    pub node_count: usize,
    pub enabled: bool,
    pub created_at: String,
}

pub fn save_schedule(dir: &Path, task: &ScheduledTask) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create schedules dir: {}", e))?;
    let path = dir.join(format!("{}.json", task.id));
    let json = serde_json::to_string_pretty(task)
        .map_err(|e| format!("Failed to serialize schedule: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write schedule {}: {}", path.display(), e))
}

pub fn list_schedules(dir: &Path) -> Vec<ScheduledTaskSummary> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };
    let mut summaries: Vec<ScheduledTaskSummary> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .filter_map(|e| {
            let data = std::fs::read_to_string(e.path()).ok()?;
            let task: ScheduledTask = serde_json::from_str(&data).ok()?;
            Some(ScheduledTaskSummary {
                id: task.id,
                name: task.name,
                schedule: task.schedule,
                node_count: task.flow.nodes.len(),
                enabled: task.enabled,
                created_at: task.created_at,
            })
        })
        .collect();
    summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    summaries
}

pub fn load_schedule(dir: &Path, id: &str) -> Option<ScheduledTask> {
    if !is_safe_id(id) { return None; }
    let path = dir.join(format!("{}.json", id));
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn delete_schedule(dir: &Path, id: &str) -> bool {
    if !is_safe_id(id) { return false; }
    let path = dir.join(format!("{}.json", id));
    std::fs::remove_file(path).is_ok()
}
