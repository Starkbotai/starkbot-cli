use crate::backend::{BackendConfig, BackendHandle};
use crate::events::{BackendEvent, FrontendCommand};
use crate::types::*;

use metalcraft::{AgentMessage, AgentState, RunOutcome};
use starkbot_core::context;
use starkbot_core::dispatch::AgentRunner;
use starkbot_core::persona::Persona;
use starkbot_graph::build_skill_graph;
use starkbot_skills::SkillRegistry;
use starkbot_tools::approval::{self, ApprovalMode};
use starkbot_config::keys::KeyStore;
use starkbot_config::AppConfig;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Shared state for pending approval response channels.
/// The approval poller inserts entries, the command handler removes them.
type PendingApprovals = Arc<Mutex<HashMap<String, std::sync::mpsc::SyncSender<bool>>>>;

/// The concrete backend engine. Wraps AgentRunner and owns all state.
pub struct StarkbotEngine {
    // Config
    persona: Persona,
    api_key: String,
    model_name: String,
    auto_approve: bool,

    // Runtime
    runner: Option<AgentRunner>,
    app_config: AppConfig,
    skills_dir: PathBuf,
    cwd: String,

    // Cached data for snapshots
    messages: Vec<ChatMessageDto>,
    tool_activity: Vec<String>,
    agent_busy: bool,
    status: String,
    skills: Vec<SkillInfo>,
    personas: Vec<PersonaInfo>,
    api_keys: Vec<ApiKeyInfo>,
    available_models: Vec<String>,
    graph_nodes: Vec<GraphNodeDto>,
    graph_edges: Vec<GraphEdgeDto>,

    // Approval tracking (shared with async tasks)
    pending_approvals: PendingApprovals,
    approval_counter: Arc<Mutex<u64>>,
}

impl StarkbotEngine {
    pub fn new(config: BackendConfig) -> anyhow::Result<Self> {
        let app_config = AppConfig::open();

        // Initialize config
        let bundled_agents = find_bundled_dir("agents");
        let bundled_skills = find_bundled_dir("skills");
        app_config.ensure_initialized(bundled_agents.as_deref(), bundled_skills.as_deref())
            .unwrap_or_else(|e| log::warn!("Config init failed: {}", e));

        // Migrate old keys
        migrate_keys_from_db(&app_config);

        let personas_dir = Persona::default_personas_dir();
        let config_agents_dir = app_config.agents_dir();

        // Skills dir
        let config_skills_dir = app_config.skills_dir();
        let skills_dir = if config_skills_dir.is_dir()
            && std::fs::read_dir(&config_skills_dir)
                .map(|mut d| d.next().is_some())
                .unwrap_or(false)
        {
            config_skills_dir
        } else {
            Persona::default_skills_dir()
        };

        // Load persona
        let persona = Persona::load_with_config(
            &config.persona_slug,
            Some(&config_agents_dir),
            &personas_dir,
        )
        .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Resolve API key
        let api_key = if !config.api_key.is_empty() {
            config.api_key.clone()
        } else {
            match std::env::var("OPENAI_API_KEY") {
                Ok(key) => key,
                Err(_) => {
                    let store = KeyStore::load(&app_config.keys_path()).unwrap_or_default();
                    store
                        .get("OPENAI_API_KEY")
                        .map(|s| s.to_string())
                        .ok_or_else(|| anyhow::anyhow!("OPENAI_API_KEY must be set"))?
                }
            }
        };

        // Resolve model
        let model_name = if !config.model_name.is_empty() {
            config.model_name.clone()
        } else {
            let settings =
                starkbot_config::settings::Settings::load(&app_config.settings_path()).unwrap_or_default();
            std::env::var("OPENAI_MODEL").unwrap_or(settings.model)
        };

        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string());

        // Populate skill FTS
        populate_skill_fts(&skills_dir);

