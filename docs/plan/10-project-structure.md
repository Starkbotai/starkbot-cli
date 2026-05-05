# 10 - Project Structure

## Workspace Layout

```
starkbot-cli/
├── Cargo.toml                    # Workspace root
├── .env.example
├── .gitignore
├── docs/
│   └── plan/                     # This plan
│       ├── 00-overview.md
│       └── ...
│
├── crates/
│   ├── starkbot-app/             # Main binary crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs           # Entry point, CLI args, app bootstrap
│   │       ├── app.rs            # Application state, event loop
│   │       ├── events.rs         # Event types and dispatch
│   │       └── cli.rs            # Clap CLI argument parsing
│   │
│   ├── starkbot-tui/             # TUI rendering layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── views/
│   │       │   ├── mod.rs
│   │       │   ├── chat.rs       # Chat view
│   │       │   ├── skills.rs     # Skills browser
│   │       │   ├── graph.rs      # Graph viewer (graf-rs)
│   │       │   └── memory.rs     # Memory browser
│   │       ├── widgets/
│   │       │   ├── mod.rs
│   │       │   ├── message.rs    # Chat message widget
│   │       │   ├── tool_bar.rs   # Tool activity bar
│   │       │   ├── status.rs     # Status line
│   │       │   └── input.rs      # Input line widget
│   │       └── theme.rs          # Colors and styling
│   │
│   ├── starkbot-core/            # Core logic (agent, dispatch, session)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── dispatch.rs       # Message dispatch engine
│   │       ├── persona.rs        # Persona loading and management
│   │       ├── session.rs        # Session lifecycle
│   │       ├── context.rs        # Context compaction
│   │       └── scheduler.rs      # Cron/heartbeat scheduler
│   │
│   ├── starkbot-tools/           # Tool implementations
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs       # Tool registry builder
│   │       ├── approval.rs       # Approval hooks
│   │       ├── guard.rs          # Step guard
│   │       ├── bash.rs
│   │       ├── read_file.rs
│   │       ├── write_file.rs
│   │       ├── edit_file.rs
│   │       ├── grep.rs
│   │       ├── glob.rs
│   │       ├── find_files.rs
│   │       ├── list_files.rs
│   │       ├── load_skill.rs
│   │       ├── sub_agent.rs
│   │       ├── web_fetch.rs
│   │       ├── memory_store.rs
│   │       ├── memory_search.rs
│   │       ├── memory_graph.rs
│   │       ├── define_tasks.rs
│   │       ├── notes.rs
│   │       └── set_persona.rs
│   │
│   ├── starkbot-memory/          # Memory system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── store.rs          # Memory CRUD
│   │       ├── fts.rs            # Full-text search
│   │       ├── embedding.rs      # Vector embeddings
│   │       ├── graph.rs          # Association graph
│   │       ├── hybrid.rs         # RRF fusion search
│   │       └── decay.rs          # Memory decay
│   │
│   ├── starkbot-skills/          # Skill system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs       # Skill loading and management
│   │       ├── parser.rs         # YAML frontmatter + markdown parsing
│   │       ├── graph.rs          # Skill relationship graph
│   │       └── watcher.rs        # File watcher for hot-reload
│   │
│   └── starkbot-db/              # Database layer
│       ├── Cargo.toml
│       ├── migrations/
│       │   ├── 001_initial.sql
│       │   ├── 002_memory.sql
│       │   ├── 003_scheduling.sql
│       │   └── 004_tracking.sql
│       └── src/
│           ├── lib.rs
│           ├── pool.rs           # Connection pool
│           ├── sessions.rs       # Session queries
│           ├── messages.rs       # Message queries
│           ├── memories.rs       # Memory queries
│           ├── checkpoints.rs    # Checkpoint queries
│           └── scheduling.rs     # Cron job queries
│
├── config/                       # Default configuration
│   ├── bot_config.ron
│   ├── ai_endpoints.ron
│   └── agents/
│       ├── director.json
│       ├── coding-agent.json
│       ├── research-agent.json
│       └── devops-agent.json
│
├── skills/                       # Default skills
│   ├── planning.md
│   ├── debugging.md
│   ├── code-review.md
│   ├── testing.md
│   ├── explore-codebase.md
│   ├── commit-message.md
│   ├── research-methodology.md
│   └── summarize.md
│
├── tests/                        # Integration tests
│   ├── agent_spice_test.rs       # Spice agent tests
│   └── fixtures/                 # Test fixtures
│
└── graf-rs/                      # Graph rendering library (sub-crate or separate)
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── graph.rs              # GraphData, GraphNode, GraphEdge
        ├── layout/
        │   ├── mod.rs
        │   ├── force_directed.rs
        │   ├── hierarchical.rs
        │   ├── radial.rs
        │   └── circular.rs
        ├── render.rs             # Ratatui Widget impl
        ├── viewport.rs           # Pan, zoom, navigation
        └── style.rs              # Edge/node styling
```

