//! Spice test that simulates the DOTA check-in flow end-to-end.
//!
//! This mirrors exactly what happens when the engine executes the flow:
//! - Loads the starkbot persona
//! - Stubs the integration install into a temp dir (keys + skill)
//! - Wraps the prompt in [AUTONOMOUS MODE] just like the engine
//! - Runs with AutoApprove (no interactive approval)
//!
//! Run with:
//!   OPENAI_API_KEY=sk-... DOTA_API_KEY=wc2a_... cargo test -p starkbot-app \
//!     --test dota_flow_spice_test -- --ignored --nocapture

use metalcraft::{AgentState, Executor, RunOutcome};
use starkbot_core::dispatch::AgentRunner;
use starkbot_tools::approval::ApprovalMode;
use std::time::Instant;

/// The exact prompt from the DOTA check-in flow template.
const DOTA_FLOW_PROMPT: &str = r#"You are an AI agent playing Defense of the Agents. Execute this game loop:

Step 1 — Load the skill:
  load_skill("dota-game")

Step 2 — Get credentials:
  api_key_read("DOTA_API_KEY") → store as $API_KEY
  api_key_read("DOTA_AGENT_NAME") → store as $AGENT_NAME

Step 3 — Check game state:
  bash: curl -s https://wc2-agentic-dev-3o6un.ondigitalocean.app/api/game/state?game=3
  Parse the JSON. Find your hero by matching $AGENT_NAME in the heroes array.

Step 4 — Decide and deploy:
  Based on the game state, build a deployment JSON and POST it:
  bash: curl -s -X POST https://wc2-agentic-dev-3o6un.ondigitalocean.app/api/strategy/deployment \
    -H "Authorization: Bearer $API_KEY" \
    -H "Content-Type: application/json" \
    -d '{...your deployment...}'

  If this is your FIRST deployment, include heroClass and heroLane.
  If you have abilityChoices, pick one with abilityChoice.
  If HP is low, use action: "recall".
  Always include a message describing your strategy.

Step 5 — Summarize what you did and why in 2-3 sentences."#;

/// Stub-install the DOTA integration into a temp directory.
/// Returns (keys_path, skills_dir) pointing to the temp dir.
fn stub_dota_install(
    tmp: &std::path::Path,
    dota_api_key: &str,
    agent_name: &str,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let skills_dir = tmp.join("skills");
    std::fs::create_dir_all(&skills_dir).unwrap();

    // Copy the real dota-game.md skill from the integration preset
    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .to_path_buf();
    let preset_skill = workspace_root.join("test_fixtures/dota/dota-game.md");
    std::fs::copy(&preset_skill, skills_dir.join("dota-game.md"))
        .expect("Failed to copy dota-game.md from test_fixtures/dota/");

    // Write keys to a temp keystore
    let keys_path = tmp.join("keys.json");
    let keys_json = serde_json::json!({
        "DOTA_API_KEY": dota_api_key,
        "DOTA_AGENT_NAME": agent_name,
    });
    std::fs::write(&keys_path, serde_json::to_string_pretty(&keys_json).unwrap()).unwrap();

    // Create custom dir (strategy.json)
    let custom_dir = tmp.join("custom/defense-of-the-agents");
    std::fs::create_dir_all(&custom_dir).unwrap();
    let strategy = serde_json::json!({
        "preferredHeroClass": "melee",
        "laneFocus": "mid",
        "behavior": "Balanced play — protect towers and farm safely."
    });
    std::fs::write(
        custom_dir.join("strategy.json"),
        serde_json::to_string_pretty(&strategy).unwrap(),
    ).unwrap();

    (keys_path, skills_dir)
}

