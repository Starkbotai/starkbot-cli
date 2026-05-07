use serde::{Deserialize, Serialize};

/// Events emitted from the engine to any frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendEvent {
    /// A tool call was made by the agent.
    ToolCall { name: String, args: String },
    /// A tool call completed.
    ToolResult { name: String, success: bool, preview: String },
    /// The agent finished its turn.
    TurnComplete { answer: String },
    /// The agent emitted reasoning/thinking text.
    ThinkingText { content: String },
    /// An error occurred during agent execution.
    Error { message: String },
    /// A tool needs user approval before proceeding.
    ApprovalRequired { request_id: String, tool_name: String, args_display: String },
    /// The model was changed.
    ModelChanged { model: String },
    /// Status update (busy/idle).
    StatusUpdate { busy: bool, message: String },
    /// An informational message from the engine.
    Info { message: String },
    /// Snapshot of current app state (response to RequestSnapshot).
    Snapshot(Box<crate::types::AppSnapshot>),
    /// Debug log entry for frontends to display.
    DebugLog { timestamp: String, level: String, message: String },
    /// A chat session was loaded from disk.
    SessionLoaded(Box<crate::types::ChatSession>),
    /// The sessions list was updated.
    SessionsUpdated(Vec<crate::types::SessionSummary>),
    /// A flow was loaded from disk.
    FlowLoaded(Box<crate::types::SavedFlow>),
    /// The flows list was updated.
    FlowsListed(Vec<crate::types::FlowSummary>),
    /// Flow logs loaded.
    FlowLogsLoaded(Vec<crate::types::FlowLogEntry>),
    /// Number of currently running flows changed.
    FlowRunningCount { count: usize },
    /// Available flow templates from installed integrations.
    FlowTemplatesListed(Vec<crate::types::FlowTemplateInfo>),
    /// The integrations list was updated.
    IntegrationsUpdated(Vec<crate::types::IntegrationPresetInfo>),
    /// Internal events log updated.
    EventsLogUpdated(Vec<crate::types::InternalEventDto>),
    /// Remote packs list fetched from extension server.
    PacksListed(Vec<crate::types::PackInfo>),
    /// A pack was installed successfully.
    PackInstalled { slug: String },
    /// A pack operation failed.
    PackError { message: String },
}

/// Commands sent from any frontend to the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FrontendCommand {
    /// Send a chat message to the agent.
    SendMessage { content: String },
    /// Respond to an approval request.
    ApprovalResponse { request_id: String, approved: bool },
    /// Switch the LLM model.
    SwitchModel { model: String },
    /// Switch persona (requires engine rebuild).
    SwitchPersona { slug: String },
    /// Execute a slash command (/clear, /tokens, etc.).
    SlashCommand { command: String },
    /// Add or update an API key.
    ApiKeyAdd { name: String, key: String },
    /// Delete an API key.
    ApiKeyDelete { name: String },
    /// Request a full state snapshot.
    RequestSnapshot,
    /// Load a saved chat session.
    LoadSession { session_id: String },
    /// Delete a saved chat session.
    DeleteSession { session_id: String },
    /// Save a flow definition.
    FlowSave { flow: crate::types::SavedFlow },
    /// Load a flow definition.
    FlowLoad { flow_id: String },
    /// Delete a flow definition.
    FlowDelete { flow_id: String },
    /// Toggle a flow's enabled state.
    FlowToggleEnabled { flow_id: String },
    /// Load flow logs.
    FlowLogsLoad,
    /// Run a flow once immediately.
    FlowRunOnce { flow_id: String },
    /// List all saved flows.
    FlowList,
    /// Install an integration preset.
    IntegrationInstall { preset_id: String, api_keys: Vec<(String, String)> },
    /// Uninstall an integration preset.
    IntegrationUninstall { preset_id: String },
    /// Import a flow template from an installed integration preset.
    IntegrationImportFlow { preset_id: String },
    /// List available flow templates from installed integrations.
    FlowListTemplates,
    /// Load the internal events log.
    EventsLogLoad,
    /// Fetch available packs from the extension server.
    PacksList,
    /// Install a pack from the extension server by slug.
    PackInstall { slug: String },
    /// Uninstall a local pack by slug.
    PackUninstall { slug: String },
    /// Shutdown the engine.
    Shutdown,
}
