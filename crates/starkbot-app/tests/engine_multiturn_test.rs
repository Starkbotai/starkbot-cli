use starkbot_api::{BackendConfig, BackendEvent, Backend, FrontendCommand, StarkbotEngine};
use tokio::sync::mpsc;
use std::time::Duration;

/// Collect events from the backend until TurnComplete or Error, with a timeout.
async fn collect_until_turn_complete(
    events: &mut mpsc::UnboundedReceiver<BackendEvent>,
    timeout: Duration,
) -> Vec<BackendEvent> {
    let mut collected = Vec::new();
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        tokio::select! {
            evt = events.recv() => {
                match evt {
                    Some(evt) => {
                        let is_terminal = matches!(
                            &evt,
                            BackendEvent::TurnComplete { .. } | BackendEvent::Error { .. }
                        );
                        collected.push(evt);
                        if is_terminal {
                            return collected;
                        }
                    }
                    None => {
                        panic!("Event channel closed before TurnComplete");
                    }
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                panic!(
                    "Timed out waiting for TurnComplete after {:?}. Collected {} events: {:?}",
                    timeout,
                    collected.len(),
                    collected.iter().map(event_summary).collect::<Vec<_>>()
                );
            }
        }
    }
}

/// Like collect_until_turn_complete, but also auto-approves any ApprovalRequired events.
async fn collect_until_turn_complete_with_auto_approve(
    events: &mut mpsc::UnboundedReceiver<BackendEvent>,
    commands: &mpsc::UnboundedSender<FrontendCommand>,
    timeout: Duration,
) -> Vec<BackendEvent> {
    let mut collected = Vec::new();
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        tokio::select! {
            evt = events.recv() => {
                match evt {
                    Some(evt) => {
                        // Auto-approve any approval requests
                        if let BackendEvent::ApprovalRequired { ref request_id, ref tool_name, .. } = evt {
                            eprintln!("  Auto-approving: {} ({})", tool_name, request_id);
                            let _ = commands.send(FrontendCommand::ApprovalResponse {
                                request_id: request_id.clone(),
                                approved: true,
                            });
                        }
                        let is_terminal = matches!(
                            &evt,
                            BackendEvent::TurnComplete { .. } | BackendEvent::Error { .. }
                        );
                        collected.push(evt);
                        if is_terminal {
                            return collected;
                        }
                    }
                    None => {
                        panic!("Event channel closed before TurnComplete");
                    }
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                panic!(
                    "Timed out waiting for TurnComplete after {:?}. Collected {} events: {:?}",
                    timeout,
                    collected.len(),
                    collected.iter().map(event_summary).collect::<Vec<_>>()
                );
            }
        }
    }
}

/// Extract the answer string from a collected event list.
fn extract_answer(events: &[BackendEvent]) -> String {
    for evt in events {
        if let BackendEvent::TurnComplete { answer } = evt {
            return answer.clone();
        }
    }
    String::new()
}

/// Check whether a specific tool was called in the event list.
fn assert_tool_called(events: &[BackendEvent], tool_name: &str) {
    let found = events.iter().any(|e| matches!(e, BackendEvent::ToolCall { name, .. } if name == tool_name));
    assert!(
        found,
        "Expected tool '{}' to be called. Tools called: {:?}",
        tool_name,
        events.iter()
            .filter_map(|e| if let BackendEvent::ToolCall { name, .. } = e { Some(name.as_str()) } else { None })
            .collect::<Vec<_>>()
    );
}

/// Short summary of an event for debug output.
fn event_summary(evt: &BackendEvent) -> String {
    match evt {
        BackendEvent::ToolCall { name, .. } => format!("ToolCall({})", name),
        BackendEvent::ToolResult { name, success, .. } => format!("ToolResult({}, ok={})", name, success),
        BackendEvent::TurnComplete { answer } => format!("TurnComplete({}ch)", answer.len()),
        BackendEvent::ThinkingText { content } => format!("Thinking({}ch)", content.len()),
        BackendEvent::Error { message } => format!("Error({})", message),
        BackendEvent::StatusUpdate { busy, message } => format!("Status(busy={}, {})", busy, message),
        BackendEvent::ApprovalRequired { tool_name, .. } => format!("Approval({})", tool_name),
        BackendEvent::Info { message } => format!("Info({})", &message[..message.len().min(40)]),
        BackendEvent::DebugLog { message, .. } => format!("Debug({})", &message[..message.len().min(40)]),
        _ => format!("{:?}", evt),
    }
}

