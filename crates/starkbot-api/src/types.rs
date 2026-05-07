use serde::{Deserialize, Serialize};

pub use starkbot_config::sessions::{ChatSession, ChatSessionMessage, SessionSummary};
pub use starkbot_config::schedules::{
    FlowNode, FlowNodeType, FlowEdge, FlowDefinition,
    SavedFlow, FlowSummary, FlowLogEntry,
};

/// A chat message DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageDto {
    pub role: String,
    pub content: String,
}

/// Info about a persona for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaInfo {
    pub key: String,
    pub label: String,
    pub description: String,
    pub emoji: String,
    pub enabled: bool,
    pub tool_groups: Vec<String>,
    pub skill_tags: Vec<String>,
    pub system_prompt_preview: String,
}

/// Info about a skill for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    pub tags: Vec<String>,
    pub requires_tools: Vec<String>,
    pub requires_keys: Vec<String>,
    pub content: String,
}

/// Info about an API key for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub name: String,
    pub masked_key: String,
}

/// A graph node for visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodeDto {
    pub id: String,
    pub label: String,
    pub category: String,
    pub weight: f32,
}

/// A graph edge for visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdgeDto {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub kind: String,
    pub weight: f32,
}

/// A required key entry for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredKeyInfo {
    pub name: String,
    pub label: String,
}

/// Info about an integration preset for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationPresetInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub api_key_name: Option<String>,
    pub required_keys: Vec<RequiredKeyInfo>,
    pub skills: Vec<String>,
    pub installed: bool,
    pub configured: bool,
    pub has_flow_template: bool,
}

/// Info about a pack available on the extension server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackInfo {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub installed: bool,
}

/// Info about a flow template available from an installed integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowTemplateInfo {
    pub preset_id: String,
    pub preset_name: String,
    pub template_name: String,
}

/// A DTO for internal event bus events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalEventDto {
    pub timestamp: String,
    pub kind: String,
    pub payload: String,
}

/// Full application state snapshot for frontends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub persona_name: String,
    pub model_name: String,
    pub agent_busy: bool,
    pub status: String,
    pub messages: Vec<ChatMessageDto>,
    pub tool_activity: Vec<String>,
    pub skills: Vec<SkillInfo>,
    pub personas: Vec<PersonaInfo>,
    pub api_keys: Vec<ApiKeyInfo>,
    pub available_models: Vec<String>,
    pub graph_nodes: Vec<GraphNodeDto>,
    pub graph_edges: Vec<GraphEdgeDto>,
    pub skills_dir: String,
    pub agents_dir: String,
    pub sessions: Vec<SessionSummary>,
    pub sessions_dir: String,
    pub flows_dir: String,
    pub inference_configured: bool,
    pub integrations: Vec<IntegrationPresetInfo>,
    pub extension_server: String,
}
