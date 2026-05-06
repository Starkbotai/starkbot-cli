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

fn setup_logging() {
    use std::io::Write;
    use env_logger::Builder;
    use log::LevelFilter;

    let log_path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("starkbot-cli")
        .join("debug.log");

    // Ensure parent dir exists
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Truncate on startup
    let log_file = std::fs::File::create(&log_path).ok();

    let log_file = std::sync::Arc::new(std::sync::Mutex::new(log_file));

    Builder::new()
        .filter_module("starkbot", LevelFilter::Info)
        .filter_module("metalcraft", LevelFilter::Info)
        .filter_level(LevelFilter::Warn)
        .format({
            let log_file = log_file.clone();
            move |buf, record| {
                let line = format!(
                    "[{} {} {}] {}\n",
                    chrono::Local::now().format("%H:%M:%S%.3f"),
                    record.level(),
                    record.module_path().unwrap_or("?"),
                    record.args()
                );
                // Write to stderr (normal env_logger behavior)
                let _ = buf.write_all(line.as_bytes());
                // Also write to file
                if let Ok(mut guard) = log_file.lock() {
                    if let Some(ref mut f) = *guard {
                        let _ = f.write_all(line.as_bytes());
                        let _ = f.flush();
                    }
                }
                Ok(())
            }
        })
        .init();

    log::info!("Logging initialized, file: {}", log_path.display());
}

fn main() {
    setup_logging();
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
                log::info!("[tauri] Starting event forwarding loop");
                while let Some(event) = event_rx.recv().await {
                    log::info!("[tauri] Emitting: {:?}", format!("{:?}", &event).chars().take(100).collect::<String>());
                    match handle.emit("backend-event", &event) {
                        Ok(_) => {}
                        Err(e) => log::error!("[tauri] emit failed: {}", e),
                    }
                }
                log::warn!("[tauri] Event forwarding loop ended (channel closed)");
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
