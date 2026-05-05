# StarkBot CLI

A terminal-native AI agent platform built in Rust. Think [stark-bot](https://github.com/anthropics/stark-bot) but running locally in your terminal instead of the cloud.

```
╭─────────────────────────────────────────────╮
│  StarkBot CLI                                │
│  Persona: Coding Agent                       │
│  Model:   gpt-4o                             │
╰─────────────────────────────────────────────╯
```

## Architecture

StarkBot CLI is built on three core pillars:

- **[metalcraft](https://crates.io/crates/metalcraft)** — LangGraph-style stateful graph engine for Rust. Handles the ReAct agent loop, tool calling, approval hooks, checkpointing, and step guards.
- **[rig](https://crates.io/crates/rig)** — Provider-agnostic LLM access. Currently wired to OpenAI; designed for multi-provider support.
- **[ratatui](https://crates.io/crates/ratatui)** — Terminal UI framework powering the interactive TUI with multiple views.

Testing is powered by **[spice-framework](https://crates.io/crates/spice-framework)**, a test harness for nondeterministic LLM agents with retry/consensus modes and 30+ assertions.

## Features

### Terminal UI

Four tabbed views navigable with `Tab`:

| View | Description |
|------|-------------|
| **Chat** | Conversation with the agent, input line, live tool activity feed |
| **Skills** | Browse and inspect loaded skills with `j`/`k` navigation |
| **Graph** | Force-directed skill relationship graph with pan/zoom/select |
| **Memory** | Memory browser (coming soon) |

### Agent Tools (10)

| Tool | Description |
|------|-------------|
| `bash` | Shell command execution with configurable timeout (default 60s, max 300s) |
| `read_file` | Read files with optional line ranges and line numbering |
| `write_file` | Create or overwrite files with automatic parent directory creation |
| `edit_file` | Find-and-replace editing requiring unique match |
| `grep` | Recursive content search with 100 match limit |
| `find_files` | Recursive filename search with 200 result limit |
| `list_files` | Directory listing with optional recursive mode (max depth 3) |
| `web_fetch` | HTTP fetch with HTML-to-markdown conversion |
| `load_skill` | Load skill prompts with frontmatter stripping |
| `sub_agent` | Spawn sub-agents with isolated tool sets and configurable limits |

### Personas

Persona configs live in `personas/*.json` and define which tools and skills each agent variant can access.

| Persona | Tools | Description |
|---------|-------|-------------|
| `coding-agent` | All 10 | General-purpose coding assistant |
| `research-agent` | read_file, grep, find_files, list_files, web_fetch, load_skill | Read-only research agent |
| `director` | sub_agent, load_skill, list_files | Orchestrator that delegates to sub-agents |
| `devops-agent` | bash, read_file, write_file, edit_file, grep, list_files | Infrastructure and deployment |

### Skills

Markdown files with YAML frontmatter in `skills/`. Hot-reloadable. Currently ships with:

- **debugging** — Systematic debugging workflow
- **code-review** — Code review checklist and standards
- **planning** — Project planning and task breakdown
- **explore-codebase** — Codebase exploration strategy

### Approval System

Two modes controlled by `--auto-approve`:

- **Interactive** (default) — Read-only tools auto-approved; write/exec tools require confirmation
- **Auto-approve** — All tools run without confirmation

### Safety

- **Step guard** with error spiral detection (3 consecutive all-error turns → halt) and loop detection (repeated identical tool calls)
- **Output truncation** — stdout capped at 30K chars, stderr at 10K
- **Tool-level approval hooks** — per-tool allow/deny decisions

### Memory & Storage

SQLite-backed with:
- Full-text search (FTS5) on memory content
- Embeddings table for future vector similarity search
- Association graph for memory relationships
- 8 memory categories: Fact, Preference, Entity, Observation, Task, DailyLog, LongTerm, Identity

### Context Management

- Token estimation (~4 chars/token)
- Sliding-window compaction via LLM summarization when context approaches limits
- Configurable thresholds (default 128K window, 0.6 compaction ratio)

## Getting Started

### Prerequisites

- Rust toolchain (1.75+)
- An OpenAI API key

### Build

```bash
git clone <repo-url> && cd starkbot-cli
cargo build --release
```

### Configure

```bash
cp .env.example .env
# Edit .env and set your OPENAI_API_KEY
```

### Run

**Interactive TUI:**
```bash
./target/release/starkbot                          # default: coding-agent
./target/release/starkbot research-agent           # specific persona
./target/release/starkbot coding-agent --auto-approve  # skip tool confirmations
```

**One-shot mode:**
```bash
./target/release/starkbot coding-agent "Read Cargo.toml and summarize the dependencies"
```

### TUI Controls

| Key | Action |
|-----|--------|
| `Tab` | Cycle through views (Chat → Skills → Graph → Memory) |
| `Enter` | Send message |
| `Ctrl+C` | Quit |
| `j`/`k` | Navigate skills list (Skills view) |
| Arrow keys | Pan graph (Graph view) |
| `+`/`-` | Zoom graph |
| `n`/`p` | Select next/prev node (Graph view) |
| `/quit` | Quit |
| `/clear` | Clear conversation |
| `/tokens` | Show estimated token count |
| `/help` | Show help |

## Project Structure

```
starkbot-cli/
├── Cargo.toml              # Workspace root
├── personas/               # Agent persona configs (JSON)
├── skills/                 # Skill definitions (Markdown + YAML)
├── docs/
│   ├── plan/               # Architecture plan (11 documents)
│   └── current_gaps.md     # Gap analysis vs stark-bot
└── crates/
    ├── starkbot-app/       # Binary entry point, TUI event loop, one-shot mode
    ├── starkbot-tui/       # ratatui views, input handling, drawing
    ├── starkbot-core/      # Persona loading, context compaction, AgentRunner
    ├── starkbot-tools/     # All 10 tools, approval hooks, step guard
    ├── starkbot-skills/    # Skill registry, YAML frontmatter parsing
    ├── starkbot-memory/    # Memory store, categories, context builder
    ├── starkbot-db/        # SQLite schema, migrations, queries
    └── starkbot-graph/     # Force-directed layout, ratatui graph widget
```

## Testing

```bash
# Unit tests
cargo test

# Agent integration tests (requires OPENAI_API_KEY)
cargo test -p starkbot-app -- --ignored

# Filter by test name or tag
TEST_FILTER=reads-file cargo test -p starkbot-app -- --ignored
TEST_TAG=security cargo test -p starkbot-app -- --ignored
```

## Roadmap

See [docs/current_gaps.md](docs/current_gaps.md) for a detailed gap analysis against stark-bot. Key priorities:

1. **More tools** — git, delete/rename file, glob, apply_patch, committer, deploy
2. **Wired hybrid memory** — vector search via embedding server, graph expansion, RRF fusion
3. **Parallel sub-agents** — dependency chains, concurrency limits, status tracking
4. **Scheduling** — cron jobs, heartbeat system, autonomous operation
5. **Multi-model** — Anthropic Claude, local models via llama endpoints
6. **Knowledge graph** — impulse map with heartbeat traversal
7. **Soul system** — self-modifying agent identity

## License

MIT
