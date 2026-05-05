# 06 - Multi-Agent Orchestration

## Overview

StarkBot CLI supports hierarchical multi-agent execution, mirroring StarkBot's Director pattern. A Director agent can delegate to specialized sub-agents, each with their own tool sets and personas.

## Agent Hierarchy

```
┌─────────────────────────────────────────────┐
│              Director Agent                   │
│  (Orchestrates, delegates, synthesizes)       │
├─────────────────────────────────────────────┤
│                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │  Code    │  │ Research │  │  DevOps  │  │
│  │  Agent   │  │  Agent   │  │  Agent   │  │
│  └──────────┘  └──────────┘  └──────────┘  │
│                                              │
│  ┌──────────┐  ┌──────────┐                 │
│  │ Finance  │  │ Creative │                 │
│  │  Agent   │  │  Agent   │                 │
│  └──────────┘  └──────────┘                 │
│                                              │
└─────────────────────────────────────────────┘
```

## Persona Definitions

```json
// personas/director.json
{
  "name": "Director",
  "description": "Orchestrator that delegates to specialized agents",
  "tools": ["sub_agent", "set_persona", "define_tasks", "memory_search", "load_skill"],
  "skills": ["planning", "research-methodology"],
  "system_prompt": "You are the Director agent. Your role is to analyze requests and delegate to specialized sub-agents when appropriate. For simple tasks, handle them directly. For complex multi-domain tasks, spawn sub-agents in parallel."
}
```

```json
// personas/coding-agent.json
{
  "name": "Code Engineer",
  "description": "Expert coding assistant for reading, writing, and debugging code",
  "tools": ["read_file", "write_file", "edit_file", "bash", "grep", "glob", "find_files", "list_files", "load_skill", "memory_search"],
  "skills": ["debugging", "code-review", "planning", "explore-codebase", "commit-message", "testing"],
  "system_prompt": "You are an expert code engineer..."
}
```

```json
// personas/research-agent.json
{
  "name": "Research Agent",
  "description": "Information gathering and analysis (read-only)",
  "tools": ["read_file", "grep", "glob", "find_files", "list_files", "web_fetch", "load_skill", "memory_search", "memory_store"],
  "skills": ["research-methodology", "summarize", "explore-codebase"],
  "system_prompt": "You are a research specialist. You gather information, analyze patterns, and synthesize findings. You do NOT modify files."
}
```

## Execution Modes

### Mode 1: Direct (Single Agent)

User's message goes directly to the active persona's agent loop.

```
User → [Coding Agent] → Response
```

### Mode 2: Delegated (Director → Sub-Agent)

Director analyzes the request and spawns appropriate sub-agent(s).

```
User → [Director] → spawns → [Code Agent] → result → [Director] → Response
```

### Mode 3: Parallel (Director → Multiple Sub-Agents)

For multi-domain tasks, Director spawns agents in parallel.

```
User → [Director] → spawns → [Code Agent]     ─┐
                            → [Research Agent]  ─┼→ [Director] → Response
                            → [DevOps Agent]   ─┘
```

## Sub-Agent Tool

```rust
pub struct SubAgentTool {
    config: ToolConfig,
}

#[async_trait]
impl Tool for SubAgentTool {
    fn name(&self) -> &str { "sub_agent" }

    fn description(&self) -> &str {
        "Spawn a specialized sub-agent to handle a specific task. \
         The sub-agent runs with its own persona and tool set."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "persona": {
                    "type": "string",
                    "description": "The persona to use (coding-agent, research-agent, devops-agent)"
                },
                "task": {
                    "type": "string",
                    "description": "The task to delegate to the sub-agent"
                },
                "max_steps": {
                    "type": "integer",
                    "description": "Maximum steps for the sub-agent (default: 30)"
                }
            },
            "required": ["persona", "task"]
        })
    }

    async fn call(&self, args: Value) -> Result<Value> {
        let persona_name = args["persona"].as_str().unwrap();
        let task = args["task"].as_str().unwrap();
        let max_steps = args.get("max_steps").and_then(|v| v.as_u64()).unwrap_or(30) as usize;

        // Load persona
        let persona = load_persona(persona_name)?;
        let registry = build_registry(&persona, &self.config);

        // Build sub-agent graph
        let graph = create_react_agent_with_hooks(
            self.config.model.clone(),
            registry,
            &persona.build_system_prompt(),
            None, // Sub-agents auto-approve (controlled by parent)
        )?;

        let executor = Executor::new(graph).max_steps(max_steps);
        let state = AgentState::new(task);

        match executor.run(state, &format!("sub-{}", uuid::Uuid::new_v4())).await? {
            RunOutcome::Completed(state) => {
                Ok(json!({
                    "result": state.final_answer().unwrap_or_default(),
                    "tools_used": state.tools_called(),
                    "turns": state.messages.len(),
                }))
            }
            RunOutcome::Interrupted { state, reason, .. } => {
                Ok(json!({
                    "result": state.final_answer().unwrap_or_default(),
                    "interrupted": true,
                    "reason": reason,
                }))
            }
        }
    }
}
```