#[tokio::test]
#[ignore]
async fn dota_flow_end_to_end() {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set");
    let model_name = std::env::var("OPENAI_MODEL")
        .unwrap_or_else(|_| "gpt-4.1-mini".to_string());

    let dota_api_key = std::env::var("DOTA_API_KEY")
        .unwrap_or_else(|_| {
            // Fall back to reading from real keystore
            let app_config = starkbot_config::AppConfig::open();
            let store = starkbot_config::keys::KeyStore::load(&app_config.keys_path())
                .expect("Failed to load keystore");
            store.get("DOTA_API_KEY")
                .expect("DOTA_API_KEY not set (env or keystore)")
                .to_string()
        });
    let agent_name = std::env::var("DOTA_AGENT_NAME")
        .unwrap_or_else(|_| {
            let app_config = starkbot_config::AppConfig::open();
            let store = starkbot_config::keys::KeyStore::load(&app_config.keys_path())
                .expect("Failed to load keystore");
            store.get("DOTA_AGENT_NAME")
                .unwrap_or("TestAgent")
                .to_string()
        });

    // Stub the integration install into a temp dir
    let tmp = tempfile::tempdir().expect("Failed to create temp dir");
    let (keys_path, skills_dir) = stub_dota_install(tmp.path(), &dota_api_key, &agent_name);

    println!("=== DOTA Flow Spice Test ===");
    println!("Agent name: {}", agent_name);
    println!("Model: {}", model_name);
    println!("Temp dir: {}", tmp.path().display());

    // Resolve workspace paths
    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .to_path_buf();

    // Load persona — same one the engine uses
    let personas_dir = workspace_root.join("personas");
    let persona = starkbot_core::persona::Persona::load("starkbot", &personas_dir)
        .expect("Failed to load starkbot persona");

    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".into());

    // Build the runner exactly like the flow engine does
    let runner = AgentRunner::build_for_tui(
        &persona,
        &skills_dir,
        &cwd,
        &api_key,
        &model_name,
        ApprovalMode::AutoApprove,
        Some(keys_path),
    ).expect("Failed to build agent runner");

    // Wrap in autonomous mode — exactly like the engine does for flows
    let autonomous_prompt = format!(
        "[AUTONOMOUS MODE] You are executing a flow step. \
         You cannot ask the user follow-up questions — you must \
         do your best with the information given. Use your tools \
         to research and gather any missing information. \
         Produce a complete answer.\n\n{}", DOTA_FLOW_PROMPT
    );

    println!("\n--- Prompt ---\n{}\n--- End Prompt ---\n", DOTA_FLOW_PROMPT);

    let state = AgentState::new(&autonomous_prompt);
    let executor = Executor::new_from_arc(runner.graph.clone())
        .max_steps(100)
        .with_step_guard(runner.step_guard.clone());

    let start = Instant::now();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(180),
        executor.run(state, "dota-flow-test"),
    ).await;

    let duration = start.elapsed();

    match result {
        Ok(Ok(RunOutcome::Completed(state))) => {
            let answer = state.final_answer().unwrap_or("(no answer)");
            let tools = state.tools_called();

            println!("\n=== RESULT (completed in {:.1}s) ===", duration.as_secs_f64());
            println!("Tools called: {:?}", tools);
            println!("\n--- Agent Answer ---\n{}\n--- End Answer ---", answer);

            // Verify the agent actually used the right tools
            assert!(
                tools.contains(&"load_skill".to_string()),
                "Agent should have called load_skill. Tools called: {:?}", tools
            );
            assert!(
                tools.contains(&"api_key_read".to_string()),
                "Agent should have called api_key_read. Tools called: {:?}", tools
            );
            assert!(
                tools.contains(&"bash".to_string()),
                "Agent should have called bash (for curl). Tools called: {:?}", tools
            );

            // Verify the answer mentions game-related content
            let answer_lower = answer.to_lowercase();
            assert!(
                answer_lower.contains("deploy") || answer_lower.contains("lane")
                    || answer_lower.contains("game") || answer_lower.contains("hero")
                    || answer_lower.contains("strategy"),
                "Agent answer should mention game concepts. Got: {}", answer
            );

            println!("\n=== PASS ===");
        }
        Ok(Ok(RunOutcome::Interrupted { state, reason, .. })) => {
            let answer = state.final_answer().unwrap_or("(no answer)");
            println!("\n=== INTERRUPTED after {:.1}s ===", duration.as_secs_f64());
            println!("Reason: {}", reason);
            println!("Tools called: {:?}", state.tools_called());
            println!("Partial answer: {}", answer);
            panic!("Flow was interrupted: {}", reason);
        }
        Ok(Err(e)) => {
            println!("\n=== ERROR after {:.1}s ===", duration.as_secs_f64());
            panic!("Executor error: {}", e);
        }
        Err(_) => {
            println!("\n=== TIMEOUT after {:.1}s ===", duration.as_secs_f64());
            panic!("Flow timed out after 180s");
        }
    }
}