        // Load skill data
        let skill_registry = SkillRegistry::load_from_dir(&skills_dir);
        let skills: Vec<SkillInfo> = {
            let mut s: Vec<_> = skill_registry
                .all()
                .iter()
                .map(|s| SkillInfo {
                    name: s.name.clone(),
                    description: s.description.clone(),
                    version: s.version.clone(),
                    tags: s.tags.clone(),
                    requires_tools: s.requires_tools.clone(),
                    content: s.content.clone(),
                })
                .collect();
            s.sort_by(|a, b| a.name.cmp(&b.name));
            s
        };

        // Build graph
        let skill_data: Vec<(String, String, Vec<String>)> = skill_registry
            .all()
            .iter()
            .map(|s| (s.name.clone(), s.description.clone(), s.tags.clone()))
            .collect();
        let graph = build_skill_graph(&skill_data);
        let graph_nodes: Vec<GraphNodeDto> = graph
            .nodes
            .iter()
            .map(|n| GraphNodeDto {
                id: n.id.clone(),
                label: n.label.clone(),
                category: n.category.clone(),
                weight: n.weight,
            })
            .collect();
        let graph_edges: Vec<GraphEdgeDto> = graph
            .edges
            .iter()
            .map(|e| GraphEdgeDto {
                from: e.from.clone(),
                to: e.to.clone(),
                label: e.label.clone(),
                kind: e.kind.clone(),
                weight: e.weight,
            })
            .collect();

        // Load personas
        let personas: Vec<PersonaInfo> = {
            let slugs = Persona::list_available_with_config(Some(&config_agents_dir), &personas_dir);
            let mut loaded: Vec<_> = slugs
                .iter()
                .filter_map(|s| {
                    Persona::load_with_config(s, Some(&config_agents_dir), &personas_dir).ok()
                })
                .collect();
            loaded.sort_by_key(|p| p.sort_order);
            loaded
                .iter()
                .map(|p| {
                    let preview: String = p.system_prompt.chars().take(500).collect();
                    PersonaInfo {
                        key: p.key.clone(),
                        label: p.label.clone(),
                        description: p.description.clone(),
                        emoji: p.emoji.clone(),
                        enabled: p.enabled,
                        tool_groups: p.tool_groups.clone(),
                        skill_tags: p.skill_tags.clone(),
                        system_prompt_preview: preview,
                    }
                })
                .collect()
        };

        // Load API keys
        let api_keys: Vec<ApiKeyInfo> = {
            let store = KeyStore::load(&app_config.keys_path()).unwrap_or_default();
            store
                .list_masked()
                .into_iter()
                .map(|(name, masked)| ApiKeyInfo {
                    name,
                    masked_key: masked,
                })
                .collect()
        };

        let available_models = vec![
            "gpt-5.4".to_string(),
            "gpt-5.4-mini".to_string(),
            "gpt-5.5".to_string(),
        ];

