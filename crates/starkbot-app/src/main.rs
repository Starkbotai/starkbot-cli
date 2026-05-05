use crossterm::event::{self, Event};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use metalcraft::{AgentMessage, AgentState, RunOutcome};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use starkbot_core::context;
use starkbot_core::dispatch::AgentRunner;
use starkbot_core::persona::Persona;
use starkbot_db::Database;
use starkbot_graph::build_skill_graph;
use starkbot_skills::SkillRegistry;
use starkbot_tools::approval::{self, ApprovalMode};
use starkbot_tui::{draw, handle_key, PendingApproval, TuiState};
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

enum AgentEvent {
    ToolCall(String, String),
    ToolResult(String, bool, String),
    Done(String),
    Error(String),
}

fn default_db_path() -> PathBuf {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("starkbot");
    std::fs::create_dir_all(&data_dir).ok();
    data_dir.join("starkbot.db")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let auto_approve = args.iter().any(|a| a == "--auto-approve");
    let args: Vec<String> = args.into_iter().filter(|a| a != "--auto-approve").collect();

    let persona_slug = args.first().map(|s| s.as_str()).unwrap_or("starkbot");
    let one_shot_task = if args.len() > 1 { Some(args[1..].join(" ")) } else { None };

    let personas_dir = Persona::default_personas_dir();
    let skills_dir = Persona::default_skills_dir();

    let persona = match Persona::load(persona_slug, &personas_dir) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("\nAvailable personas:");
            for slug in Persona::list_available(&personas_dir) {
                eprintln!("  {}", slug);
            }
            std::process::exit(1);
        }
    };

    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let model_name = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5.4".to_string());

    let approval_mode = if auto_approve {
        ApprovalMode::AutoApprove
    } else {
        ApprovalMode::default_interactive()
    };

    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let db_path = default_db_path();

    // Initialize DB on startup (ensures tables exist)
    let _db = Database::open(&db_path)?;

    // Populate skill FTS index
    {
        let db = Database::open(&db_path)?;
        let skill_registry = SkillRegistry::load_from_dir(&skills_dir);
        for skill in skill_registry.all() {
            db.upsert_skill_fts(
                &skill.name,
                &skill.description,
                &skill.tags.join(" "),
                &skill.content,
            ).ok();
        }
    }

    // One-shot mode: no TUI, just run and print
    if let Some(task) = one_shot_task {
        return run_oneshot(&persona, &skills_dir, &cwd, &api_key, &model_name, approval_mode, &task, &db_path).await;
    }

    // TUI mode — use channel-based approval so prompts render in-frame
    let (approval_tx, approval_rx) = approval::approval_channel();
    let tui_approval_mode = if auto_approve {
        ApprovalMode::AutoApprove
    } else {
        ApprovalMode::tui_interactive(approval_tx.clone())
    };
    let mut runner = AgentRunner::build_for_tui(&persona, &skills_dir, &cwd, &api_key, &model_name, tui_approval_mode.clone(), Some(db_path.clone()))?;

    // Load skills for the graph
    let skill_registry = SkillRegistry::load_from_dir(&skills_dir);
    let skill_data: Vec<(String, String, Vec<String>)> = skill_registry.all().iter()
        .map(|s| (s.name.clone(), s.description.clone(), s.tags.clone()))
        .collect();
    let skill_graph = build_skill_graph(&skill_data);

    let mut tui_state = TuiState::new(persona.name(), &model_name);
    tui_state.skill_names = skill_registry.names().into_iter().map(String::from).collect();
    tui_state.skills = {
        let mut skills: Vec<_> = skill_registry.all().into_iter().cloned().collect();
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        skills
    };
    tui_state.skill_graph = skill_graph;

    // Load personas for TUI tab
    {
        let slugs = Persona::list_available(&personas_dir);
        let mut loaded: Vec<_> = slugs.iter()
            .filter_map(|s| Persona::load(s, &personas_dir).ok())
            .collect();
        loaded.sort_by_key(|p| p.sort_order);
        tui_state.personas = loaded;
    }

    // Load API keys for TUI tab
    {
        let db = Database::open(&db_path)?;
        tui_state.api_keys = db.list_api_keys().unwrap_or_default();
    }
    tui_state.db_path = Some(db_path.clone());

    tui_state.add_message("assistant", &format!(
        "Hello! I'm {}. How can I help?", persona.name()
    ));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let (agent_tx, mut agent_rx) = mpsc::unbounded_channel::<AgentEvent>();
    let mut agent_state: Option<AgentState> = None;

    loop {
        terminal.draw(|f| draw(f, &tui_state))?;

        // Check for agent events (non-blocking)
        while let Ok(evt) = agent_rx.try_recv() {
            match evt {
                AgentEvent::ToolCall(name, args) => {
                    tui_state.add_tool_activity(&format!("▶ {} {}", name, args));
                    tui_state.add_message("tool", &format!("▶ {}({})", name, truncate_str(&args, 80)));
                }
                AgentEvent::ToolResult(name, success, preview) => {
                    let icon = if success { "✓" } else { "✗" };
                    tui_state.add_tool_activity(&format!("{} {}", icon, name));
                    if !success {
                        tui_state.add_message("error", &format!("{} {} failed: {}", icon, name, truncate_str(&preview, 100)));
                    }
                }
                AgentEvent::Done(answer) => {
                    tui_state.agent_busy = false;
                    tui_state.status = "Ready".to_string();
                    tui_state.add_message("assistant", &answer);
                }
                AgentEvent::Error(err) => {
                    tui_state.agent_busy = false;
                    tui_state.status = "Ready".to_string();
                    tui_state.add_message("error", &err);
                }
            }
        }

        // Check for approval requests (non-blocking)
        if tui_state.pending_approval.is_none() {
            if let Ok((request, resp_tx)) = approval_rx.try_recv() {
                tui_state.pending_approval = Some(PendingApproval { request, response_tx: resp_tx });
            }
        }

        // Handle TUI-level actions (e.g., API key add/delete, model change)
        handle_tui_actions(&mut tui_state);

        // Handle model change separately (needs runner access)
        if let Some(starkbot_tui::TuiAction::ChangeModel { model }) = tui_state.pending_action.take() {
            match AgentRunner::build_for_tui(&persona, &skills_dir, &cwd, &api_key, &model, tui_approval_mode.clone(), Some(db_path.clone())) {
                Ok(new_runner) => {
                    runner = new_runner;
                    tui_state.model_name = model.clone();
                    tui_state.add_message("assistant", &format!("Model switched to {}.", model));
                }
                Err(e) => {
                    tui_state.add_message("error", &format!("Failed to switch model: {}", e));
                }
            }
        }

        // Handle input
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if let Some(input) = handle_key(&mut tui_state, key) {
                    // Check for slash commands
                    if input.starts_with('/') {
                        handle_command(&input, &mut tui_state, &mut agent_state);
                        continue;
                    }

                    // Send to agent
                    tui_state.add_message("user", &input);
                    tui_state.agent_busy = true;
                    tui_state.status = "Agent thinking...".to_string();

                    let turn_state = match agent_state.take() {
                        Some(prev) => prev.continue_with(&input),
                        None => AgentState::new(&input),
                    };

                    let runner_graph = runner.graph.clone();
                    let guard = runner.step_guard.clone();
                    let tx = agent_tx.clone();

                    tokio::spawn(async move {
                        let executor = metalcraft::Executor::new_from_arc(runner_graph)
                            .max_steps(100)
                            .with_step_guard(guard);

                        match executor.run(turn_state, "agent").await {
                            Ok(RunOutcome::Completed(state)) => {
                                // Extract tool activity from state messages
                                for msg in &state.messages {
                                    match msg {
                                        AgentMessage::ToolCall { name, args, .. } => {
                                            let args_str = serde_json::to_string(args).unwrap_or_default();
                                            let _ = tx.send(AgentEvent::ToolCall(name.clone(), args_str));
                                        }
                                        AgentMessage::ToolResult { name, result, .. } => {
                                            let success = !result.starts_with("ERROR:");
                                            let _ = tx.send(AgentEvent::ToolResult(name.clone(), success, result.clone()));
                                        }
                                        _ => {}
                                    }
                                }
                                let answer = state.final_answer().unwrap_or("(no answer)").to_string();
                                let _ = tx.send(AgentEvent::Done(answer));
                            }
                            Ok(RunOutcome::Interrupted { reason, .. }) => {
                                let _ = tx.send(AgentEvent::Error(format!("Interrupted: {}", reason)));
                            }
                            Err(e) => {
                                let _ = tx.send(AgentEvent::Error(format!("Error: {}", e)));
                            }
                        }
                    });
                }
            }
        }

        if tui_state.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn handle_tui_actions(state: &mut TuiState) {
    if let Some(action) = state.pending_action.take() {
        match action {
            starkbot_tui::TuiAction::AddApiKey { name, key } => {
                if let Some(ref db_path) = state.db_path {
                    if let Ok(db) = Database::open(db_path) {
                        if db.upsert_api_key(&name, &key).is_ok() {
                            state.api_keys = db.list_api_keys().unwrap_or_default();
                            state.add_message("assistant", &format!("API key {} installed.", name));
                        }
                    }
                }
            }
            starkbot_tui::TuiAction::DeleteApiKey { name } => {
                if let Some(ref db_path) = state.db_path {
                    if let Ok(db) = Database::open(db_path) {
                        if db.delete_api_key(&name).unwrap_or(false) {
                            state.api_keys = db.list_api_keys().unwrap_or_default();
                            state.add_message("assistant", &format!("API key {} deleted.", name));
                        }
                    }
                }
            }
            // ChangeModel is handled in the main loop (needs runner access)
            other @ starkbot_tui::TuiAction::ChangeModel { .. } => {
                state.pending_action = Some(other);
            }
        }
    }
}

