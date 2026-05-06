use serde::{Deserialize, Serialize};
use std::path::Path;

/// Validate that an ID is safe for use as a filename (UUID-like: alphanumeric + hyphens).
fn is_safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub persona: String,
    pub title: String,
    pub created_at: String,
    pub messages: Vec<ChatSessionMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub persona: String,
    pub title: String,
    pub created_at: String,
    pub message_count: usize,
}

pub fn save_session(dir: &Path, session: &ChatSession) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create sessions dir: {}", e))?;
    let path = dir.join(format!("{}.json", session.id));
    let json = serde_json::to_string_pretty(session)
        .map_err(|e| format!("Failed to serialize session: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write session {}: {}", path.display(), e))
}

pub fn list_sessions(dir: &Path) -> Vec<SessionSummary> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };
    let mut summaries: Vec<SessionSummary> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .filter_map(|e| {
            let data = std::fs::read_to_string(e.path()).ok()?;
            let session: ChatSession = serde_json::from_str(&data).ok()?;
            Some(SessionSummary {
                id: session.id,
                persona: session.persona,
                title: session.title,
                created_at: session.created_at,
                message_count: session.messages.len(),
            })
        })
        .collect();
    summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    summaries
}

pub fn load_session(dir: &Path, id: &str) -> Option<ChatSession> {
    if !is_safe_id(id) { return None; }
    let path = dir.join(format!("{}.json", id));
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn delete_session(dir: &Path, id: &str) -> bool {
    if !is_safe_id(id) { return false; }
    let path = dir.join(format!("{}.json", id));
    std::fs::remove_file(path).is_ok()
}
