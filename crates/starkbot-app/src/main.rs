use crossterm::event::{self, Event};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use starkbot_api::{Backend, BackendConfig, FrontendCommand};
use starkbot_api::engine::StarkbotEngine;
use starkbot_tui::{draw, handle_key, TuiState};
use std::io;
use std::time::Duration;

// One-shot mode still uses AgentRunner directly for simplicity
use metalcraft::{AgentState, RunOutcome};
use starkbot_core::dispatch::AgentRunner;
use starkbot_core::persona::Persona;
use starkbot_tools::approval::ApprovalMode;
use starkbot_config::AppConfig;
use starkbot_config::keys::KeyStore;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let auto_approve = args.iter().any(|a| a == "--auto-approve");
    let args: Vec<String> = args.into_iter().filter(|a| a != "--auto-approve").collect();

    let persona_slug = args.first().map(|s| s.as_str()).unwrap_or("starkbot");
    let one_shot_task = if args.len() > 1 { Some(args[1..].join(" ")) } else { None };

    // One-shot mode: no TUI, just run and print
    if let Some(task) = one_shot_task {
        return run_oneshot(persona_slug, auto_approve, &task).await;
    }

    // TUI mode — use the API layer
    let config = BackendConfig {
        persona_slug: persona_slug.to_string(),
        api_key: String::new(), // will be resolved from env/keys.json
        model_name: String::new(), // will be resolved from env/settings.json
        auto_approve,
    };

    let mut engine = StarkbotEngine::new(config)?;
    let handle = engine.start().await?;

    let mut tui_state = TuiState::from_snapshot(&handle.initial_snapshot);
    let mut event_rx = handle.events;
    let cmd_tx = handle.commands;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    loop {
        terminal.draw(|f| draw(f, &tui_state))?;

        // Check for backend events (non-blocking)
        while let Ok(evt) = event_rx.try_recv() {
            tui_state.apply_event(&evt);
        }

        // Handle terminal input
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if let Some(cmd) = handle_key(&mut tui_state, key) {
                    // Check for quit commands
                    match &cmd {
                        FrontendCommand::SendMessage { content } if content == "/quit" || content == "/exit" => {
                            tui_state.should_quit = true;
                        }
                        _ => {
                            let _ = cmd_tx.send(cmd);
                        }
                    }
                }
            }
        }

        if tui_state.should_quit {
            break;
        }
    }

    // Shutdown engine
    let _ = cmd_tx.send(FrontendCommand::Shutdown);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

async fn run_oneshot(
    persona_slug: &str,
    auto_approve: bool,
    task: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_config = AppConfig::open();
    let bundled_agents = find_bundled_dir("agents");
    let bundled_skills = find_bundled_dir("skills");
    let bundled_presets = find_bundled_dir("integration_presets");
    app_config.ensure_initialized(bundled_agents.as_deref(), bundled_skills.as_deref(), bundled_presets.as_deref())
        .unwrap_or_else(|e| eprintln!("Warning: config init failed: {}", e));

    let personas_dir = Persona::default_personas_dir();
    let config_agents_dir = app_config.agents_dir();
    let config_skills_dir = app_config.skills_dir();
    let skills_dir = if config_skills_dir.is_dir() && std::fs::read_dir(&config_skills_dir).map(|mut d| d.next().is_some()).unwrap_or(false) {
        config_skills_dir
    } else {
        Persona::default_skills_dir()
    };

    let persona = match Persona::load_with_config(persona_slug, Some(&config_agents_dir), &personas_dir) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let store = KeyStore::load(&app_config.keys_path()).unwrap_or_default();
    let api_key = match store.get("OPENAI_API_KEY").map(|s| s.to_string()) {
        Some(key) => key,
        None => {
            eprintln!("Error: No OpenAI API key configured.");
            eprintln!("Configure it in the GUI (Settings > Inference) or run:");
            eprintln!("  starkbot-cli install_integration OPENAI_API_KEY <your-key>");
            std::process::exit(1);
        }
    };

    let settings = starkbot_config::settings::Settings::load(&app_config.settings_path()).unwrap_or_default();
    let model_name = std::env::var("OPENAI_MODEL").unwrap_or(settings.model.clone());
    let cwd = std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_else(|_| ".".to_string());
    let keys_path = app_config.keys_path();

    let approval_mode = if auto_approve {
        ApprovalMode::AutoApprove
    } else {
        ApprovalMode::default_interactive()
    };

    println!("╭─────────────────────────────────────────────╮");
    println!("│  StarkBot CLI                                │");
    println!("│  Persona: {:<33}│", persona.name());
    println!("│  Model:   {:<33}│", model_name);
    println!("╰─────────────────────────────────────────────╯");
    println!("\nTask: {}\n", task);

    let runner = AgentRunner::build_with_keys(&persona, &skills_dir, &cwd, &api_key, &model_name, approval_mode, Some(keys_path.to_path_buf()))?;
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
