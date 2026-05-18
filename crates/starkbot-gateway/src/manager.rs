use crate::types::{ChannelHandle, ChannelType, NormalizedMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Trait for handling incoming gateway messages.
#[async_trait::async_trait]
pub trait MessageHandler: Send + Sync + 'static {
    async fn handle_message(&self, msg: NormalizedMessage) -> Result<String, String>;
}

/// Manages running gateway channels.
pub struct ChannelManager {
    channels: Arc<Mutex<HashMap<String, ChannelHandle>>>,
    handler: Arc<dyn MessageHandler>,
}

impl ChannelManager {
    pub fn new(handler: Arc<dyn MessageHandler>) -> Self {
        Self {
            channels: Arc::new(Mutex::new(HashMap::new())),
            handler,
        }
    }

    /// Start a channel. Returns an error if already running or if settings are missing.
    pub async fn start_channel(
        &self,
        channel_id: String,
        channel_type: ChannelType,
        name: String,
        settings: &HashMap<String, String>,
    ) -> anyhow::Result<()> {
        {
            let channels = self.channels.lock().await;
            if channels.contains_key(&channel_id) {
                anyhow::bail!("Channel '{}' is already running", name);
            }
        }

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let handler = self.handler.clone();
        let id = channel_id.clone();
        let n = name.clone();
        let safe_mode = settings.get("safe_mode").map(|v| v == "1" || v == "true").unwrap_or(true);

        let channels = self.channels.clone();
        let cid = channel_id.clone();

        match channel_type {
            ChannelType::Custom => {
                let port: u16 = settings
                    .get("listen_port")
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(9090);
                let auth_token = settings.get("auth_token").cloned().filter(|t| !t.is_empty());

                // Verify port is bindable before registering
                let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await
                    .map_err(|e| anyhow::anyhow!("Failed to bind port {}: {}", port, e))?;

                let handle = ChannelHandle {
                    channel_id: channel_id.clone(),
                    channel_type,
                    name,
                    shutdown_tx,
                };
                channels.lock().await.insert(channel_id, handle);

                tokio::spawn(async move {
                    if let Err(e) = crate::custom::start_with_listener(id, n.clone(), auth_token, safe_mode, handler, shutdown_rx, listener).await {
                        log::error!("[gateway] Custom channel '{}' error: {}", n, e);
                        // Remove from map on failure
                        channels.lock().await.remove(&cid);
                    }
                });
            }
            ChannelType::Discord => {
                let token = settings
                    .get("discord_bot_token")
                    .cloned()
                    .unwrap_or_default();
                if token.is_empty() {
                    anyhow::bail!("Discord Bot Token is required");
                }
                let handle = ChannelHandle {
                    channel_id: channel_id.clone(),
                    channel_type,
                    name,
                    shutdown_tx,
                };
                channels.lock().await.insert(channel_id, handle);

                tokio::spawn(async move {
                    if let Err(e) = crate::discord::start(id, n.clone(), token, safe_mode, handler, shutdown_rx).await {
                        log::error!("[gateway] Discord channel '{}' error: {}", n, e);
                        channels.lock().await.remove(&cid);
                    }
                });
            }
            ChannelType::Telegram => {
                let token = settings
                    .get("telegram_bot_token")
                    .cloned()
                    .unwrap_or_default();
                if token.is_empty() {
                    anyhow::bail!("Telegram Bot Token is required");
                }
                let handle = ChannelHandle {
                    channel_id: channel_id.clone(),
                    channel_type,
                    name,
                    shutdown_tx,
                };
                channels.lock().await.insert(channel_id, handle);

                tokio::spawn(async move {
                    if let Err(e) = crate::telegram::start(id, n.clone(), token, safe_mode, handler, shutdown_rx).await {
                        log::error!("[gateway] Telegram channel '{}' error: {}", n, e);
                        channels.lock().await.remove(&cid);
                    }
                });
            }
        }

        Ok(())
    }

    /// Stop a running channel.
    pub async fn stop_channel(&self, channel_id: &str) -> bool {
        let mut channels = self.channels.lock().await;
        if let Some(handle) = channels.remove(channel_id) {
            let _ = handle.shutdown_tx.send(());
            true
        } else {
            false
        }
    }

    /// Stop all running channels.
    pub async fn stop_all(&self) {
        let mut channels = self.channels.lock().await;
        for (_, handle) in channels.drain() {
            let _ = handle.shutdown_tx.send(());
        }
    }

    /// Check if a channel is running.
    pub async fn is_running(&self, channel_id: &str) -> bool {
        self.channels.lock().await.contains_key(channel_id)
    }

    /// Get IDs of all running channels.
    pub async fn running_ids(&self) -> Vec<String> {
        self.channels.lock().await.keys().cloned().collect()
    }
}
