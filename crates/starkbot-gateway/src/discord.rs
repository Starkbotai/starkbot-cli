use crate::manager::MessageHandler;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Start a Discord bot channel (stub).
pub async fn start(
    channel_id: String,
    channel_name: String,
    _bot_token: String,
    _safe_mode: bool,
    _handler: Arc<dyn MessageHandler>,
    shutdown_rx: oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    log::info!("[gateway:discord] '{}' (id={}) — not yet implemented, waiting for shutdown", channel_name, channel_id);
    let _ = shutdown_rx.await;
    log::info!("[gateway:discord] '{}' shut down", channel_name);
    Ok(())
}
