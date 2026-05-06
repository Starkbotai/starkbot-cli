// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use starkbot_api::{Backend, BackendConfig, FrontendCommand};
use starkbot_api::engine::StarkbotEngine;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;

/// Shared state accessible from Tauri commands.
struct AppState {
    cmd_tx: mpsc::UnboundedSender<FrontendCommand>,
}

/// Tauri command: send a chat message.
#[tauri::command]
async fn send_message(content: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::SendMessage { content })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: respond to an approval request.
#[tauri::command]
async fn approval_response(request_id: String, approved: bool, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::ApprovalResponse { request_id, approved })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: switch model.
#[tauri::command]
async fn switch_model(model: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::SwitchModel { model })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: add an API key.
#[tauri::command]
async fn api_key_add(name: String, key: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::ApiKeyAdd { name, key })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: delete an API key.
#[tauri::command]
async fn api_key_delete(name: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::ApiKeyDelete { name })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: request a state snapshot.
#[tauri::command]
async fn request_snapshot(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::RequestSnapshot)
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: get the initial snapshot (called once on frontend mount).
#[tauri::command]
async fn get_initial_snapshot(state: tauri::State<'_, Arc<starkbot_api::types::AppSnapshot>>) -> Result<starkbot_api::types::AppSnapshot, String> {
    Ok((**state).clone())
}

fn main() {
    env_logger::init();
    dotenvy::dotenv().ok();

    let persona_slug = std::env::var("STARKBOT_PERSONA").unwrap_or_else(|_| "starkbot".to_string());

    tauri::Builder::default()
        .setup(move |app| {
            let handle = app.handle().clone();

            // Start engine in async context
            tauri::async_runtime::spawn(async move {
                let config = BackendConfig {
                    persona_slug,
                    api_key: String::new(),
                    model_name: String::new(),
                    auto_approve: false,
                };

                let mut engine = match StarkbotEngine::new(config) {
                    Ok(e) => e,
                    Err(e) => {
                        log::error!("Failed to create engine: {}", e);
                        return;
                    }
                };

                let backend_handle = match engine.start().await {
                    Ok(h) => h,
                    Err(e) => {
                        log::error!("Failed to start engine: {}", e);
                        return;
                    }
                };

                let snapshot = backend_handle.initial_snapshot.clone();
                let cmd_tx = backend_handle.commands.clone();
                let mut event_rx = backend_handle.events;

                // Store state for Tauri commands
                handle.manage(Arc::new(AppState { cmd_tx }));
                handle.manage(Arc::new(snapshot));

                // Forward backend events to the webview
                while let Some(event) = event_rx.recv().await {
                    if let Ok(json) = serde_json::to_string(&event) {
                        let _ = handle.emit("backend-event", json);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            send_message,
            approval_response,
            switch_model,
            api_key_add,
            api_key_delete,
            request_snapshot,
            get_initial_snapshot,
        ])
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tauri application");
}
