use metalcraft::BeforeToolCallAction;
use std::collections::HashSet;
use std::io::{self, Write};
use std::sync::Arc;

/// An approval request sent to the TUI for user decision.
#[derive(Clone, Debug)]
pub struct ApprovalRequest {
    pub tool_name: String,
    pub args_display: String,
}

/// Channel-based approval sender (used in the hook closure).
pub type ApprovalSender = std::sync::mpsc::SyncSender<(ApprovalRequest, std::sync::mpsc::SyncSender<bool>)>;

/// Channel-based approval receiver (polled by the TUI event loop).
pub type ApprovalReceiver = std::sync::mpsc::Receiver<(ApprovalRequest, std::sync::mpsc::SyncSender<bool>)>;

/// Create a channel pair for TUI-based approval.
pub fn approval_channel() -> (ApprovalSender, ApprovalReceiver) {
    // Bounded channel with capacity 1 — hook blocks until TUI responds
    std::sync::mpsc::sync_channel(1)
}

/// Build a `BeforeToolCallHook` for metalcraft based on approval mode.
pub fn build_hook(mode: ApprovalMode) -> Option<metalcraft::BeforeToolCallHook> {
    match mode {
        ApprovalMode::AutoApprove => None,
        ApprovalMode::Interactive { auto_approve_tools, channel } => {
            Some(Arc::new(move |name: &str, args: &serde_json::Value| {
                if auto_approve_tools.contains(name) {
                    return BeforeToolCallAction::Proceed;
                }
                match &channel {
                    Some(tx) => prompt_via_channel(tx, name, args),
                    None => prompt_user_cli(name, args),
                }
            }))
        }
    }
}

#[derive(Clone)]
pub enum ApprovalMode {
    AutoApprove,
    Interactive {
        auto_approve_tools: HashSet<String>,
        /// If Some, approvals route through this channel (TUI mode).
        /// If None, approvals use direct stdin/stderr (CLI oneshot mode).
        channel: Option<ApprovalSender>,
    },
}

impl ApprovalMode {
    /// Interactive mode for CLI (oneshot) — prompts directly on stdin/stderr.
    pub fn default_interactive() -> Self {
        let auto = ["read_file", "list_files", "grep", "find_files", "load_skill", "api_keys_check"]
            .into_iter()
            .map(String::from)
            .collect();
        Self::Interactive { auto_approve_tools: auto, channel: None }
    }

    /// Interactive mode for TUI — routes approvals through a channel.
    pub fn tui_interactive(tx: ApprovalSender) -> Self {
        let auto = ["read_file", "list_files", "grep", "find_files", "load_skill", "api_keys_check"]
            .into_iter()
            .map(String::from)
            .collect();
        Self::Interactive { auto_approve_tools: auto, channel: Some(tx) }
    }
}

fn format_args_display(tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "bash" => {
            let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("(no command)");
            format!("command: {}", cmd)
        }
        "write_file" | "edit_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            format!("path: {}", path)
        }
        "sub_agent" => {
            let task = args.get("task").and_then(|v| v.as_str()).unwrap_or("?");
            let tool_set = args.get("tool_set").and_then(|v| v.as_str()).unwrap_or("?");
            format!("task: \"{}\"\ntool_set: {}", task, tool_set)
        }
        _ => {
            if let Some(obj) = args.as_object() {
                obj.iter()
                    .map(|(k, v)| {
                        let val = match v {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        format!("{}: {}", k, val)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                serde_json::to_string(args).unwrap_or_default()
            }
        }
    }
}

/// Prompt via channel (TUI mode) — blocks until TUI responds.
fn prompt_via_channel(tx: &ApprovalSender, tool_name: &str, args: &serde_json::Value) -> BeforeToolCallAction {
    let request = ApprovalRequest {
        tool_name: tool_name.to_string(),
        args_display: format_args_display(tool_name, args),
    };

    // Create a one-shot response channel
    let (resp_tx, resp_rx) = std::sync::mpsc::sync_channel(1);

    // Send request to TUI — if the TUI is gone, deny
    if tx.send((request, resp_tx)).is_err() {
        return BeforeToolCallAction::Deny("Approval channel closed".into());
    }

    // Block waiting for TUI response
    match resp_rx.recv() {
        Ok(true) => BeforeToolCallAction::Proceed,
        Ok(false) => BeforeToolCallAction::Deny(format!("User denied tool '{tool_name}'")),
        Err(_) => BeforeToolCallAction::Deny("Approval response channel closed".into()),
    }
}

/// Prompt via CLI (oneshot mode) — reads from stdin.
fn prompt_user_cli(tool_name: &str, args: &serde_json::Value) -> BeforeToolCallAction {
    let args_display = format_args_display(tool_name, args);

    eprintln!();
    eprintln!("  \x1b[33m⚡ {}\x1b[0m {}", tool_name, args_display);
    eprint!("  Approve? [Y/n] ");
    io::stderr().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return BeforeToolCallAction::Deny("Failed to read input".into());
    }

    let answer = input.trim().to_lowercase();
    if matches!(answer.as_str(), "" | "y" | "yes") {
        BeforeToolCallAction::Proceed
    } else {
        BeforeToolCallAction::Deny(format!("User denied tool '{tool_name}'"))
    }
}
