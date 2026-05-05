# 02 - Metalcraft Integration

## Overview

Metalcraft is the agentic engine powering all agent execution in StarkBot CLI. It provides:
- Stateful graph orchestration (LangGraph-style)
- ReAct agent loops (think → act → observe)
- Tool calling with approval hooks
- Checkpointing for resumable workflows
- Step guards for safety
- Parallel execution and sub-graphs

## Core Agent Graph

The primary agent uses metalcraft's `create_react_agent_with_hooks`:

```rust
use metalcraft::{
    create_react_agent_with_hooks, AgentState, Executor,
    ToolRegistry, BeforeToolCallHook, CompiledGraph,
};

// Build the main agent graph
let graph = create_react_agent_with_hooks(
    model,
    registry,
    &system_prompt,
    Some(approval_hook),
)?;

let executor = Executor::new(graph)
    .max_steps(100)
    .with_step_guard(safety_guard)
    .with_checkpointer(sqlite_checkpointer);
```

## Graph Topology

### Simple Agent (Single Persona)

```
┌─────────┐     ┌───────┐
│  Agent  │────▶│ Tools │
│  (LLM)  │◀───│       │
└────┬────┘     └───────┘
     │
     ▼ (is_done)
   [END]
```

### Director Agent (Multi-Agent)

```
┌──────────┐     ┌────────────┐     ┌───────────────┐
│ Director │────▶│  Router    │────▶│ Finance Agent │
│   Node   │     │ (conditnl) │     │  (subgraph)   │
└────┬─────┘     └─────┬──────┘     └───────────────┘
     │                  │
     │                  ├───────────▶┌───────────────┐
     │                  │            │  Code Agent   │
     │                  │            │  (subgraph)   │
     │                  │            └───────────────┘
     │                  │
     │                  └───────────▶┌───────────────┐
     │                               │Research Agent │
     ▼ (is_done)                     │  (subgraph)   │
   [END]                             └───────────────┘
```

## State Model

```rust
use metalcraft::Reducer;

#[derive(Clone)]
pub struct StarkState {
    pub messages: Vec<AgentMessage>,
    pub pending_tool_calls: Vec<PendingToolCall>,
    pub is_done: bool,
    // StarkBot extensions
    pub active_persona: String,
    pub active_skills: Vec<String>,
    pub memory_context: Vec<MemoryEntry>,
    pub task_plan: Option<TaskPlan>,
    pub sub_agent_results: Vec<SubAgentResult>,
}

pub enum StarkUpdate {
    // Metalcraft standard
    ToolCalls(Vec<PendingToolCall>),
    FinalAnswer(String),
    ToolResults(Vec<ToolResult>),
    // StarkBot extensions
    PersonaSwitch(String),
    SkillActivated(String),
    MemoryRecalled(Vec<MemoryEntry>),
    TaskDefined(TaskPlan),
    SubAgentCompleted(SubAgentResult),
}

impl Reducer for StarkState {
    type Update = StarkUpdate;
    fn apply(&mut self, update: Self::Update) {
        match update {
            StarkUpdate::ToolCalls(calls) => {
                self.pending_tool_calls = calls;
                self.is_done = false;
            }
            StarkUpdate::FinalAnswer(text) => {
                self.messages.push(AgentMessage::Assistant(text));
                self.is_done = true;
            }
            // ... other variants
        }
    }
}
```

## Execution Flow

### Per-Message Execution

```
1. User input received from TUI
2. Memory retrieval (hybrid search for context)
3. Build AgentState with message + memory context
4. Executor.run(state, thread_id)
5. Each step:
   a. LLM called with system prompt + tools + history
   b. If tool calls → approval hook → execute → loop
   c. If final answer → return to TUI
   d. Step guard checks for safety
6. Stream events to TUI via mpsc channel
7. Checkpoint state for resumability
```

### Streaming to TUI

