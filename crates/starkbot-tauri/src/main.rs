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

/// Tauri command: execute a slash command.
#[tauri::command]
async fn slash_command(command: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::SlashCommand { command })
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

/// Tauri command: load a saved chat session.
#[tauri::command]
async fn load_session(session_id: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::LoadSession { session_id })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: delete a saved chat session.
#[tauri::command]
async fn delete_session(session_id: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::DeleteSession { session_id })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: save a flow definition.
#[tauri::command]
async fn flow_save(flow: starkbot_api::types::SavedFlow, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::FlowSave { flow })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: load a flow definition.
#[tauri::command]
async fn flow_load(flow_id: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::FlowLoad { flow_id })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: delete a flow definition.
#[tauri::command]
async fn flow_delete(flow_id: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::FlowDelete { flow_id })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: load flow logs.
#[tauri::command]
async fn flow_logs_load(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::FlowLogsLoad)
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: load internal events log.
#[tauri::command]
async fn events_log_load(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::EventsLogLoad)
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: run a flow once immediately.
#[tauri::command]
async fn flow_run_once(flow_id: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::FlowRunOnce { flow_id })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: toggle a flow's enabled state.
#[tauri::command]
async fn flow_toggle_enabled(flow_id: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::FlowToggleEnabled { flow_id })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: list all saved flows.
#[tauri::command]
async fn flow_list(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::FlowList)
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: install an integration preset.
#[tauri::command]
async fn integration_install(preset_id: String, api_keys: Vec<(String, String)>, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::IntegrationInstall { preset_id, api_keys })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: uninstall an integration preset.
#[tauri::command]
async fn integration_uninstall(preset_id: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::IntegrationUninstall { preset_id })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: import a flow template from an installed integration.
#[tauri::command]
async fn integration_import_flow(preset_id: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::IntegrationImportFlow { preset_id })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: list available flow templates from installed integrations.
#[tauri::command]
async fn flow_list_templates(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::FlowListTemplates)
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: create a gateway channel.
#[tauri::command]
async fn channel_create(channel_type: String, name: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::ChannelCreate { channel_type, name })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: delete a gateway channel.
#[tauri::command]
async fn channel_delete(channel_id: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::ChannelDelete { channel_id })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: start a gateway channel.
#[tauri::command]
async fn channel_start(channel_id: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::ChannelStart { channel_id })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: stop a gateway channel.
#[tauri::command]
async fn channel_stop(channel_id: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::ChannelStop { channel_id })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: update a channel setting.
#[tauri::command]
async fn channel_setting_update(channel_id: String, key: String, value: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::ChannelSettingUpdate { channel_id, key, value })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: load settings for a channel.
#[tauri::command]
async fn channel_settings_load(channel_id: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::ChannelSettingsLoad { channel_id })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: list all channels.
#[tauri::command]
async fn channels_list(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::ChannelsList)
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: fetch packs list from extension server.
#[tauri::command]
async fn packs_list(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::PacksList)
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: install a pack from extension server.
#[tauri::command]
async fn pack_install(slug: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::PackInstall { slug })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: uninstall a local pack.
#[tauri::command]
async fn pack_uninstall(slug: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cmd_tx.send(FrontendCommand::PackUninstall { slug })
        .map_err(|e| format!("Failed to send: {}", e))
}

/// Tauri command: list files in the custom/ directory.
#[tauri::command]
async fn list_custom_files(_state: tauri::State<'_, Arc<starkbot_api::types::AppSnapshot>>) -> Result<Vec<CustomFileEntry>, String> {
    let config = starkbot_config::AppConfig::open();
    let custom_dir = config.custom_dir();
    if !custom_dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    list_dir_recursive(&custom_dir, &custom_dir, &mut entries)?;
    Ok(entries)
}

/// Tauri command: read a custom file.
#[tauri::command]
async fn read_custom_file(path: String) -> Result<String, String> {
    let config = starkbot_config::AppConfig::open();
    let full_path = config.custom_dir().join(&path);
    // Security: ensure the path is within custom_dir
    let canonical = full_path.canonicalize().map_err(|e| format!("Invalid path: {}", e))?;
    let custom_canonical = config.custom_dir().canonicalize().map_err(|e| format!("Custom dir error: {}", e))?;
    if !canonical.starts_with(&custom_canonical) {
        return Err("Path traversal not allowed".to_string());
    }
    std::fs::read_to_string(&canonical).map_err(|e| format!("Failed to read: {}", e))
}

/// Tauri command: write a custom file.
#[tauri::command]
async fn write_custom_file(path: String, content: String) -> Result<(), String> {
    let config = starkbot_config::AppConfig::open();
    let full_path = config.custom_dir().join(&path);
    // Ensure parent exists
    if let Some(parent) = full_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Security: ensure the path is within custom_dir after creating parent
    let canonical = full_path.canonicalize().unwrap_or(full_path.clone());
    let custom_canonical = config.custom_dir().canonicalize().map_err(|e| format!("Custom dir error: {}", e))?;
    if !canonical.starts_with(&custom_canonical) {
        return Err("Path traversal not allowed".to_string());
    }
    std::fs::write(&full_path, content).map_err(|e| format!("Failed to write: {}", e))
}

#[derive(serde::Serialize)]
struct CustomFileEntry {
    path: String,
    name: String,
    is_dir: bool,
}

fn list_dir_recursive(base: &std::path::Path, dir: &std::path::Path, entries: &mut Vec<CustomFileEntry>) -> Result<(), String> {
    let read = std::fs::read_dir(dir).map_err(|e| format!("Failed to read {}: {}", dir.display(), e))?;
    for entry in read.filter_map(|e| e.ok()) {
        let path = entry.path();
        let rel = path.strip_prefix(base).unwrap_or(&path);
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = path.is_dir();
        entries.push(CustomFileEntry {
            path: rel.to_string_lossy().to_string(),
            name,
            is_dir,
        });
        if is_dir {
            list_dir_recursive(base, &path, entries)?;
        }
    }
    Ok(())
}

/// Tauri command: open a folder in the OS file manager.
#[tauri::command]
async fn open_folder(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| e.to_string())
}

/// Tauri command: open a URL in the default browser.
#[tauri::command]
async fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| e.to_string())
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
            slash_command,
            approval_response,
            switch_model,
            api_key_add,
            api_key_delete,
            request_snapshot,
            get_initial_snapshot,
            open_folder,
            open_url,
            load_session,
            delete_session,
            flow_save,
            flow_load,
            flow_delete,
            flow_list,
            flow_run_once,
            flow_toggle_enabled,
            flow_logs_load,
            events_log_load,
            integration_install,
            integration_uninstall,
            integration_import_flow,
            flow_list_templates,
            list_custom_files,
            read_custom_file,
            write_custom_file,
            packs_list,
            pack_install,
            pack_uninstall,
            channel_create,
            channel_delete,
            channel_start,
            channel_stop,
            channel_setting_update,
            channel_settings_load,
            channels_list,
        ])
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tauri application");
}