/// Set cwd to workspace root so persona/skill resolution works.
fn ensure_workspace_root() {
    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .to_path_buf();
    let _ = std::env::set_current_dir(&workspace_root);
}

/// Build a BackendConfig from env vars.
fn test_config(persona: &str) -> BackendConfig {
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set for engine tests");
    let model_name = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5.4".to_string());

    BackendConfig {
        persona_slug: persona.to_string(),
        api_key,
        model_name,
        auto_approve: true,
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn multi_turn_conversation() {
    dotenvy::dotenv().ok();
    env_logger::try_init().ok();
    ensure_workspace_root();

    let config = test_config("coding-agent");
    let mut engine = StarkbotEngine::new(config).expect("Failed to create engine");
    let mut handle = engine.start().await.expect("Failed to start engine");

    let timeout = Duration::from_secs(120);

    // Turn 1: Ask a question
    handle.commands.send(FrontendCommand::SendMessage {
        content: "What is the Rust ownership model? Keep your answer to 2-3 sentences.".into(),
    }).expect("Failed to send message");

    let events1 = collect_until_turn_complete(&mut handle.events, timeout).await;
    let answer1 = extract_answer(&events1);
    assert!(!answer1.is_empty(), "Turn 1 should produce a non-empty answer");
    assert!(
        answer1.len() >= 20,
        "Turn 1 answer too short ({} chars): {}",
        answer1.len(),
        answer1
    );

    // Turn 2: Follow-up that requires context from turn 1
    handle.commands.send(FrontendCommand::SendMessage {
        content: "Can you give me a short code example demonstrating what you just explained?".into(),
    }).expect("Failed to send message");

    let events2 = collect_until_turn_complete(&mut handle.events, timeout).await;
    let answer2 = extract_answer(&events2);
    assert!(!answer2.is_empty(), "Turn 2 should produce a non-empty answer");
    // The follow-up should reference ownership/borrowing concepts from turn 1
    let answer2_lower = answer2.to_lowercase();
    let has_context = answer2_lower.contains("own")
        || answer2_lower.contains("borrow")
        || answer2_lower.contains("move")
        || answer2_lower.contains("fn ")
        || answer2_lower.contains("let ");
    assert!(
        has_context,
        "Turn 2 should maintain context from turn 1. Got: {}",
        &answer2[..answer2.len().min(300)]
    );

    // Shutdown
    handle.commands.send(FrontendCommand::Shutdown).expect("Failed to send shutdown");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn multi_turn_skill_loading() {
    dotenvy::dotenv().ok();
    env_logger::try_init().ok();
    ensure_workspace_root();

    let config = test_config("coding-agent");
    let mut engine = StarkbotEngine::new(config).expect("Failed to create engine");
    let mut handle = engine.start().await.expect("Failed to start engine");

    let timeout = Duration::from_secs(120);

    // Turn 1: Ask about a topic — use explicit instruction to load skill
    handle.commands.send(FrontendCommand::SendMessage {
        content: "Use the load_skill tool to load a skill about testing, then explain what you learned.".into(),
    }).expect("Failed to send message");

    let events1 = collect_until_turn_complete(&mut handle.events, timeout).await;
    let answer1 = extract_answer(&events1);
    assert!(!answer1.is_empty(), "Should produce a non-empty answer");

    // Check if load_skill was called (informational, not a hard failure)
    let skill_loaded = events1.iter().any(|e| matches!(e, BackendEvent::ToolCall { name, .. } if name == "load_skill"));
    if !skill_loaded {
        eprintln!("NOTE: load_skill was not called on turn 1. Tools called: {:?}",
            events1.iter()
                .filter_map(|e| if let BackendEvent::ToolCall { name, .. } = e { Some(name.as_str()) } else { None })
                .collect::<Vec<_>>());
    }

    // Turn 2: Follow-up that requires context from turn 1
    handle.commands.send(FrontendCommand::SendMessage {
        content: "Can you show me an example based on what you just described?".into(),
    }).expect("Failed to send message");

    let events2 = collect_until_turn_complete(&mut handle.events, timeout).await;
    let answer2 = extract_answer(&events2);
    assert!(!answer2.is_empty(), "Turn 2 should produce a non-empty answer");

    // Shutdown
    handle.commands.send(FrontendCommand::Shutdown).expect("Failed to send shutdown");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn multi_turn_tool_use() {
    dotenvy::dotenv().ok();
    env_logger::try_init().ok();
    ensure_workspace_root();

    let config = test_config("coding-agent");
    let mut engine = StarkbotEngine::new(config).expect("Failed to create engine");
    let mut handle = engine.start().await.expect("Failed to start engine");

    let timeout = Duration::from_secs(120);

    // Turn 1: Ask agent to read a file
    handle.commands.send(FrontendCommand::SendMessage {
        content: "Read the file Cargo.toml and tell me the package name.".into(),
    }).expect("Failed to send message");

    let events1 = collect_until_turn_complete(&mut handle.events, timeout).await;
    assert_tool_called(&events1, "read_file");
    let answer1 = extract_answer(&events1);
    assert!(!answer1.is_empty(), "Should produce an answer about the package");

    // Turn 2: Ask a follow-up about the same file
    handle.commands.send(FrontendCommand::SendMessage {
        content: "What dependencies does it have?".into(),
    }).expect("Failed to send message");

    let events2 = collect_until_turn_complete(&mut handle.events, timeout).await;
    let answer2 = extract_answer(&events2);
    assert!(!answer2.is_empty(), "Turn 2 should produce a non-empty answer about dependencies");

    // Shutdown
    handle.commands.send(FrontendCommand::Shutdown).expect("Failed to send shutdown");
}

/// Simulates the exact flow: greeting → topic question with tool use → completion.
/// This verifies that a multi-turn conversation with tool calls runs to completion
/// without hanging or dropping context.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn multi_turn_greeting_then_question() {
    dotenvy::dotenv().ok();
    env_logger::try_init().ok();
    ensure_workspace_root();

    let config = test_config("coding-agent");
    let mut engine = StarkbotEngine::new(config).expect("Failed to create engine");
    let mut handle = engine.start().await.expect("Failed to start engine");

    let timeout = Duration::from_secs(120);

    // Turn 1: Simple greeting
    handle.commands.send(FrontendCommand::SendMessage {
        content: "hi".into(),
    }).expect("Failed to send message");

    let events1 = collect_until_turn_complete(&mut handle.events, timeout).await;
    let answer1 = extract_answer(&events1);
    assert!(!answer1.is_empty(), "Agent should respond to greeting");
    eprintln!("Turn 1 (greeting): {}", &answer1[..answer1.len().min(100)]);

    // Turn 2: Ask about a topic that triggers tool use (e.g. reading a file)
    handle.commands.send(FrontendCommand::SendMessage {
        content: "Can you read the file Cargo.toml and tell me what workspace members are defined?".into(),
    }).expect("Failed to send message");

    let events2 = collect_until_turn_complete(&mut handle.events, timeout).await;
    let answer2 = extract_answer(&events2);
    assert!(!answer2.is_empty(), "Agent should produce a substantive answer after tool use");

    // Verify tool was actually called
    let tools_called: Vec<&str> = events2.iter()
        .filter_map(|e| if let BackendEvent::ToolCall { name, .. } = e { Some(name.as_str()) } else { None })
        .collect();
    eprintln!("Turn 2 tools: {:?}", tools_called);
    eprintln!("Turn 2 answer: {}", &answer2[..answer2.len().min(200)]);

    assert!(
        !tools_called.is_empty(),
        "Agent should use at least one tool to answer about Cargo.toml"
    );

    // Shutdown
    handle.commands.send(FrontendCommand::Shutdown).expect("Failed to send shutdown");
}

/// Reproduces the exact TUI flow with starkbot persona: auto_approve=true,
/// greeting → skill-related question → load_skill → sub_agent → answer.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn multi_turn_starkbot_flow() {
    dotenvy::dotenv().ok();
    env_logger::try_init().ok();
    ensure_workspace_root();

    let config = test_config("starkbot");
    let mut engine = StarkbotEngine::new(config).expect("Failed to create engine");
    let mut handle = engine.start().await.expect("Failed to start engine");

    let timeout = Duration::from_secs(120);

    // Turn 1: Simple greeting
    eprintln!("--- Turn 1: sending 'hi' ---");
    handle.commands.send(FrontendCommand::SendMessage {
        content: "hi".into(),
    }).expect("Failed to send message");

    let events1 = collect_until_turn_complete(&mut handle.events, timeout).await;
    let answer1 = extract_answer(&events1);
    eprintln!("Turn 1 answer: {}", &answer1[..answer1.len().min(100)]);
    assert!(!answer1.is_empty(), "Agent should respond to greeting");

    // Turn 2: Ask about spice framework — should trigger load_skill then sub_agent
    eprintln!("--- Turn 2: sending skill question ---");
    handle.commands.send(FrontendCommand::SendMessage {
        content: "tell me about the spice agent test framework".into(),
    }).expect("Failed to send message");

    let events2 = collect_until_turn_complete(&mut handle.events, timeout).await;
    let answer2 = extract_answer(&events2);

    let tools_called: Vec<&str> = events2.iter()
        .filter_map(|e| if let BackendEvent::ToolCall { name, .. } = e { Some(name.as_str()) } else { None })
        .collect();
    eprintln!("Turn 2 tools: {:?}", tools_called);
    eprintln!("Turn 2 answer: {}", &answer2[..answer2.len().min(300)]);

    assert!(!answer2.is_empty(), "Agent should produce answer after load_skill");

    // Shutdown
    handle.commands.send(FrontendCommand::Shutdown).expect("Failed to send shutdown");
}

/// Same as above but with auto_approve=false, which requires the test to
/// respond to ApprovalRequired events. This matches real TUI behavior.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn multi_turn_starkbot_interactive_approval() {
    dotenvy::dotenv().ok();
    env_logger::try_init().ok();
    ensure_workspace_root();

    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set");
    let model_name = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5.4".to_string());

    let config = BackendConfig {
        persona_slug: "starkbot".to_string(),
        api_key,
        model_name,
        auto_approve: false,
    };

    let mut engine = StarkbotEngine::new(config).expect("Failed to create engine");
    let mut handle = engine.start().await.expect("Failed to start engine");

    let timeout = Duration::from_secs(120);

    // Turn 1: greeting
    handle.commands.send(FrontendCommand::SendMessage {
        content: "hi".into(),
    }).expect("Failed to send message");

    let events1 = collect_until_turn_complete(&mut handle.events, timeout).await;
    let answer1 = extract_answer(&events1);
    assert!(!answer1.is_empty(), "Agent should respond to greeting");

    // Turn 2: Ask about spice — this will trigger load_skill (auto-approved),
    // then sub_agent (needs manual approval). We auto-approve all requests.
    handle.commands.send(FrontendCommand::SendMessage {
        content: "tell me about the spice agent test framework".into(),
    }).expect("Failed to send message");

    // Drain events, auto-approving any ApprovalRequired requests
    let events2 = collect_until_turn_complete_with_auto_approve(
        &mut handle.events,
        &handle.commands,
        timeout,
    ).await;

    let answer2 = extract_answer(&events2);
    let tools_called: Vec<&str> = events2.iter()
        .filter_map(|e| if let BackendEvent::ToolCall { name, .. } = e { Some(name.as_str()) } else { None })
        .collect();
    eprintln!("Turn 2 tools: {:?}", tools_called);
    eprintln!("Turn 2 answer: {}", &answer2[..answer2.len().min(300)]);

    assert!(!answer2.is_empty(), "Agent should produce answer");

    handle.commands.send(FrontendCommand::Shutdown).expect("Failed to send shutdown");
}