```rust
// In the TUI event loop
let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();

// Agent task streams events
tokio::spawn(async move {
    let mut stream = executor.stream(state, &thread_id);
    while let Some(Ok((event, state))) = stream.next().await {
        tx.send(AppEvent::AgentStep(event, state)).ok();
    }
});
```

## Approval Hooks

```rust
pub fn build_approval_hook(
    tx: UnboundedSender<AppEvent>,
    rx: tokio::sync::oneshot::Receiver<bool>,
) -> BeforeToolCallHook {
    Arc::new(move |tool_name, args| {
        match tool_name {
            // Auto-approve read-only tools
            "read_file" | "grep" | "find_files" | "list_files" | "load_skill" => {
                BeforeToolCallAction::Proceed
            }
            // Send approval request to TUI
            _ => {
                tx.send(AppEvent::ApprovalRequired(tool_name.into(), args.clone()));
                match rx.blocking_recv() {
                    Ok(true) => BeforeToolCallAction::Proceed,
                    _ => BeforeToolCallAction::Deny("User denied".into()),
                }
            }
        }
    })
}
```

## Step Guards

```rust
pub fn build_safety_guard() -> StepGuard<StarkState> {
    let state = Arc::new(Mutex::new(GuardState::default()));

    Arc::new(move |agent_state, event| {
        let mut guard = state.lock().unwrap();

        // Detect error spirals (3+ consecutive failures)
        if guard.consecutive_errors >= 3 {
            return GuardAction::Stop("Error spiral detected".into());
        }

        // Detect loops (same tool call repeated)
        if guard.detect_loop(&event) {
            return GuardAction::Stop("Loop detected".into());
        }

        // Enforce max cost budget
        if guard.total_tokens > MAX_TOKEN_BUDGET {
            return GuardAction::Stop("Token budget exceeded".into());
        }

        GuardAction::Continue
    })
}
```

## Checkpointing

```rust
use metalcraft::Checkpointer;

pub struct SqliteCheckpointer {
    db: Arc<SqlitePool>,
}

#[async_trait]
impl Checkpointer<StarkState> for SqliteCheckpointer {
    async fn save(&self, thread_id: &str, state: &StarkState, next_node: &str) -> Result<()> {
        let serialized = serde_json::to_vec(state)?;
        sqlx::query("INSERT OR REPLACE INTO checkpoints (thread_id, state, next_node, updated_at) VALUES (?, ?, ?, ?)")
            .bind(thread_id)
            .bind(&serialized)
            .bind(next_node)
            .bind(chrono::Utc::now())
            .execute(&*self.db)
            .await?;
        Ok(())
    }

    async fn load(&self, thread_id: &str) -> Result<Option<(StarkState, String)>> {
        // Load from SQLite
    }
}
```

## Sub-Agent Spawning

Using metalcraft's `SubgraphNode` for multi-agent:

```rust
use metalcraft::SubgraphNode;

let code_agent_subgraph = SubgraphNode::new(
    code_agent_graph.into_arc(),
    // Extract: outer state → inner state
    |outer: &StarkState| AgentState::new(&outer.current_task_description()),
    // Merge: inner final state → outer update
    |inner: AgentState| StarkUpdate::SubAgentCompleted(SubAgentResult {
        agent: "code".into(),
        answer: inner.final_answer().unwrap_or_default(),
    }),
    50, // max steps for subgraph
);
```

## Multi-Turn Conversations

```rust
// First message
let state = StarkState::new("debug the auth module");
let outcome = executor.run(state, "session-1").await?;

// Follow-up
if let RunOutcome::Completed(mut state) = outcome {
    state.continue_with("now write tests for the fix");
    let outcome = executor.run(state, "session-1").await?;
}
```

## Human-in-the-Loop (Interrupts)

```rust
// Agent node can interrupt for approval
NodeOutcome::Interrupt {
    update: StarkUpdate::TaskDefined(plan),
    reason: "Task plan requires approval before execution".into(),
}

// TUI shows plan, user approves
// Resume with injected update
let outcome = executor.resume("session-1", Some(StarkUpdate::PlanApproved)).await?;
```
