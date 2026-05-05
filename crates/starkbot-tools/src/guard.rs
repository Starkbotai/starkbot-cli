use metalcraft::{AgentMessage, AgentState, GuardAction, StepEvent, StepGuard};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct GuardConfig {
    pub max_consecutive_errors: usize,
    pub loop_window: usize,
    pub verbose: bool,
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self { max_consecutive_errors: 3, loop_window: 5, verbose: true }
    }
}

pub fn build_agent_guard(config: GuardConfig) -> StepGuard<AgentState> {
    let state_tracker = Arc::new(Mutex::new(GuardTracker::new(config)));
    Arc::new(move |state: &AgentState, _event: &StepEvent| {
        let mut tracker = state_tracker.lock().unwrap();
        tracker.check(state)
    })
}

struct GuardTracker {
    config: GuardConfig,
    seen_up_to: usize,
    consecutive_error_turns: usize,
    recent_calls: Vec<u64>,
}

impl GuardTracker {
    fn new(config: GuardConfig) -> Self {
        Self { config, seen_up_to: 0, consecutive_error_turns: 0, recent_calls: Vec::new() }
    }

    fn check(&mut self, state: &AgentState) -> GuardAction {
        let new_messages = &state.messages[self.seen_up_to..];
        self.seen_up_to = state.messages.len();

        let mut batch_results: Vec<bool> = Vec::new();
        let mut new_tool_calls: Vec<u64> = Vec::new();

        for msg in new_messages {
            match msg {
                AgentMessage::ToolCall { name, args, .. } => {
                    if self.config.verbose {
                        let args_brief = summarize_args(args);
                        eprintln!("  \x1b[36m▶ {name}\x1b[0m({args_brief})");
                    }
                    new_tool_calls.push(call_hash(name, args));
                }
                AgentMessage::ToolResult { name, result, .. } => {
                    let is_error = result.starts_with("ERROR:");
                    if self.config.verbose {
                        if is_error {
                            eprintln!("  \x1b[31m✗ {name}\x1b[0m: {}", truncate(result, 120));
                        } else {
                            eprintln!("  \x1b[32m✓ {name}\x1b[0m {}", truncate(result, 80));
                        }
                    }
                    batch_results.push(is_error);
                }
                _ => {}
            }
        }

        if self.config.max_consecutive_errors > 0 && !batch_results.is_empty() {
            if batch_results.iter().all(|&is_err| is_err) {
                self.consecutive_error_turns += 1;
            } else {
                self.consecutive_error_turns = 0;
            }
            if self.consecutive_error_turns >= self.config.max_consecutive_errors {
                return GuardAction::Stop(format!(
                    "Error spiral: {} consecutive turns with all tool calls failing",
                    self.consecutive_error_turns
                ));
            }
        }

        if self.config.loop_window > 0 {
            let existing: HashSet<u64> = self.recent_calls.iter().copied().collect();
            for &hash in &new_tool_calls {
                if existing.contains(&hash) {
                    return GuardAction::Stop("Loop detected: repeated identical tool call".into());
                }
            }
            self.recent_calls.extend(new_tool_calls);
            let window = self.config.loop_window;
            if self.recent_calls.len() > window {
                let drain = self.recent_calls.len() - window;
                self.recent_calls.drain(..drain);
            }
        }

        GuardAction::Continue
    }
}

fn call_hash(name: &str, args: &serde_json::Value) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    serde_json::to_string(args).unwrap_or_default().hash(&mut hasher);
    hasher.finish()
}

fn summarize_args(args: &serde_json::Value) -> String {
    if let Some(obj) = args.as_object() {
        obj.iter()
            .map(|(k, v)| {
                let val = match v {
                    serde_json::Value::String(s) => truncate(s, 60),
                    other => truncate(&other.to_string(), 60),
                };
                format!("{k}: {val}")
            })
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        truncate(&args.to_string(), 80)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max]) }
}