## Dependencies

### Workspace Cargo.toml

```toml
[workspace]
members = [
    "crates/starkbot-app",
    "crates/starkbot-tui",
    "crates/starkbot-core",
    "crates/starkbot-tools",
    "crates/starkbot-memory",
    "crates/starkbot-skills",
    "crates/starkbot-db",
    "graf-rs",
]
resolver = "2"

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
futures = "0.3"

# Metalcraft (agentic engine)
metalcraft = { path = "../../rust4ai/metalcraft", features = ["rig"] }

# AI/LLM
rig-core = "0.36"

# TUI
ratatui = "0.28"
crossterm = "0.28"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
ron = "0.8"

# Utilities
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
dotenvy = "0.15"
dirs = "5"
clap = { version = "4", features = ["derive"] }

# File watching
notify = "6"

# Embeddings (choose one)
fastembed = "4"
# OR candle-core + candle-nn + candle-transformers

# Testing
spice = { path = "../../rust4ai/spice" }
```

## Build & Run

```bash
# Development
cargo run -p starkbot-app

# With specific persona
cargo run -p starkbot-app -- --persona coding-agent

# One-shot mode
cargo run -p starkbot-app -- --persona coding-agent "fix the bug in main.rs"

# Release build
cargo build --release
cp target/release/starkbot-app ~/.local/bin/starkbot

# Run tests
cargo test                           # Unit tests
cargo test --test agent_spice_test   # Agent behavior tests
```

## Feature Flags

```toml
# starkbot-app/Cargo.toml
[features]
default = ["anthropic"]
anthropic = ["rig-core/anthropic"]
openai = ["rig-core/openai"]
local-embeddings = ["fastembed"]
web3 = ["dep:ethers"]  # Optional web3 plugin
```

## Binary Size Optimization

```toml
# Cargo.toml (workspace)
[profile.release]
strip = true
lto = true
codegen-units = 1
opt-level = "z"  # Size optimization
```

## Development Workflow

1. **Core logic** → `starkbot-core` (testable without TUI)
2. **Tools** → `starkbot-tools` (unit testable with mocks)
3. **Memory** → `starkbot-memory` (testable with in-memory SQLite)
4. **TUI** → `starkbot-tui` (manual testing in terminal)
5. **Integration** → `starkbot-app` (full application)
6. **Agent behavior** → `tests/agent_spice_test.rs` (spice framework)

## Milestones

### M1: Foundation
- [ ] Workspace setup with all crates
- [ ] Basic TUI shell (ratatui + crossterm)
- [ ] Chat view with input
- [ ] Metalcraft agent running with 3 tools (read_file, bash, grep)
- [ ] Single persona (coding-agent)

### M2: Full Agent
- [ ] All tools implemented
- [ ] Persona system with switching
- [ ] Approval hooks in TUI
- [ ] Step guard with error detection
- [ ] Context compaction

### M3: Memory & Skills
- [ ] SQLite persistence layer
- [ ] Skill registry with hot-reload
- [ ] Memory system (FTS + embeddings + graph)
- [ ] Skills browser view
- [ ] Memory browser view

### M4: Multi-Agent & Graph
- [ ] Director + sub-agent spawning
- [ ] graf-rs library
- [ ] Skill graph visualization
- [ ] Memory graph visualization
- [ ] Knowledge graph (impulse map)

### M5: Production Polish
- [ ] Scheduling (cron + heartbeat)
- [ ] Full spice test suite
- [ ] Configuration system (RON)
- [ ] Session management (save/load/list)
- [ ] Performance optimization
