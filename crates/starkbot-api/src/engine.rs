use crate::backend::{BackendConfig, BackendHandle};
use crate::event_bus::EventBus;
use crate::events::{BackendEvent, FrontendCommand};
use crate::types::*;

use metalcraft::{AgentMessage, AgentState, GuardAction, RunOutcome, StepEvent, StepGuard};
use starkbot_core::context;
use starkbot_core::dispatch::AgentRunner;
use starkbot_core::persona::Persona;
use starkbot_graph::build_skill_graph;
use starkbot_skills::SkillRegistry;
use starkbot_tools::approval::{self, ApprovalMode};
use starkbot_config::keys::KeyStore;
use starkbot_config::sessions;
use starkbot_config::schedules;
use starkbot_config::integrations;
use starkbot_config::AppConfig;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::mpsc;
use tokio::time::Instant;

fn debug_log(msg: impl Into<String>) -> BackendEvent {
    let now = chrono::Local::now();
    BackendEvent::DebugLog {
        timestamp: now.format("%H:%M:%S%.3f").to_string(),
        level: "INFO".to_string(),
        message: msg.into(),
    }
}

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

    // Sessions
    sessions: Vec<SessionSummary>,
    current_session_id: Option<String>,

    // Integrations
    integrations: Vec<IntegrationPresetInfo>,

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
        let bundled_presets = find_bundled_dir("integration_presets");
        app_config.ensure_initialized(bundled_agents.as_deref(), bundled_skills.as_deref(), bundled_presets.as_deref())
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

        // Resolve API key (no ENV fallback — must be configured via UI or keys.json)
        let api_key = if !config.api_key.is_empty() {
            config.api_key.clone()
        } else {
            let store = KeyStore::load(&app_config.keys_path()).unwrap_or_default();
            store
                .get("OPENAI_API_KEY")
                .map(|s| s.to_string())
                .unwrap_or_default()
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
                    requires_keys: s.requires_keys.clone(),
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

        // Load sessions
        let session_list = sessions::list_sessions(&app_config.sessions_dir());

        // Load integrations
        let integration_list = build_integrations_list(&app_config);

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
            sessions: session_list,
            current_session_id: None,
            integrations: integration_list,
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

        // Build runner (skip if no API key configured)
        if !self.api_key.is_empty() {
            self.build_runner(&approval_tx)?;
        }

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
        let flow_running_count = Arc::new(AtomicUsize::new(0));

        // Clone data the command loop needs
        let mut agent_state: Option<AgentState> = None;
        let persona = self.persona.clone();
        let app_config = AppConfig::open(); // Re-open since AppConfig isn't Clone
        let skills_dir = self.skills_dir.clone();
        let cwd = self.cwd.clone();
        let mut api_key = self.api_key.clone();
        let mut model_name = self.model_name.clone();
        let auto_approve = self.auto_approve;
        let mut runner_graph = self.runner.as_ref().map(|r| r.graph.clone());
        let mut step_guard = self.runner.as_ref().map(|r| r.step_guard.clone());
        let mut messages = self.messages.clone();
        let mut tool_activity = self.tool_activity.clone();
        let mut agent_busy = false;
        #[allow(unused_assignments)]
        let mut api_keys_cache = self.api_keys.clone();
        #[allow(unused_assignments)]
        let mut integrations_cache = self.integrations.clone();
        let mut current_session_id: Option<String> = None;
        let mut session_created_at: Option<String> = None;
        let persona_name_for_session = persona.name().to_string();

        let event_tx_clone = event_tx.clone();

        // --- Event Bus ---
        let event_bus = Arc::new(EventBus::new());
        let shutdown_flag = Arc::new(AtomicBool::new(false));

        // Shared mutable config so workers see API key / model changes
        let shared_api_key = Arc::new(Mutex::new(self.api_key.clone()));
        let shared_model_name = Arc::new(Mutex::new(self.model_name.clone()));

        // Pulse emitter: fires every 5 seconds
        {
            let bus = event_bus.clone();
            let shutdown = shutdown_flag.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    interval.tick().await;
                    if shutdown.load(Ordering::Relaxed) { break; }
                    bus.emit("pulse", "");
                }
            });
        }

        // Scheduler worker: listens for pulse, triggers due flows
        {
            let bus = event_bus.clone();
            let sched_app_config = AppConfig::open();
            let sched_event_tx = event_tx.clone();
            let shutdown = shutdown_flag.clone();
            tokio::spawn(async move {
                let mut rx = bus.subscribe();
                // Seed last-run to now so flows don't all fire immediately on startup
                let mut flow_last_run: HashMap<String, Instant> = HashMap::new();
                let boot = Instant::now();
                loop {
                    if shutdown.load(Ordering::Relaxed) { break; }
                    match rx.recv().await {
                        Ok(evt) if evt.kind == "pulse" => {
                            let flows = schedules::list_flows(&sched_app_config.flows_dir());
                            let now = Instant::now();
                            for summary in &flows {
                                if !summary.enabled {
                                    continue;
                                }
                                let Some(flow) = schedules::load_flow(&sched_app_config.flows_dir(), &summary.id) else { continue };
                                let entry_node = flow.flow.nodes.iter().find(|n| matches!(n.node_type, schedules::FlowNodeType::Entry));
                                let Some(entry) = entry_node else { continue };

                                let schedule_type = entry.data.get("schedule_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("minutes");
                                let interval_val = entry.data.get("interval")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(5);
                                let interval_secs = match schedule_type {
                                    "hours" => interval_val * 3600,
                                    _ => interval_val * 60,
                                };

                                // First time seeing this flow: seed to boot time so it waits one full interval
                                let last = flow_last_run.entry(summary.id.clone()).or_insert(boot);
                                let should_run = now.duration_since(*last).as_secs() >= interval_secs;

                                if should_run {
                                    *last = now;
                                    log::info!("[scheduler] Triggering flow '{}' (every {} {})", summary.name, interval_val, schedule_type);
                                    let _ = sched_event_tx.send(debug_log(format!("Scheduler: triggering flow '{}'", summary.name)));
                                    bus.emit("run_flow", &summary.id);
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            log::warn!("[scheduler] Lagged {} events", n);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });
        }

        // Flow executor worker: listens for run_flow events
        {
            let bus = event_bus.clone();
            let exec_persona = self.persona.clone();
            let exec_skills_dir = self.skills_dir.clone();
            let exec_cwd = self.cwd.clone();
            let exec_shared_api_key = shared_api_key.clone();
            let exec_shared_model_name = shared_model_name.clone();
            let exec_app_config = AppConfig::open();
            let exec_event_tx = event_tx.clone();
            let exec_persona_name = persona.name().to_string();
            let exec_flow_running = flow_running_count.clone();
            let shutdown = shutdown_flag.clone();
            let in_flight: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
            tokio::spawn(async move {
                let mut rx = bus.subscribe();
                loop {
                    if shutdown.load(Ordering::Relaxed) { break; }
                    match rx.recv().await {
                        Ok(evt) if evt.kind == "run_flow" => {
                            let flow_id = evt.payload.clone();

                            // Skip if this flow is already running
                            {
                                let set = in_flight.lock().unwrap();
                                if set.contains(&flow_id) {
                                    log::info!("[flow-executor] Flow '{}' already running, skipping", flow_id);
                                    continue;
                                }
                            }

                            let Some(flow) = schedules::load_flow(&exec_app_config.flows_dir(), &flow_id) else {
                                let _ = exec_event_tx.send(BackendEvent::Error { message: format!("Flow '{}' not found", flow_id) });
                                continue;
                            };

                            let run_id = uuid::Uuid::new_v4().to_string();
                            schedules::append_flow_log(&exec_app_config.flow_logs_path(), &FlowLogEntry {
                                timestamp: chrono::Local::now().to_rfc3339(),
                                flow_id: flow.id.clone(),
                                flow_name: flow.name.clone(),
                                action: "run_once".to_string(),
                                detail: String::new(),
                                run_id: Some(run_id.clone()),
                            });

                            let prompts = schedules::walk_flow_prompts(&flow.flow);
                            if prompts.is_empty() {
                                let _ = exec_event_tx.send(BackendEvent::Error { message: format!("Flow '{}' has no prompt nodes", flow.name) });
                                bus.emit("flow_error", &format!("{}|no prompt nodes", flow.id));
                                continue;
                            }

                            // Read current api_key and model from shared state
                            let current_api_key = exec_shared_api_key.lock().unwrap().clone();
                            let current_model_name = exec_shared_model_name.lock().unwrap().clone();

                            if current_api_key.is_empty() {
                                let _ = exec_event_tx.send(BackendEvent::Error { message: "Flow execution requires an API key.".to_string() });
                                bus.emit("flow_error", &format!("{}|no API key", flow.id));
                                continue;
                            }

                            log::info!("[flow-executor] Starting flow '{}' with {} prompt(s)", flow.name, prompts.len());

                            // Mark as in-flight
                            in_flight.lock().unwrap().insert(flow_id.clone());

                            let flow_event_tx = exec_event_tx.clone();
                            let flow_persona = exec_persona.clone();
                            let flow_skills_dir = exec_skills_dir.clone();
                            let flow_cwd = exec_cwd.clone();
                            let flow_api_key = current_api_key;
                            let flow_model_name = current_model_name;
                            let flow_app_config = AppConfig::open();
                            let flow_name = flow.name.clone();
                            let flow_id_clone = flow.id.clone();
                            let flow_persona_name = exec_persona_name.clone();
                            let keys_path = exec_app_config.keys_path();
                            let flow_run_id = run_id.clone();
                            let flow_running = exec_flow_running.clone();
                            let bus_clone = bus.clone();
                            let in_flight_clone = in_flight.clone();

                            let prev = flow_running.fetch_add(1, Ordering::SeqCst);
                            let _ = exec_event_tx.send(BackendEvent::FlowRunningCount { count: prev + 1 });

                            tokio::spawn(async move {
                                let runner = match AgentRunner::build_for_tui(
                                    &flow_persona,
                                    &flow_skills_dir,
                                    &flow_cwd,
                                    &flow_api_key,
                                    &flow_model_name,
                                    ApprovalMode::AutoApprove,
                                    Some(keys_path),
                                ) {
                                    Ok(r) => r,
                                    Err(e) => {
                                        log::error!("[flow-executor] Failed to build runner: {}", e);
                                        bus_clone.emit("flow_error", &format!("{}|{}", flow_id_clone, e));
                                        let prev = flow_running.fetch_sub(1, Ordering::SeqCst);
                                        let _ = flow_event_tx.send(BackendEvent::FlowRunningCount { count: prev - 1 });
                                        return;
                                    }
                                };

                                let mut session_messages: Vec<ChatSessionMessage> = Vec::new();
                                let mut all_succeeded = true;
                                let mut last_error: Option<String> = None;

                                for prompt_text in &prompts {
                                    session_messages.push(ChatSessionMessage {
                                        role: "user".to_string(),
                                        content: prompt_text.clone(),
                                    });

                                    let autonomous_prompt = format!(
                                        "[AUTONOMOUS MODE] You are executing a flow step. \
                                         You cannot ask the user follow-up questions — you must \
                                         do your best with the information given. Use your tools \
                                         to research and gather any missing information. \
                                         Produce a complete answer.\n\n{}", prompt_text
                                    );
                                    let turn_state = AgentState::new(&autonomous_prompt);

                                    let (atx, mut arx) = mpsc::unbounded_channel::<BackendEvent>();
                                    let emitting_sg = build_emitting_guard(runner.step_guard.clone(), atx.clone());
                                    let rg = runner.graph.clone();

                                    let (result_tx, result_rx) = tokio::sync::oneshot::channel::<Result<AgentState, String>>();

                                    tokio::spawn(async move {
                                        let executor = metalcraft::Executor::new_from_arc(rg)
                                            .max_steps(100)
                                            .with_step_guard(emitting_sg);

                                        let result = tokio::time::timeout(
                                            std::time::Duration::from_secs(120),
                                            executor.run(turn_state, "agent"),
                                        ).await;

                                        match result {
                                            Ok(Ok(RunOutcome::Completed(state))) => {
                                                let answer = state.final_answer().unwrap_or("(no answer)").to_string();
                                                let _ = atx.send(BackendEvent::TurnComplete { answer });
                                                let _ = result_tx.send(Ok(state));
                                            }
                                            Ok(Ok(RunOutcome::Interrupted { reason, .. })) => {
                                                let _ = atx.send(BackendEvent::Error { message: format!("Interrupted: {}", reason) });
                                                let _ = result_tx.send(Err(format!("Interrupted: {}", reason)));
                                            }
                                            Ok(Err(e)) => {
                                                let _ = atx.send(BackendEvent::Error { message: format!("Error: {}", e) });
                                                let _ = result_tx.send(Err(format!("{}", e)));
                                            }
                                            Err(_) => {
                                                let _ = atx.send(BackendEvent::Error { message: "Flow prompt timed out after 120s".to_string() });
                                                let _ = result_tx.send(Err("Timeout".to_string()));
                                            }
                                        }
                                    });

                                    let fwd_handle = tokio::spawn(async move {
                                        while arx.recv().await.is_some() {}
                                    });

                                    match result_rx.await {
                                        Ok(Ok(state)) => {
                                            let answer = state.final_answer().unwrap_or("(no answer)").to_string();
                                            session_messages.push(ChatSessionMessage {
                                                role: "assistant".to_string(),
                                                content: answer,
                                            });
                                        }
                                        Ok(Err(err)) => {
                                            session_messages.push(ChatSessionMessage {
                                                role: "error".to_string(),
                                                content: err.clone(),
                                            });
                                            last_error = Some(err);
                                            all_succeeded = false;
                                            break;
                                        }
                                        Err(_) => {
                                            let msg = "Internal error: result channel closed".to_string();
                                            session_messages.push(ChatSessionMessage {
                                                role: "error".to_string(),
                                                content: msg.clone(),
                                            });
                                            last_error = Some(msg);
                                            all_succeeded = false;
                                            break;
                                        }
                                    }

                                    let _ = fwd_handle.await;
                                }

                                // Save as ChatSession
                                let session_id = uuid::Uuid::new_v4().to_string();
                                let session = ChatSession {
                                    id: session_id,
                                    persona: flow_persona_name,
                                    title: format!("Flow: {}", flow_name),
                                    created_at: chrono::Local::now().to_rfc3339(),
                                    messages: session_messages,
                                };
                                match sessions::save_session(&flow_app_config.sessions_dir(), &session) {
                                    Ok(_) => log::info!("[flow-executor] Saved session '{}'", session.title),
                                    Err(e) => log::error!("[flow-executor] Failed to save session: {}", e),
                                }

                                let updated_sessions = sessions::list_sessions(&flow_app_config.sessions_dir());
                                let _ = flow_event_tx.send(BackendEvent::SessionsUpdated(updated_sessions));

                                let detail = if all_succeeded {
                                    "completed successfully".to_string()
                                } else if let Some(ref err) = last_error {
                                    format!("error: {}", err)
                                } else {
                                    "completed with errors".to_string()
                                };
                                schedules::append_flow_log(&flow_app_config.flow_logs_path(), &FlowLogEntry {
                                    timestamp: chrono::Local::now().to_rfc3339(),
                                    flow_id: flow_id_clone.clone(),
                                    flow_name: flow_name.clone(),
                                    action: "executed".to_string(),
                                    detail: detail.clone(),
                                    run_id: Some(flow_run_id),
                                });

                                let prev = flow_running.fetch_sub(1, Ordering::SeqCst);
                                let _ = flow_event_tx.send(BackendEvent::FlowRunningCount { count: prev - 1 });

                                let logs = schedules::load_flow_logs(&flow_app_config.flow_logs_path());
                                let _ = flow_event_tx.send(BackendEvent::FlowLogsLoaded(logs));

                                if all_succeeded {
                                    bus_clone.emit("flow_completed", &flow_id_clone);
                                } else {
                                    bus_clone.emit("flow_error", &format!("{}|{}", flow_id_clone, detail));
                                }

                                // Remove from in-flight set
                                in_flight_clone.lock().unwrap().remove(&flow_id_clone);

                                log::info!("[flow-executor] Flow '{}' {}", flow_name, if all_succeeded { "completed" } else { "failed" });
                            });
                        }
                        Ok(_) => {} // ignore non-run_flow events
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            log::warn!("[flow-executor] Lagged {} events", n);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });
        }

        let event_bus_for_loop = event_bus.clone();
        let shutdown_for_loop = shutdown_flag.clone();
        let shared_api_key_for_loop = shared_api_key.clone();
        let shared_model_for_loop = shared_model_name.clone();

        // Spawn the main engine loop
        tokio::spawn(async move {
            let event_tx = event_tx_clone;
            let event_bus = event_bus_for_loop;
            let shutdown_flag = shutdown_for_loop;
            let shared_api_key = shared_api_key_for_loop;
            let shared_model = shared_model_for_loop;

            let emit = |evt: &BackendEvent| {
                let _ = event_tx.send(evt.clone());
            };

            let mut agent_event_rx: Option<mpsc::UnboundedReceiver<BackendEvent>> = None;
            let mut state_rx: Option<tokio::sync::oneshot::Receiver<AgentState>> = None;

            loop {
                tokio::select! {
                    // Process commands from frontend
                    cmd = cmd_rx.recv() => {
                        let Some(cmd) = cmd else { break; };

                        match cmd {
                            FrontendCommand::SendMessage { content } => {
                                log::info!("[engine] SendMessage received: {:?}", truncate_str(&content, 50));
                                let _ = event_tx.send(debug_log(format!("SendMessage: {}", truncate_str(&content, 50))));

                                if agent_busy {
                                    log::info!("[engine] Agent busy, ignoring message");
                                    let _ = event_tx.send(debug_log("Agent busy, message ignored"));
                                    continue;
                                }

                                if content.starts_with('/') {
                                    log::info!("[engine] Slash command: {}", &content);
                                    let _ = event_tx.send(debug_log(format!("Slash command: {}", &content)));
                                    handle_slash_command(&content, &mut messages, &mut tool_activity, &mut agent_state, &event_tx);
                                    continue;
                                }

                                // Check if inference is configured
                                if runner_graph.is_none() || api_key.is_empty() {
                                    messages.push(ChatMessageDto { role: "user".to_string(), content: content.clone() });
                                    let err_msg = "Inference not configured. Add your OpenAI API key in Settings > Inference.".to_string();
                                    messages.push(ChatMessageDto { role: "error".to_string(), content: err_msg.clone() });
                                    emit(&BackendEvent::Error { message: err_msg });
                                    continue;
                                }

                                messages.push(ChatMessageDto { role: "user".to_string(), content: content.clone() });

                                // Start a session if none exists
                                if current_session_id.is_none() {
                                    let id = uuid::Uuid::new_v4().to_string();
                                    current_session_id = Some(id);
                                    session_created_at = Some(chrono::Local::now().to_rfc3339());
                                }

                                emit(&BackendEvent::StatusUpdate { busy: true, message: "Agent thinking...".to_string() });
                                agent_busy = true;

                                let turn_state = match agent_state.take() {
                                    Some(prev) => {
                                        log::info!("[engine] Continuing conversation ({} prior messages)", prev.messages.len());
                                        let _ = event_tx.send(debug_log(format!("Continuing conversation ({} prior msgs)", prev.messages.len())));
                                        prev.continue_with(&content)
                                    }
                                    None => {
                                        log::info!("[engine] New conversation");
                                        let _ = event_tx.send(debug_log("Starting new conversation"));
                                        AgentState::new(&content)
                                    }
                                };

                                let rg = runner_graph.clone().unwrap();
                                let sg = step_guard.clone().unwrap();
                                let (atx, arx) = mpsc::unbounded_channel::<BackendEvent>();
                                agent_event_rx = Some(arx);

                                // Build an emitting guard that sends real-time events
                                let emitting_sg = build_emitting_guard(sg, atx.clone());

                                let (stx, srx) = tokio::sync::oneshot::channel::<AgentState>();
                                state_rx = Some(srx);

                                log::info!("[engine] Spawning executor task");
                                let _ = event_tx.send(debug_log("Spawning executor task"));

                                let dbg_tx = event_tx.clone();
                                tokio::spawn(async move {
                                    log::info!("[executor] Starting executor.run()");
                                    let _ = dbg_tx.send(debug_log("Executor: calling run()"));

                                    let executor = metalcraft::Executor::new_from_arc(rg)
                                        .max_steps(100)
                                        .with_step_guard(emitting_sg);

                                    let result = tokio::time::timeout(
                                        std::time::Duration::from_secs(120),
                                        executor.run(turn_state, "agent"),
                                    ).await;

                                    match result {
                                        Ok(Ok(RunOutcome::Completed(state))) => {
                                            log::info!("[executor] Completed with {} messages", state.messages.len());
                                            let _ = dbg_tx.send(debug_log(format!("Executor completed: {} messages", state.messages.len())));
                                            let answer = state.final_answer().unwrap_or("(no answer)").to_string();
                                            let _ = atx.send(BackendEvent::TurnComplete { answer });
                                            let _ = stx.send(state);
                                        }
                                        Ok(Ok(RunOutcome::Interrupted { reason, .. })) => {
                                            log::warn!("[executor] Interrupted: {}", reason);
                                            let _ = dbg_tx.send(debug_log(format!("Executor interrupted: {}", reason)));
                                            let _ = atx.send(BackendEvent::Error { message: format!("Interrupted: {}", reason) });
                                        }
                                        Ok(Err(e)) => {
                                            log::error!("[executor] Error: {}", e);
                                            let _ = dbg_tx.send(debug_log(format!("Executor error: {}", e)));
                                            let _ = atx.send(BackendEvent::Error { message: format!("Error: {}", e) });
                                        }
                                        Err(_) => {
                                            log::error!("[executor] Timed out after 120s");
                                            let _ = dbg_tx.send(debug_log("Executor timed out after 120s"));
                                            let _ = atx.send(BackendEvent::Error { message: "Agent timed out after 120 seconds".to_string() });
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
                                if api_key.is_empty() {
                                    emit(&BackendEvent::Error { message: "Cannot switch model: no API key configured.".to_string() });
                                    continue;
                                }
                                let am = if auto_approve {
                                    ApprovalMode::AutoApprove
                                } else {
                                    ApprovalMode::tui_interactive(approval_tx.clone())
                                };
                                let keys_path = app_config.keys_path();
                                match AgentRunner::build_for_tui(&persona, &skills_dir, &cwd, &api_key, &model, am, Some(keys_path)) {
                                    Ok(new_runner) => {
                                        runner_graph = Some(new_runner.graph.clone());
                                        step_guard = Some(new_runner.step_guard.clone());
                                        model_name = model.clone();
                                        *shared_model.lock().unwrap() = model_name.clone();

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

                                        // If this is the inference key, rebuild the runner
                                        if name == "OPENAI_API_KEY" {
                                            api_key = key.clone();
                                            *shared_api_key.lock().unwrap() = api_key.clone();
                                            let am = if auto_approve {
                                                ApprovalMode::AutoApprove
                                            } else {
                                                ApprovalMode::tui_interactive(approval_tx.clone())
                                            };
                                            match AgentRunner::build_for_tui(&persona, &skills_dir, &cwd, &api_key, &model_name, am, Some(keys_path.clone())) {
                                                Ok(new_runner) => {
                                                    runner_graph = Some(new_runner.graph.clone());
                                                    step_guard = Some(new_runner.step_guard.clone());
                                                    let msg = "Inference configured. You can now start chatting.".to_string();
                                                    messages.push(ChatMessageDto { role: "assistant".to_string(), content: msg.clone() });
                                                    emit(&BackendEvent::Info { message: msg });
                                                }
                                                Err(e) => {
                                                    emit(&BackendEvent::Error { message: format!("Failed to configure inference: {}", e) });
                                                }
                                            }
                                        } else {
                                            let msg = format!("API key {} installed.", name);
                                            messages.push(ChatMessageDto { role: "assistant".to_string(), content: msg.clone() });
                                            emit(&BackendEvent::Info { message: msg });
                                        }
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

                                        if name == "OPENAI_API_KEY" {
                                            api_key = String::new();
                                            *shared_api_key.lock().unwrap() = String::new();
                                            runner_graph = None;
                                            step_guard = None;
                                            let msg = "Inference disabled. Configure an API key in Settings > Inference to continue.".to_string();
                                            messages.push(ChatMessageDto { role: "assistant".to_string(), content: msg.clone() });
                                            emit(&BackendEvent::Info { message: msg });
                                        } else {
                                            let msg = format!("API key {} deleted.", name);
                                            messages.push(ChatMessageDto { role: "assistant".to_string(), content: msg.clone() });
                                            emit(&BackendEvent::Info { message: msg });
                                        }
                                    }
                                }
                            }

                            FrontendCommand::SlashCommand { command } => {
                                if command.trim() == "/clear" {
                                    current_session_id = None;
                                    session_created_at = None;
                                }
                                handle_slash_command(&command, &mut messages, &mut tool_activity, &mut agent_state, &event_tx);
                            }

                            FrontendCommand::RequestSnapshot => {
                                // Rebuild integrations list (picks up newly seeded presets)
                                integrations_cache = build_integrations_list(&app_config);
                                let _ = event_tx.send(BackendEvent::IntegrationsUpdated(integrations_cache.clone()));
                            }

                            FrontendCommand::LoadSession { session_id } => {
                                if let Some(session) = sessions::load_session(&app_config.sessions_dir(), &session_id) {
                                    let _ = event_tx.send(BackendEvent::SessionLoaded(Box::new(session)));
                                } else {
                                    let _ = event_tx.send(BackendEvent::Error { message: format!("Failed to load session {}", session_id) });
                                }
                            }

                            FrontendCommand::DeleteSession { session_id } => {
                                // Clear current session ID if it matches the deleted one
                                if current_session_id.as_deref() == Some(&session_id) {
                                    current_session_id = None;
                                    session_created_at = None;
                                }
                                sessions::delete_session(&app_config.sessions_dir(), &session_id);
                                let updated = sessions::list_sessions(&app_config.sessions_dir());
                                let _ = event_tx.send(BackendEvent::SessionsUpdated(updated));
                            }

                            FrontendCommand::FlowSave { flow } => {
                                let _ = schedules::save_flow(&app_config.flows_dir(), &flow);
                                schedules::append_flow_log(&app_config.flow_logs_path(), &FlowLogEntry {
                                    timestamp: chrono::Local::now().to_rfc3339(),
                                    flow_id: flow.id.clone(),
                                    flow_name: flow.name.clone(),
                                    action: "saved".to_string(),
                                    detail: format!("{} nodes", flow.flow.nodes.len()),
                                    run_id: None,
                                });
                                let updated = schedules::list_flows(&app_config.flows_dir());
                                let _ = event_tx.send(BackendEvent::FlowsListed(updated));
                            }

                            FrontendCommand::FlowLoad { flow_id } => {
                                if let Some(flow) = schedules::load_flow(&app_config.flows_dir(), &flow_id) {
                                    let _ = event_tx.send(BackendEvent::FlowLoaded(Box::new(flow)));
                                } else {
                                    let _ = event_tx.send(BackendEvent::Error {
                                        message: format!("Flow '{}' not found", flow_id),
                                    });
                                }
                            }

                            FrontendCommand::FlowDelete { flow_id } => {
                                // Log before deleting (so we can capture the name)
                                if let Some(flow) = schedules::load_flow(&app_config.flows_dir(), &flow_id) {
                                    schedules::append_flow_log(&app_config.flow_logs_path(), &FlowLogEntry {
                                        timestamp: chrono::Local::now().to_rfc3339(),
                                        flow_id: flow.id.clone(),
                                        flow_name: flow.name.clone(),
                                        action: "deleted".to_string(),
                                        detail: String::new(),
                                        run_id: None,
                                    });
                                }
                                schedules::delete_flow(&app_config.flows_dir(), &flow_id);
                                let updated = schedules::list_flows(&app_config.flows_dir());
                                let _ = event_tx.send(BackendEvent::FlowsListed(updated));
                            }

                            FrontendCommand::FlowToggleEnabled { flow_id } => {
                                if let Some(mut flow) = schedules::load_flow(&app_config.flows_dir(), &flow_id) {
                                    flow.enabled = !flow.enabled;
                                    let _ = schedules::save_flow(&app_config.flows_dir(), &flow);
                                    let action = if flow.enabled { "enabled" } else { "disabled" };
                                    schedules::append_flow_log(&app_config.flow_logs_path(), &FlowLogEntry {
                                        timestamp: chrono::Local::now().to_rfc3339(),
                                        flow_id: flow.id.clone(),
                                        flow_name: flow.name.clone(),
                                        action: action.to_string(),
                                        detail: String::new(),
                                        run_id: None,
                                    });
                                }
                                let updated = schedules::list_flows(&app_config.flows_dir());
                                let _ = event_tx.send(BackendEvent::FlowsListed(updated));
                            }

                            FrontendCommand::FlowRunOnce { flow_id } => {
                                log::info!("[engine] FlowRunOnce -> emitting run_flow:{}", flow_id);
                                let _ = event_tx.send(debug_log(format!("FlowRunOnce: emitting run_flow for {}", flow_id)));
                                event_bus.emit("run_flow", &flow_id);
                            }

                            FrontendCommand::FlowLogsLoad => {
                                let logs = schedules::load_flow_logs(&app_config.flow_logs_path());
                                let _ = event_tx.send(BackendEvent::FlowLogsLoaded(logs));
                            }

                            FrontendCommand::FlowList => {
                                let flows = schedules::list_flows(&app_config.flows_dir());
                                let _ = event_tx.send(BackendEvent::FlowsListed(flows));
                            }

                            FrontendCommand::EventsLogLoad => {
                                let events: Vec<InternalEventDto> = event_bus.recent_events().into_iter().map(|e| {
                                    InternalEventDto {
                                        timestamp: e.timestamp,
                                        kind: e.kind,
                                        payload: e.payload,
                                    }
                                }).collect();
                                let _ = event_tx.send(BackendEvent::EventsLogUpdated(events));
                            }

                            FrontendCommand::IntegrationInstall { preset_id, api_keys: install_keys } => {
                                // Look in bundled presets first, then installed packs
                                let bundled_dir = app_config.integration_presets_dir().join(&preset_id);
                                let preset_dir = if bundled_dir.join("manifest.json").exists() {
                                    bundled_dir
                                } else {
                                    app_config.packs_dir().join(&preset_id)
                                };
                                let manifest = integrations::load_manifest(&preset_dir);

                                // Save API keys if provided
                                if !install_keys.is_empty() {
                                    let keys_path = app_config.keys_path();
                                    if let Ok(mut store) = KeyStore::load(&keys_path) {
                                        for (key_name, key_value) in &install_keys {
                                            store.upsert(key_name, key_value);
                                        }
                                        let _ = store.save(&keys_path);
                                        api_keys_cache = store.list_masked().into_iter()
                                            .map(|(n, m)| ApiKeyInfo { name: n, masked_key: m })
                                            .collect();
                                    }
                                }

                                // Copy skill files from preset to skills dir
                                if let Some(ref manifest) = manifest {
                                    for skill_file in &manifest.skills {
                                        let src = preset_dir.join(skill_file);
                                        let dst = skills_dir.join(skill_file);
                                        if src.exists() && !dst.exists() {
                                            let _ = std::fs::copy(&src, &dst);
                                        }
                                    }

                                    // Copy custom configs from preset dir to custom/{preset_id}/
                                    if !manifest.custom_configs.is_empty() {
                                        let custom_dest = app_config.custom_dir().join(&preset_id);
                                        let _ = std::fs::create_dir_all(&custom_dest);
                                        for config_path in &manifest.custom_configs {
                                            let src = preset_dir.join(config_path);
                                            if src.exists() {
                                                // Preserve relative path structure under custom/{preset_id}/
                                                let filename = std::path::Path::new(config_path)
                                                    .file_name()
                                                    .unwrap_or_default();
                                                let dst = custom_dest.join(filename);
                                                if !dst.exists() {
                                                    let _ = std::fs::copy(&src, &dst);
                                                }
                                            }
                                        }
                                    }
                                }

                                // Add to registry
                                let mut registry = integrations::IntegrationRegistry::load(&app_config.integrations_path());
                                registry.install(&preset_id);
                                let _ = registry.save(&app_config.integrations_path());

                                // Rebuild integrations list and notify frontend
                                integrations_cache = build_integrations_list(&app_config);
                                let _ = event_tx.send(BackendEvent::IntegrationsUpdated(integrations_cache.clone()));

                                let msg = format!("Integration '{}' installed.", preset_id);
                                messages.push(ChatMessageDto { role: "assistant".to_string(), content: msg.clone() });
                                emit(&BackendEvent::Info { message: msg });
                            }

                            FrontendCommand::IntegrationUninstall { preset_id } => {
                                // Load manifest to know what to clean up
                                let bundled = app_config.integration_presets_dir().join(&preset_id);
                                let preset_dir = if bundled.join("manifest.json").exists() {
                                    bundled
                                } else {
                                    app_config.packs_dir().join(&preset_id)
                                };
                                if let Some(manifest) = integrations::load_manifest(&preset_dir) {
                                    // Remove copied skill files
                                    for skill_file in &manifest.skills {
                                        let dst = skills_dir.join(skill_file);
                                        let _ = std::fs::remove_file(&dst);
                                    }
                                    // Remove custom configs directory
                                    if !manifest.custom_configs.is_empty() {
                                        let custom_dest = app_config.custom_dir().join(&preset_id);
                                        let _ = std::fs::remove_dir_all(&custom_dest);
                                    }
                                }

                                // Remove from registry
                                let mut registry = integrations::IntegrationRegistry::load(&app_config.integrations_path());
                                registry.uninstall(&preset_id);
                                let _ = registry.save(&app_config.integrations_path());

                                // Rebuild integrations list and notify frontend
                                integrations_cache = build_integrations_list(&app_config);
                                let _ = event_tx.send(BackendEvent::IntegrationsUpdated(integrations_cache.clone()));

                                let msg = format!("Integration '{}' uninstalled.", preset_id);
                                messages.push(ChatMessageDto { role: "assistant".to_string(), content: msg.clone() });
                                emit(&BackendEvent::Info { message: msg });
                            }

                            FrontendCommand::IntegrationImportFlow { preset_id } => {
                                let bundled = app_config.integration_presets_dir().join(&preset_id);
                                let preset_dir = if bundled.join("manifest.json").exists() {
                                    bundled
                                } else {
                                    app_config.packs_dir().join(&preset_id)
                                };
                                if let Some(manifest) = integrations::load_manifest(&preset_dir) {
                                    if let Some(ref template_file) = manifest.flow_template {
                                        let template_path = preset_dir.join(template_file);
                                        match std::fs::read_to_string(&template_path) {
                                            Ok(content) => {
                                                match serde_json::from_str::<SavedFlow>(&content) {
                                                    Ok(mut flow) => {
                                                        // Assign new ID and timestamps
                                                        flow.id = uuid::Uuid::new_v4().to_string();
                                                        let now = chrono::Local::now().to_rfc3339();
                                                        flow.created_at = now.clone();
                                                        flow.updated_at = now;
                                                        flow.enabled = false;
                                                        let flow_path = app_config.flows_dir().join(format!("{}.json", flow.id));
                                                        let json = serde_json::to_string_pretty(&flow).unwrap_or_default();
                                                        let _ = std::fs::write(&flow_path, &json);
                                                        let flows = schedules::list_flows(&app_config.flows_dir());
                                                        let _ = event_tx.send(BackendEvent::FlowsListed(flows));
                                                        let msg = format!("Flow template '{}' imported from {}.", flow.name, manifest.name);
                                                        messages.push(ChatMessageDto { role: "assistant".to_string(), content: msg.clone() });
                                                        emit(&BackendEvent::Info { message: msg });
                                                    }
                                                    Err(e) => {
                                                        let msg = format!("Failed to parse flow template: {}", e);
                                                        emit(&BackendEvent::Error { message: msg });
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                let msg = format!("Failed to read flow template: {}", e);
                                                emit(&BackendEvent::Error { message: msg });
                                            }
                                        }
                                    }
                                }
                            }

                            FrontendCommand::FlowListTemplates => {
                                let templates = build_flow_templates_list(&app_config);
                                let _ = event_tx.send(BackendEvent::FlowTemplatesListed(templates));
                            }

                            FrontendCommand::PacksList => {
                                let settings = starkbot_config::settings::Settings::load(&app_config.settings_path())
                                    .unwrap_or_default();
                                let client = starkbot_config::packs::PackClient::new(&settings, app_config.packs_dir());
                                let etx = event_tx.clone();
                                tokio::spawn(async move {
                                    match client.list_remote().await {
                                        Ok(summaries) => {
                                            let packs: Vec<PackInfo> = summaries.into_iter().map(|s| {
                                                let installed = client.is_installed(&s.slug);
                                                PackInfo {
                                                    slug: s.slug,
                                                    name: s.name,
                                                    description: s.description,
                                                    icon: s.icon,
                                                    installed,
                                                }
                                            }).collect();
                                            let _ = etx.send(BackendEvent::PacksListed(packs));
                                        }
                                        Err(e) => {
                                            let _ = etx.send(BackendEvent::PackError { message: e });
                                        }
                                    }
                                });
                            }

                            FrontendCommand::PackInstall { slug } => {
                                let settings = starkbot_config::settings::Settings::load(&app_config.settings_path())
                                    .unwrap_or_default();
                                let client = starkbot_config::packs::PackClient::new(&settings, app_config.packs_dir());
                                let etx = event_tx.clone();
                                let s = slug.clone();
                                tokio::spawn(async move {
                                    match client.install(&s).await {
                                        Ok(_) => {
                                            let _ = etx.send(BackendEvent::PackInstalled { slug: s });
                                        }
                                        Err(e) => {
                                            let _ = etx.send(BackendEvent::PackError { message: e });
                                        }
                                    }
                                });
                            }

                            FrontendCommand::PackUninstall { slug } => {
                                let settings = starkbot_config::settings::Settings::load(&app_config.settings_path())
                                    .unwrap_or_default();
                                let client = starkbot_config::packs::PackClient::new(&settings, app_config.packs_dir());
                                match client.uninstall(&slug) {
                                    Ok(_) => {
                                        let msg = format!("Pack '{}' uninstalled.", slug);
                                        emit(&BackendEvent::Info { message: msg });
                                    }
                                    Err(e) => {
                                        emit(&BackendEvent::PackError { message: e });
                                    }
                                }
                            }

                            FrontendCommand::Shutdown => {
                                shutdown_flag.store(true, Ordering::Relaxed);
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
                                    log::info!("[engine] Agent event: ToolCall {}", name);
                                    let _ = event_tx.send(debug_log(format!("Agent ToolCall: {}", name)));
                                    let activity = format!("▶ {} {}", name, args);
                                    tool_activity.push(activity);
                                    if tool_activity.len() > 20 { tool_activity.remove(0); }
                                    messages.push(ChatMessageDto {
                                        role: "tool".to_string(),
                                        content: format!("▶ {}({})", name, truncate_str(args, 80)),
                                    });
                                }
                                BackendEvent::ToolResult { name, success, preview } => {
                                    log::info!("[engine] Agent event: ToolResult {} success={}", name, success);
                                    let _ = event_tx.send(debug_log(format!("Agent ToolResult: {} ok={}", name, success)));
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
                                BackendEvent::ThinkingText { content } => {
                                    log::info!("[engine] Agent event: ThinkingText ({} chars)", content.len());
                                }
                                BackendEvent::TurnComplete { answer } => {
                                    log::info!("[engine] Agent event: TurnComplete ({} chars)", answer.len());
                                    let _ = event_tx.send(debug_log(format!("TurnComplete: {} chars", answer.len())));
                                    agent_busy = false;
                                    // Restore agent state for conversation continuity
                                    if let Some(mut rx) = state_rx.take() {
                                        match rx.try_recv() {
                                            Ok(s) => {
                                                log::info!("[engine] Agent state restored ({} messages)", s.messages.len());
                                                let _ = event_tx.send(debug_log(format!("State restored: {} msgs", s.messages.len())));
                                                agent_state = Some(s);
                                            }
                                            Err(_) => {
                                                log::warn!("[engine] Could not restore agent state");
                                                let _ = event_tx.send(debug_log("WARNING: agent state not available"));
                                            }
                                        }
                                    }
                                    messages.push(ChatMessageDto {
                                        role: "assistant".to_string(),
                                        content: answer.clone(),
                                    });

                                    // Save session to disk
                                    if let Some(ref sid) = current_session_id {
                                        let title = messages.iter()
                                            .find(|m| m.role == "user")
                                            .map(|m| {
                                                let trimmed = m.content.trim();
                                                if trimmed.is_empty() { return "Untitled".to_string(); }
                                                let t: String = trimmed.chars().take(60).collect();
                                                if trimmed.chars().count() > 60 { format!("{}...", t) } else { t }
                                            })
                                            .unwrap_or_else(|| "Untitled".to_string());
                                        let session_msgs: Vec<starkbot_config::sessions::ChatSessionMessage> = messages.iter()
                                            .filter(|m| m.role == "user" || m.role == "assistant")
                                            .map(|m| starkbot_config::sessions::ChatSessionMessage {
                                                role: m.role.clone(),
                                                content: m.content.clone(),
                                            })
                                            .collect();
                                        let session = starkbot_config::sessions::ChatSession {
                                            id: sid.clone(),
                                            persona: persona_name_for_session.clone(),
                                            title,
                                            created_at: session_created_at.clone().unwrap_or_else(|| chrono::Local::now().to_rfc3339()),
                                            messages: session_msgs,
                                        };
                                        let _ = starkbot_config::sessions::save_session(&app_config.sessions_dir(), &session);
                                        let updated = starkbot_config::sessions::list_sessions(&app_config.sessions_dir());
                                        let _ = event_tx.send(BackendEvent::SessionsUpdated(updated));
                                    }

                                    emit(&BackendEvent::StatusUpdate { busy: false, message: "Ready".to_string() });
                                }
                                BackendEvent::Error { message } => {
                                    log::error!("[engine] Agent event: Error: {}", message);
                                    let _ = event_tx.send(debug_log(format!("Agent Error: {}", message)));
                                    agent_busy = false;
                                    emit(&BackendEvent::StatusUpdate { busy: false, message: "Ready".to_string() });
                                }
                                _ => {}
                            }
                            // Forward to frontend
                            let _ = event_tx.send(evt);
                        } else {
                            log::info!("[engine] Agent event channel closed");
                            let _ = event_tx.send(debug_log("Agent event channel closed (executor done)"));
                            agent_event_rx = None;
                        }
                    }

                    // Poll approval channel (sync -> async bridge)
                    _ = tokio::time::sleep(std::time::Duration::from_millis(16)) => {
                        while let Ok((request, resp_tx)) = approval_rx.try_recv() {
                            log::info!("[engine] Approval request: {}", request.tool_name);
                            let _ = event_tx.send(debug_log(format!("Approval request: {}", request.tool_name)));
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
            skills_dir: self.app_config.skills_dir().display().to_string(),
            agents_dir: self.app_config.agents_dir().display().to_string(),
            sessions: self.sessions.clone(),
            sessions_dir: self.app_config.sessions_dir().display().to_string(),
            flows_dir: self.app_config.flows_dir().display().to_string(),
            inference_configured: !self.api_key.is_empty(),
            integrations: self.integrations.clone(),
            extension_server: self.app_config.settings().extension_server.clone(),
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
        "/clear" | "/new" => {
            messages.clear();
            tool_activity.clear();
            *agent_state = None;
            let msg = if input.trim() == "/new" {
                "New chat session started."
            } else {
                "Conversation cleared."
            };
            add_msg(messages, "assistant", msg);
            let _ = event_tx.send(BackendEvent::Info {
                message: msg.to_string(),
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
                "Commands: /new, /clear, /tokens, /help\nTab: switch views | Ctrl+C: quit"
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

fn build_integrations_list(app_config: &AppConfig) -> Vec<IntegrationPresetInfo> {
    let mut presets = integrations::list_presets(&app_config.integration_presets_dir());
    // Also include installed packs so they appear in the integrations list
    for (id, manifest) in integrations::list_presets(&app_config.packs_dir()) {
        if !presets.iter().any(|(existing_id, _)| existing_id == &id) {
            presets.push((id, manifest));
        }
    }
    let registry = integrations::IntegrationRegistry::load(&app_config.integrations_path());
    let key_store = KeyStore::load(&app_config.keys_path()).unwrap_or_default();

    presets
        .into_iter()
        .map(|(id, manifest)| {
            let installed = registry.is_installed(&id);
            // Check if all required keys are configured
            let configured = if !manifest.requires.api_keys.is_empty() {
                manifest.requires.api_keys.iter().all(|k| key_store.get(&k.name).is_some())
            } else {
                manifest.requires.api_key.as_ref()
                    .map(|k| key_store.get(k).is_some())
                    .unwrap_or(true)
            };
            let required_keys: Vec<RequiredKeyInfo> = manifest.requires.api_keys.iter()
                .map(|k| RequiredKeyInfo { name: k.name.clone(), label: k.label.clone() })
                .collect();
            IntegrationPresetInfo {
                id,
                name: manifest.name,
                description: manifest.description,
                icon: manifest.icon,
                api_key_name: manifest.requires.api_key,
                required_keys,
                skills: manifest.skills,
                installed,
                configured,
                has_flow_template: manifest.flow_template.is_some(),
            }
        })
        .collect()
}

fn build_flow_templates_list(app_config: &AppConfig) -> Vec<FlowTemplateInfo> {
    let mut presets = integrations::list_presets(&app_config.integration_presets_dir());
    for (id, manifest) in integrations::list_presets(&app_config.packs_dir()) {
        if !presets.iter().any(|(existing_id, _)| existing_id == &id) {
            presets.push((id, manifest));
        }
    }
    let registry = integrations::IntegrationRegistry::load(&app_config.integrations_path());
    let mut templates = Vec::new();
    for (id, manifest) in &presets {
        if registry.is_installed(id) {
            if let Some(ref template_file) = manifest.flow_template {
                // Try to read the flow name from the template file
                let template_path = app_config.integration_presets_dir().join(id).join(template_file);
                let template_name = if let Ok(content) = std::fs::read_to_string(&template_path) {
                    serde_json::from_str::<SavedFlow>(&content)
                        .map(|f| f.name)
                        .unwrap_or_else(|_| template_file.clone())
                } else {
                    template_file.clone()
                };
                templates.push(FlowTemplateInfo {
                    preset_id: id.clone(),
                    preset_name: manifest.name.clone(),
                    template_name,
                });
            }
        }
    }
    templates
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

/// Build a step guard that wraps an inner guard and emits real-time events.
fn build_emitting_guard(
    inner: StepGuard<AgentState>,
    emitter: mpsc::UnboundedSender<BackendEvent>,
) -> StepGuard<AgentState> {
    let seen_up_to = Arc::new(Mutex::new(0usize));
    Arc::new(move |state: &AgentState, event: &StepEvent| {
        // Log step transitions for debugging hangs
        log::info!("[guard] Step completed: node={} next={} msgs={}", event.node, event.next, state.messages.len());

        // Emit new messages as events
        let mut cursor = seen_up_to.lock().unwrap();
        if *cursor > state.messages.len() {
            *cursor = 0;
        }
        let new_messages = &state.messages[*cursor..];
        *cursor = state.messages.len();
        drop(cursor);

        for msg in new_messages {
            match msg {
                AgentMessage::ToolCall { name, args, .. } => {
                    let args_str = serde_json::to_string(args).unwrap_or_default();
                    let _ = emitter.send(BackendEvent::ToolCall { name: name.clone(), args: args_str });
                }
                AgentMessage::ToolResult { name, result, .. } => {
                    let success = !result.starts_with("ERROR:");
                    // Truncate preview to avoid flooding the event channel with huge payloads
                    let preview = if result.len() > 500 {
                        format!("{}...", &result[..500])
                    } else {
                        result.clone()
                    };
                    let _ = emitter.send(BackendEvent::ToolResult { name: name.clone(), success, preview });
                }
                AgentMessage::Assistant(text) => {
                    if !text.is_empty() {
                        let _ = emitter.send(BackendEvent::ThinkingText { content: text.clone() });
                    }
                }
                _ => {}
            }
        }

        // Delegate to inner guard for safety checks
        inner(state, event)
    })
}

/// Create a backend engine from config.
pub fn create_backend(config: BackendConfig) -> anyhow::Result<Box<dyn crate::backend::Backend>> {
    Ok(Box::new(StarkbotEngine::new(config)?))
}
