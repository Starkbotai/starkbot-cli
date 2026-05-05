# 01 - TUI Architecture

## Framework

- **ratatui** (0.28+) for terminal rendering
- **crossterm** as the backend
- **tokio** async runtime for non-blocking agent execution
- Event loop: crossterm events + async agent messages via `tokio::sync::mpsc`

## Application Shell

```
┌─ StarkBot CLI ──────────────────────────────────────────┐
│ [Tab: Chat] [Tab: Skills] [Tab: Graph] [Tab: Memory]    │
├─────────────────────────────────────────────────────────┤
│                                                          │
│                    Active View                            │
│                                                          │
│                                                          │
├─────────────────────────────────────────────────────────┤
│ Status: Agent thinking... | Persona: Director | Tools: 5 │
└─────────────────────────────────────────────────────────┘
```

## Views

### 1. Chat View (Primary)

```
┌─ Chat ──────────────────────────────────────────────────┐
│ ┌─ Messages ──────────────────────────────────────────┐ │
│ │ [user] debug the auth module                        │ │
│ │ [agent] I'll investigate the auth module...         │ │
│ │ [tool] read_file("src/auth.rs") ✓                   │ │
│ │ [tool] grep("login", "src/") ✓                      │ │
│ │ [agent] Found the issue. The session token...       │ │
│ └─────────────────────────────────────────────────────┘ │
│ ┌─ Input ─────────────────────────────────────────────┐ │
│ │ > _                                                  │ │
│ └─────────────────────────────────────────────────────┘ │
│ ┌─ Tool Activity ─────────────────────────────────────┐ │
│ │ ▶ read_file  ✓ grep  ▶ edit_file (running...)       │ │
│ └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

**Features:**
- Scrollable message history with syntax highlighting
- Real-time streaming of agent responses (token-by-token)
- Inline tool call display with status indicators
- Input line with history (up/down arrows)
- Slash commands: `/persona`, `/clear`, `/skills`, `/tokens`, `/save`

### 2. Skills Browser View

```
┌─ Skills ────────────────────────────────────────────────┐
│ ┌─ List ──────────┐ ┌─ Detail ───────────────────────┐ │
│ │ ● planning      │ │ # Planning Skill               │ │
│ │ ● debugging     │ │ Version: 2.1.0                 │ │
│ │ ● code-review   │ │ Tags: [workflow, methodology]  │ │
│ │ ○ github        │ │ Tools: [read_file, write_file] │ │
│ │ ○ swap          │ │                                │ │
│ │ ● research      │ │ ## Description                 │ │
│ │                  │ │ A structured approach to...    │ │
│ └─────────────────┘ └────────────────────────────────┘ │
│ [e]nable/disable  [r]eload  [/]search                   │
└─────────────────────────────────────────────────────────┘
```

**Features:**
- List all available skills with enabled/disabled state
- Skill detail panel with rendered markdown preview
- Enable/disable skills for current persona
- Hot-reload skills from disk
- Search/filter skills

### 3. Graph Viewer (graf-rs)

```
┌─ Skill Graph ───────────────────────────────────────────┐
│                                                          │
│         [planning]──────[debugging]                      │
│              │               │                           │
│              ▼               ▼                           │
│      [code-review]     [testing]                        │
│              │               │                           │
│              └───────┬───────┘                           │
│                      ▼                                   │
│               [deployment]                               │
│                                                          │
│ Nodes: 12 | Edges: 18 | Layout: force-directed          │
│ [z]oom [p]an [f]ocus [l]ayout [enter]inspect            │
└─────────────────────────────────────────────────────────┘
```

**Features:**
- Interactive graph rendering in terminal (via graf-rs)
- Skill dependency/relationship visualization
- Knowledge graph (impulse map) viewing
- Node inspection on focus/enter
- Multiple layout algorithms (force-directed, hierarchical, radial)
- Zoom/pan navigation

### 4. Memory Browser

```
┌─ Memory ────────────────────────────────────────────────┐
│ ┌─ Search ────────────────────────────────────────────┐ │
│ │ > auth token refresh                                 │ │
│ └─────────────────────────────────────────────────────┘ │
│ ┌─ Results (3 tier: FTS + Vector + Graph) ────────────┐ │
│ │ [0.92] The auth module uses JWT with 24h expiry     │ │
│ │ [0.87] Token refresh endpoint at /api/refresh       │ │
│ │ [0.81] Session table stores refresh tokens          │ │
│ │ [0.74] Related: "login flow" → "token lifecycle"    │ │
│ └─────────────────────────────────────────────────────┘ │
│ ┌─ Graph View ────────────────────────────────────────┐ │
│ │ [auth] ──RelatedTo── [sessions] ──PartOf── [user]   │ │
│ └─────────────────────────────────────────────────────┘ │
│ [a]dd [d]elete [g]raph [t]ype filter                    │
└─────────────────────────────────────────────────────────┘
```

**Features:**
- Hybrid search across all three memory tiers
- Results ranked by relevance score (RRF fusion)
- Graph association viewer (inline mini-graph via graf-rs)
- Memory management (add, delete, tag, categorize)
- Type filters (fact, preference, entity, observation, task)

## Navigation

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle between views |
| `1-4` | Jump to specific view |
| `Ctrl+C` | Quit |
| `Esc` | Back / cancel current action |
| `:` | Command mode |
| `/` | Search within current view |
| `?` | Help overlay |

## Event Architecture

```rust
enum AppEvent {
    // Input
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),

    // Agent
    AgentStreaming(String),         // Token from LLM
    AgentToolCall(ToolCallEvent),   // Tool invocation
    AgentToolResult(ToolResult),    // Tool completed
    AgentDone(String),             // Final answer
    AgentError(String),            // Error occurred

    // System
    Tick,                          // UI refresh (60fps)
    SchedulerEvent(SchedulerMsg),  // Cron/heartbeat
    MemoryUpdate(MemoryEvent),     // Memory changed
}
```

## Rendering Pipeline

1. `crossterm` captures terminal events
2. Events dispatched to active view handler
3. Agent runs on background tokio task, sends events via `mpsc`
4. Main loop merges input events + agent events
5. `ratatui` renders frame on each tick or event
6. graf-rs renders graph widgets inline within ratatui frames

## Responsive Layout

- Minimum terminal size: 80x24
- Panels auto-resize based on terminal dimensions
- Collapsible sidebars on narrow terminals
- Full-width mode for graph viewer
