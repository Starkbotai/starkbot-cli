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
    /// The scheduled tasks list was updated.
    SchedulesUpdated(Vec<crate::types::ScheduledTaskSummary>),
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
    /// Create a new scheduled task.
    ScheduleCreate { name: String, schedule: crate::types::Schedule, flow: crate::types::FlowDefinition },
    /// Delete a scheduled task.
    ScheduleDelete { task_id: String },
    /// Toggle a scheduled task's enabled state.
    ScheduleToggle { task_id: String },
    /// Shutdown the engine.
    Shutdown,
}
