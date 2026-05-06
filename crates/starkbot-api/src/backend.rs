use crate::events::{BackendEvent, FrontendCommand};
use crate::types::AppSnapshot;
use tokio::sync::mpsc;

/// Configuration for creating a backend engine.
pub struct BackendConfig {
    /// Persona slug to load (e.g. "starkbot").
    pub persona_slug: String,
    /// OpenAI API key.
    pub api_key: String,
    /// Model name (e.g. "gpt-5.4").
    pub model_name: String,
    /// Whether to auto-approve all tool calls.
    pub auto_approve: bool,
}

/// Handle returned by `Backend::start()` for the frontend to interact with the engine.
pub struct BackendHandle {
    /// Receive events from the engine.
    pub events: mpsc::UnboundedReceiver<BackendEvent>,
    /// Send commands to the engine.
    pub commands: mpsc::UnboundedSender<FrontendCommand>,
    /// Initial snapshot of the app state.
    pub initial_snapshot: AppSnapshot,
}

/// Trait for the backend engine. Any frontend can use this.
#[async_trait::async_trait]
pub trait Backend: Send {
    /// Start the engine and return channels for communication.
    async fn start(&mut self) -> anyhow::Result<BackendHandle>;
    /// Get a snapshot of the current state.
    fn snapshot(&self) -> AppSnapshot;
    /// Shut down the engine.
    async fn shutdown(&mut self);
}