fn handle_command(input: &str, state: &mut TuiState, agent_state: &mut Option<AgentState>) {
    match input.trim() {
        "/quit" | "/exit" => state.should_quit = true,
        "/clear" => {
            state.messages.clear();
            state.tool_activity.clear();
            *agent_state = None;
            state.add_message("assistant", "Conversation cleared.");
        }
        "/tokens" => {
            if let Some(s) = agent_state {
                let tokens = context::estimate_tokens(s);
                state.add_message("assistant", &format!("~{} tokens, {} messages", tokens, s.messages.len()));
            } else {
                state.add_message("assistant", "No conversation yet.");
            }
        }
        cmd if cmd.starts_with("/help") => {
            state.add_message("assistant", "Commands: /quit, /clear, /tokens, /help\nTab: switch views | Ctrl+C: quit");
        }
        _ => {
            state.add_message("error", &format!("Unknown command: {}", input));
        }
    }
}

async fn run_oneshot(
    persona: &Persona,
    skills_dir: &std::path::Path,
    cwd: &str,
    api_key: &str,
    model_name: &str,
    approval_mode: ApprovalMode,
    task: &str,
    db_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("╭─────────────────────────────────────────────╮");
    println!("│  StarkBot CLI                                │");
    println!("│  Persona: {:<33}│", persona.name());
    println!("│  Model:   {:<33}│", model_name);
    println!("╰─────────────────────────────────────────────╯");
    println!("\nTask: {}\n", task);

    let runner = AgentRunner::build_with_db(persona, skills_dir, cwd, api_key, model_name, approval_mode, Some(db_path.to_path_buf()))?;
    let state = AgentState::new(task);

    match runner.run(state).await? {
        RunOutcome::Completed(state) => {
            println!("\n--- Done ---");
            println!("{}", state.final_answer().unwrap_or("(no answer)"));
        }
        RunOutcome::Interrupted { reason, .. } => {
            println!("\nInterrupted: {}", reason);
        }
    }

    Ok(())
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max]) }
}
