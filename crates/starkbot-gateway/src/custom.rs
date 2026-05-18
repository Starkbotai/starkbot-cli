use crate::manager::MessageHandler;
use crate::types::{ChannelType, NormalizedMessage};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::oneshot;

#[derive(Debug, Deserialize)]
struct IncomingMessage {
    text: String,
    user_id: Option<String>,
    user_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct MessageResponse {
    response: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    channel: String,
}

struct AppState {
    channel_id: String,
    channel_name: String,
    safe_mode: bool,
    auth_token: Option<String>,
    handler: Arc<dyn MessageHandler>,
}

async fn health_handler(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        channel: state.channel_name.clone(),
    })
}

async fn message_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<IncomingMessage>,
) -> Result<Json<MessageResponse>, StatusCode> {
    // Check auth token if configured
    if let Some(ref expected_token) = state.auth_token {
        let auth_header = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth_header.strip_prefix("Bearer ").unwrap_or("");
        if token != expected_token {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    let msg_id = uuid::Uuid::new_v4().to_string();
    let msg = NormalizedMessage {
        channel_id: state.channel_id.clone(),
        channel_type: ChannelType::Custom,
        user_id: payload.user_id.unwrap_or_else(|| "anonymous".to_string()),
        user_name: payload.user_name.unwrap_or_else(|| "Anonymous".to_string()),
        text: payload.text,
        message_id: msg_id,
        safe_mode: state.safe_mode,
    };

    match state.handler.handle_message(msg).await {
        Ok(response) => Ok(Json(MessageResponse { response })),
        Err(e) => {
            log::error!("[gateway:custom] handler error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Start a custom HTTP channel with a pre-bound listener.
pub async fn start_with_listener(
    channel_id: String,
    channel_name: String,
    auth_token: Option<String>,
    safe_mode: bool,
    handler: Arc<dyn MessageHandler>,
    shutdown_rx: oneshot::Receiver<()>,
    listener: tokio::net::TcpListener,
) -> anyhow::Result<()> {
    let state = Arc::new(AppState {
        channel_id,
        channel_name: channel_name.clone(),
        safe_mode,
        auth_token,
        handler,
    });

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/message", post(message_handler))
        .with_state(state);

    log::info!("[gateway:custom] Starting '{}' on {:?}", channel_name, listener.local_addr());

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
            log::info!("[gateway:custom] Shutting down '{}'", channel_name);
        })
        .await?;

    Ok(())
}