## Metalcraft SubgraphNode Pattern

For tighter integration (shared state), use metalcraft's SubgraphNode:

```rust
use metalcraft::SubgraphNode;

// The Director graph with embedded sub-agent subgraphs
let director_graph = Graph::<StarkState>::new()
    .add_node("director", DirectorNode::new(model, director_registry))
    .add_node("code_subagent", SubgraphNode::new(
        code_agent_graph.into_arc(),
        |outer: &StarkState| AgentState::new(&outer.pending_delegation_task()),
        |inner: AgentState| StarkUpdate::SubAgentCompleted(SubAgentResult {
            agent: "code".into(),
            answer: inner.final_answer().unwrap_or_default(),
        }),
        50,
    ))
    .add_node("research_subagent", SubgraphNode::new(
        research_graph.into_arc(),
        |outer: &StarkState| AgentState::new(&outer.pending_delegation_task()),
        |inner: AgentState| StarkUpdate::SubAgentCompleted(SubAgentResult {
            agent: "research".into(),
            answer: inner.final_answer().unwrap_or_default(),
        }),
        30,
    ))
    .add_conditional("director", |state| {
        match state.delegation_target.as_deref() {
            Some("code") => "code_subagent".to_string(),
            Some("research") => "research_subagent".to_string(),
            None if state.is_done => END.to_string(),
            _ => "tools".to_string(),
        }
    })
    .add_edge("code_subagent", "director")
    .add_edge("research_subagent", "director")
    .set_entry("director")
    .compile()?;
```

## Persona Switching (In-Session)

Users can switch personas mid-session:

```
[director]> /persona set coding-agent
Switched to: Code Engineer
[coding-agent]> fix the login bug
```

The `set_persona` tool allows the agent itself to switch:
```rust
// Agent decides it needs a different toolset
tool_call: set_persona({ "persona": "devops-agent" })
// Graph is rebuilt with new tools, conversation continues
```

## TUI Integration

The TUI shows sub-agent activity:

```
┌─ Chat ────────────────────────────────────────────────┐
│ [user] deploy the app and update the docs             │
│ [director] I'll delegate this to two sub-agents...    │
│ ┌─ Sub-Agent: devops ──────────────────────────────┐  │
│ │ ▶ bash("docker build...") ✓                      │  │
│ │ ▶ bash("docker push...") ✓                       │  │
│ │ Result: Deployed to production                    │  │
│ └──────────────────────────────────────────────────┘  │
│ ┌─ Sub-Agent: code ───────────────────────────────┐   │
│ │ ▶ read_file("README.md") ✓                      │   │
│ │ ▶ edit_file("README.md") ✓                      │   │
│ │ Result: Updated deployment section               │   │
│ └──────────────────────────────────────────────────┘  │
│ [director] Both tasks completed successfully.         │
└───────────────────────────────────────────────────────┘
```

## Isolation & Safety

- Each sub-agent has its own step guard (error spiral detection)
- Sub-agents cannot spawn further sub-agents (max depth = 1)
- Tool approval for sub-agents: auto-approve unless `AlwaysAsk` level
- Sub-agent timeout: configurable per invocation (default 5 minutes)
- Token budget: sub-agents share parent's budget, guard enforces limits