        Ok(Self {
            persona,
            api_key,
            model_name,
            auto_approve: config.auto_approve,
            runner: None,
            app_config,
            skills_dir,
            cwd,
            messages: vec![],
            tool_activity: vec![],
            agent_busy: false,
            status: "Ready".to_string(),
            skills,
            personas,
            api_keys,
            available_models,
            graph_nodes,
            graph_edges,
            pending_approvals: Arc::new(Mutex::new(HashMap::new())),
            approval_counter: Arc::new(Mutex::new(0)),
        })
    }

    fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(ChatMessageDto {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    fn add_tool_activity(&mut self, activity: &str) {
        self.tool_activity.push(activity.to_string());
        if self.tool_activity.len() > 20 {
            self.tool_activity.remove(0);
        }
    }

    fn build_runner(&mut self, approval_tx: &approval::ApprovalSender) -> anyhow::Result<()> {
        let approval_mode = if self.auto_approve {
            ApprovalMode::AutoApprove
        } else {
            ApprovalMode::tui_interactive(approval_tx.clone())
        };

        let keys_path = self.app_config.keys_path();
        let runner = AgentRunner::build_for_tui(
            &self.persona,
            &self.skills_dir,
            &self.cwd,
            &self.api_key,
            &self.model_name,
            approval_mode,
            Some(keys_path),
        ).map_err(|e| anyhow::anyhow!("{}", e))?;
        self.runner = Some(runner);
        Ok(())
    }
}

#[async_trait::async_trait]
impl crate::backend::Backend for StarkbotEngine {
    async fn start(&mut self) -> anyhow::Result<BackendHandle> {
        // Create approval channels
        let (approval_tx, approval_rx) = approval::approval_channel();

        // Build runner
        self.build_runner(&approval_tx)?;

        // Welcome message
        self.add_message(
            "assistant",
            &format!("Hello! I'm {}. How can I help?", self.persona.name()),
        );

        let snapshot = self.snapshot();

        // Create event channels (frontend reads events, sends commands)
        let (event_tx, event_rx) = mpsc::unbounded_channel::<BackendEvent>();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<FrontendCommand>();

        // Shared state
        let pending_approvals = self.pending_approvals.clone();
        let approval_counter = self.approval_counter.clone();

        // Clone data the command loop needs
        let mut agent_state: Option<AgentState> = None;
        let persona = self.persona.clone();
        let app_config = AppConfig::open(); // Re-open since AppConfig isn't Clone
        let skills_dir = self.skills_dir.clone();
        let cwd = self.cwd.clone();
        let api_key = self.api_key.clone();
        let mut model_name = self.model_name.clone();
        let auto_approve = self.auto_approve;
        let mut runner_graph = self.runner.as_ref().unwrap().graph.clone();
        let mut step_guard = self.runner.as_ref().unwrap().step_guard.clone();
        let mut messages = self.messages.clone();
        let mut tool_activity = self.tool_activity.clone();
        let mut agent_busy = false;
        #[allow(unused_assignments)]
        let mut api_keys_cache = self.api_keys.clone();

        let event_tx_clone = event_tx.clone();

        // Spawn the main engine loop
        tokio::spawn(async move {
            let event_tx = event_tx_clone;

            let emit = |evt: &BackendEvent| {
                let _ = event_tx.send(evt.clone());
            };

            let mut agent_event_rx: Option<mpsc::UnboundedReceiver<BackendEvent>> = None;

            loop {
                tokio::select! {
                    // Process commands from frontend
                    cmd = cmd_rx.recv() => {
                        let Some(cmd) = cmd else { break; };

                        match cmd {
                            FrontendCommand::SendMessage { content } => {
                                if agent_busy {
                                    continue;
                                }

                                if content.starts_with('/') {
                                    handle_slash_command(&content, &mut messages, &mut tool_activity, &mut agent_state, &event_tx);
                                    continue;
                                }

                                messages.push(ChatMessageDto { role: "user".to_string(), content: content.clone() });
                                emit(&BackendEvent::StatusUpdate { busy: true, message: "Agent thinking...".to_string() });
                                agent_busy = true;

                                let turn_state = match agent_state.take() {
                                    Some(prev) => prev.continue_with(&content),
                                    None => AgentState::new(&content),
                                };

                                let rg = runner_graph.clone();
                                let sg = step_guard.clone();
                                let (atx, arx) = mpsc::unbounded_channel::<BackendEvent>();
                                agent_event_rx = Some(arx);

                                tokio::spawn(async move {
                                    let executor = metalcraft::Executor::new_from_arc(rg)
                                        .max_steps(100)
                                        .with_step_guard(sg);

                                    match executor.run(turn_state, "agent").await {
                                        Ok(RunOutcome::Completed(state)) => {
                                            for msg in &state.messages {
                                                match msg {
                                                    AgentMessage::ToolCall { name, args, .. } => {
                                                        let args_str = serde_json::to_string(args).unwrap_or_default();
                                                        let _ = atx.send(BackendEvent::ToolCall { name: name.clone(), args: args_str });
                                                    }
                                                    AgentMessage::ToolResult { name, result, .. } => {
                                                        let success = !result.starts_with("ERROR:");
                                                        let _ = atx.send(BackendEvent::ToolResult { name: name.clone(), success, preview: result.clone() });
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            let answer = state.final_answer().unwrap_or("(no answer)").to_string();
                                            let _ = atx.send(BackendEvent::TurnComplete { answer });
                                        }
                                        Ok(RunOutcome::Interrupted { reason, .. }) => {
                                            let _ = atx.send(BackendEvent::Error { message: format!("Interrupted: {}", reason) });
                                        }
                                        Err(e) => {
                                            let _ = atx.send(BackendEvent::Error { message: format!("Error: {}", e) });
                                        }
                                    }
                                });
                            }

                            FrontendCommand::ApprovalResponse { request_id, approved } => {
                                let resp_tx = {
                                    let mut map = pending_approvals.lock().unwrap();
                                    map.remove(&request_id)
                                };
                                if let Some(resp_tx) = resp_tx {
                                    let _ = resp_tx.send(approved);
                                    let tool_name = request_id.split(':').nth(1).unwrap_or("unknown");
                                    let prefix = if approved { "✓ Approved" } else { "✗ Denied" };
                                    let msg = format!("{}: {}", prefix, tool_name);
                                    messages.push(ChatMessageDto { role: "tool".to_string(), content: msg.clone() });
                                    emit(&BackendEvent::Info { message: msg });
                                }
                            }

                            FrontendCommand::SwitchModel { model } => {
                                let am = if auto_approve {
                                    ApprovalMode::AutoApprove
                                } else {
                                    ApprovalMode::tui_interactive(approval_tx.clone())
                                };
                                let keys_path = app_config.keys_path();
                                match AgentRunner::build_for_tui(&persona, &skills_dir, &cwd, &api_key, &model, am, Some(keys_path)) {
                                    Ok(new_runner) => {
                                        runner_graph = new_runner.graph.clone();
                                        step_guard = new_runner.step_guard.clone();
                                        model_name = model.clone();

                                        if let Ok(mut s) = starkbot_config::settings::Settings::load(&app_config.settings_path()) {
                                            s.model = model.clone();
                                            let _ = s.save(&app_config.settings_path());
                                        }

                                        let msg = format!("Model switched to {}.", model);
                                        messages.push(ChatMessageDto { role: "assistant".to_string(), content: msg.clone() });
                                        emit(&BackendEvent::ModelChanged { model });
                                        emit(&BackendEvent::Info { message: msg });
                                    }
                                    Err(e) => {
                                        emit(&BackendEvent::Error { message: format!("Failed to switch model: {}", e) });
                                    }
                                }
                            }

                            FrontendCommand::ApiKeyAdd { name, key } => {
                                let keys_path = app_config.keys_path();
                                if let Ok(mut store) = KeyStore::load(&keys_path) {
                                    store.upsert(&name, &key);
                                    if store.save(&keys_path).is_ok() {
                                        api_keys_cache = store.list_masked().into_iter()
                                            .map(|(n, m)| ApiKeyInfo { name: n, masked_key: m })
                                            .collect();
                                        let msg = format!("API key {} installed.", name);
                                        messages.push(ChatMessageDto { role: "assistant".to_string(), content: msg.clone() });
                                        emit(&BackendEvent::Info { message: msg });
                                    }
                                }
                            }

                            FrontendCommand::ApiKeyDelete { name } => {
                                let keys_path = app_config.keys_path();
                                if let Ok(mut store) = KeyStore::load(&keys_path) {
                                    if store.delete(&name) && store.save(&keys_path).is_ok() {
                                        api_keys_cache = store.list_masked().into_iter()
                                            .map(|(n, m)| ApiKeyInfo { name: n, masked_key: m })
                                            .collect();
                                        let msg = format!("API key {} deleted.", name);
                                        messages.push(ChatMessageDto { role: "assistant".to_string(), content: msg.clone() });
                                        emit(&BackendEvent::Info { message: msg });
                                    }
                                }
                            }

                            FrontendCommand::SlashCommand { command } => {
                                handle_slash_command(&command, &mut messages, &mut tool_activity, &mut agent_state, &event_tx);
                            }

                            FrontendCommand::RequestSnapshot => {
                                // Build snapshot from current state
                                // Note: some fields (skills, personas, graph) are static after init
                                // We send a snapshot event
                                emit(&BackendEvent::Info { message: "Snapshot requested".to_string() });
                            }

                            FrontendCommand::Shutdown => {
                                break;
                            }

                            _ => {}
                        }
                    }

                    // Forward agent events
                    evt = async {
                        match &mut agent_event_rx {
                            Some(rx) => rx.recv().await,
                            None => std::future::pending().await,
                        }
                    } => {
                        if let Some(evt) = evt {
                            // Update internal state
                            match &evt {
                                BackendEvent::ToolCall { name, args } => {
                                    let activity = format!("▶ {} {}", name, args);
                                    tool_activity.push(activity);
                                    if tool_activity.len() > 20 { tool_activity.remove(0); }
                                    messages.push(ChatMessageDto {
                                        role: "tool".to_string(),
                                        content: format!("▶ {}({})", name, truncate_str(args, 80)),
                                    });
                                }
                                BackendEvent::ToolResult { name, success, preview } => {
                                    let icon = if *success { "✓" } else { "✗" };
                                    let activity = format!("{} {}", icon, name);
                                    tool_activity.push(activity);
                                    if tool_activity.len() > 20 { tool_activity.remove(0); }
                                    if !success {
                                        messages.push(ChatMessageDto {
                                            role: "error".to_string(),
                                            content: format!("{} {} failed: {}", icon, name, truncate_str(preview, 100)),
                                        });
                                    }
                                }
                                BackendEvent::TurnComplete { answer } => {
                                    agent_busy = false;
                                    messages.push(ChatMessageDto {
                                        role: "assistant".to_string(),
                                        content: answer.clone(),
                                    });
                                    emit(&BackendEvent::StatusUpdate { busy: false, message: "Ready".to_string() });
                                }
                                BackendEvent::Error { .. } => {
                                    agent_busy = false;
                                    emit(&BackendEvent::StatusUpdate { busy: false, message: "Ready".to_string() });
                                }
                                _ => {}
                            }
                            // Forward to frontend
                            let _ = event_tx.send(evt);
                        }
                    }

                    // Poll approval channel (sync -> async bridge)
                    _ = tokio::time::sleep(std::time::Duration::from_millis(16)) => {
                        while let Ok((request, resp_tx)) = approval_rx.try_recv() {
                            let request_id = {
                                let mut counter = approval_counter.lock().unwrap();
                                *counter += 1;
                                format!("{}:{}", *counter, request.tool_name)
                            };
                            {
                                let mut map = pending_approvals.lock().unwrap();
                                map.insert(request_id.clone(), resp_tx);
                            }
                            let _ = event_tx.send(BackendEvent::ApprovalRequired {
                                request_id,
                                tool_name: request.tool_name,
                                args_display: request.args_display,
                            });
                        }
                    }
                }
            }
        });

        Ok(BackendHandle {
            events: event_rx,
            commands: cmd_tx,
            initial_snapshot: snapshot,
        })
    }

    fn snapshot(&self) -> AppSnapshot {
        AppSnapshot {
            persona_name: self.persona.name().to_string(),
            model_name: self.model_name.clone(),
            agent_busy: self.agent_busy,
            status: self.status.clone(),
            messages: self.messages.clone(),
            tool_activity: self.tool_activity.clone(),
            skills: self.skills.clone(),
            personas: self.personas.clone(),
            api_keys: self.api_keys.clone(),
            available_models: self.available_models.clone(),
            graph_nodes: self.graph_nodes.clone(),
            graph_edges: self.graph_edges.clone(),
        }
    }

    async fn shutdown(&mut self) {
        self.runner = None;
    }
}

// --- Helpers ---

fn handle_slash_command(
    input: &str,
    messages: &mut Vec<ChatMessageDto>,
    tool_activity: &mut Vec<String>,
    agent_state: &mut Option<AgentState>,
    event_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let add_msg = |messages: &mut Vec<ChatMessageDto>, role: &str, content: &str| {
        messages.push(ChatMessageDto {
            role: role.to_string(),
            content: content.to_string(),
        });
    };

    match input.trim() {
        "/quit" | "/exit" => {
            // Frontend handles quit
        }
        "/clear" => {
            messages.clear();
            tool_activity.clear();
            *agent_state = None;
            add_msg(messages, "assistant", "Conversation cleared.");
            let _ = event_tx.send(BackendEvent::Info {
                message: "Conversation cleared.".to_string(),
            });
        }
        "/tokens" => {
            let msg = if let Some(s) = agent_state {
                let tokens = context::estimate_tokens(s);
                format!("~{} tokens, {} messages", tokens, s.messages.len())
            } else {
                "No conversation yet.".to_string()
            };
            add_msg(messages, "assistant", &msg);
            let _ = event_tx.send(BackendEvent::Info { message: msg });
        }
        cmd if cmd.starts_with("/help") => {
            let msg =
                "Commands: /quit, /clear, /tokens, /help\nTab: switch views | Ctrl+C: quit"
                    .to_string();
            add_msg(messages, "assistant", &msg);
            let _ = event_tx.send(BackendEvent::Info { message: msg });
        }
        _ => {
            let msg = format!("Unknown command: {}", input);
            add_msg(messages, "error", &msg);
            let _ = event_tx.send(BackendEvent::Error { message: msg });
        }
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

fn find_bundled_dir(name: &str) -> Option<PathBuf> {
    let cwd_based = PathBuf::from(name);
    if cwd_based.is_dir() {
        return Some(cwd_based);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let exe_based = parent.join(name);
            if exe_based.is_dir() {
                return Some(exe_based);
            }
        }
    }
    None
}

fn migrate_keys_from_db(app_config: &AppConfig) {
    let old_db_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("starkbot")
        .join("starkbot.db");

    if !old_db_path.exists() {
        return;
    }

    let store = KeyStore::load(&app_config.keys_path()).unwrap_or_default();
    if !store.is_empty() {
        return;
    }

    if let Ok(db) = starkbot_db::Database::open(&old_db_path) {
        if let Ok(keys) = db.list_api_keys_raw() {
            if keys.is_empty() {
                return;
            }
            let mut new_store = KeyStore::default();
            for (name, key) in &keys {
                new_store.upsert(name, key);
            }
            if new_store.save(&app_config.keys_path()).is_ok() {
                log::info!(
                    "Migrated {} API keys from starkbot.db to keys.json",
                    keys.len()
                );
            }
        }
    }
}

fn populate_skill_fts(skills_dir: &std::path::Path) {
    let old_db_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("starkbot")
        .join("starkbot.db");

    if !old_db_path.exists() {
        return;
    }

    if let Ok(db) = starkbot_db::Database::open(&old_db_path) {
        let skill_registry = SkillRegistry::load_from_dir(skills_dir);
        for skill in skill_registry.all() {
            db.upsert_skill_fts(
                &skill.name,
                &skill.description,
                &skill.tags.join(" "),
                &skill.content,
            )
            .ok();
        }
    }
}

/// Create a backend engine from config.
pub fn create_backend(config: BackendConfig) -> anyhow::Result<Box<dyn crate::backend::Backend>> {
    Ok(Box::new(StarkbotEngine::new(config)?))
}
