# 04 - Tool System

## Overview

Tools are async functions the agent can invoke during execution. They implement metalcraft's `Tool` trait and are registered in a `ToolRegistry` per persona.

## Tool Categories

### File Operations

| Tool | Description | Risk Level |
|------|-------------|------------|
| `read_file` | Read file contents with line ranges | Read-only |
| `write_file` | Create/overwrite files | Write |
| `edit_file` | Find-and-replace editing | Write |
| `list_files` | List directory (max 3 levels) | Read-only |
| `glob` | Find files by pattern | Read-only |
| `grep` | Search file contents | Read-only |
| `find_files` | Find by filename pattern | Read-only |

### Execution

| Tool | Description | Risk Level |
|------|-------------|------------|
| `bash` | Execute shell commands | Execute |
| `sub_agent` | Spawn child agent | Agent |

### Knowledge

| Tool | Description | Risk Level |
|------|-------------|------------|
| `load_skill` | Load methodology guide | Read-only |
| `web_fetch` | Fetch URL as markdown | Network |
| `memory_store` | Store to memory system | Write |
| `memory_search` | Search memories (hybrid) | Read-only |
| `memory_graph` | Query memory associations | Read-only |

### System

| Tool | Description | Risk Level |
|------|-------------|------------|
| `define_tasks` | Create task plan | Write |
| `notes` | Read/write persistent notes | Write |
| `set_persona` | Switch agent persona | System |

## Tool Implementation Pattern

```rust
use metalcraft::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str { "read_file" }

    fn description(&self) -> &str {
        "Read file contents. Supports line ranges for large files."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute or relative file path"
                },
                "start_line": {
                    "type": "integer",
                    "description": "Start line (1-indexed, optional)"
                },
                "end_line": {
                    "type": "integer",
                    "description": "End line (inclusive, optional)"
                }
            },
            "required": ["path"]
        })
    }

    async fn call(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().ok_or("missing path")?;
        let start = args.get("start_line").and_then(|v| v.as_u64()).map(|v| v as usize);
        let end = args.get("end_line").and_then(|v| v.as_u64()).map(|v| v as usize);

        let content = tokio::fs::read_to_string(path).await?;
        let lines: Vec<&str> = content.lines().collect();

        let (start, end) = match (start, end) {
            (Some(s), Some(e)) => (s.saturating_sub(1), e.min(lines.len())),
            (Some(s), None) => (s.saturating_sub(1), lines.len()),
            _ => (0, lines.len()),
        };

        let selected: String = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:4}\t{}", start + i + 1, line))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(json!({ "content": selected, "total_lines": lines.len() }))
    }
}
```

## Tool Registry Building

```rust
pub fn build_registry(persona: &Persona, config: &ToolConfig) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    for tool_name in &persona.tools {
        match tool_name.as_str() {
            "read_file" => registry.register(ReadFileTool),
            "write_file" => registry.register(WriteFileTool),
            "edit_file" => registry.register(EditFileTool),
            "bash" => registry.register(BashTool::new(config.timeout)),
            "grep" => registry.register(GrepTool),
            "glob" => registry.register(GlobTool),
            "find_files" => registry.register(FindFilesTool),
            "list_files" => registry.register(ListFilesTool),
            "load_skill" => registry.register(LoadSkillTool::new(config.skill_registry.clone())),
            "sub_agent" => registry.register(SubAgentTool::new(config.clone())),
            "web_fetch" => registry.register(WebFetchTool),
            "memory_store" => registry.register(MemoryStoreTool::new(config.memory.clone())),
            "memory_search" => registry.register(MemorySearchTool::new(config.memory.clone())),
            "memory_graph" => registry.register(MemoryGraphTool::new(config.memory.clone())),
            "define_tasks" => registry.register(DefineTasksTool),
            "notes" => registry.register(NotesTool::new(config.notes_dir.clone())),
            "set_persona" => registry.register(SetPersonaTool),
            _ => log::warn!("Unknown tool: {}", tool_name),
        }
    }

    registry
}
```

## Output Limits

To prevent context window overflow:

| Tool | Max Output |
|------|------------|
| `bash` | 30KB stdout + 10KB stderr |
| `read_file` | 2000 lines |
| `grep` | 100 matches |
| `find_files` | 200 results |
| `list_files` | 3 levels depth |
| `web_fetch` | 50KB content |

## Approval Levels

Tools are categorized by risk for the approval hook:

```rust
pub enum ApprovalLevel {
    AutoApprove,   // read_file, grep, find_files, list_files, load_skill, memory_search
    PromptUser,    // write_file, edit_file, bash, web_fetch, memory_store
    AlwaysAsk,     // sub_agent, set_persona, define_tasks (modifies execution)
}

pub fn approval_level(tool_name: &str) -> ApprovalLevel {
    match tool_name {
        "read_file" | "grep" | "glob" | "find_files" | "list_files"
        | "load_skill" | "memory_search" | "memory_graph" => ApprovalLevel::AutoApprove,

        "sub_agent" | "set_persona" => ApprovalLevel::AlwaysAsk,

        _ => ApprovalLevel::PromptUser,
    }
}
```

## Tool Presets (from StarkBot)

Pre-configured tool call templates for common operations:

```rust
pub struct ToolPreset {
    pub name: String,
    pub tool: String,
    pub description: String,
    pub args_template: Value,
}

// Example presets
let presets = vec![
    ToolPreset {
        name: "git_status".into(),
        tool: "bash".into(),
        description: "Check git status".into(),
        args_template: json!({ "command": "git status" }),
    },
    ToolPreset {
        name: "run_tests".into(),
        tool: "bash".into(),
        description: "Run project tests".into(),
        args_template: json!({ "command": "cargo test" }),
    },
];
```

## Event Emission

Tools emit events for TUI display:

```rust
// Before tool execution
tx.send(AppEvent::AgentToolCall(ToolCallEvent {
    tool_name: "bash".into(),
    args: json!({ "command": "cargo test" }),
    status: ToolStatus::Running,
}));

// After tool execution
tx.send(AppEvent::AgentToolResult(ToolResultEvent {
    tool_name: "bash".into(),
    success: true,
    output_preview: "All 42 tests passed".into(),
    duration: Duration::from_secs(3),
}));
```
