# StarkBot CLI - Architecture Plan

## Vision

StarkBot CLI is a terminal-native (TUI) reimagining of StarkBot — the autonomous AI agent framework. Instead of running as a cloud/browser application with Actix-web + React, it runs entirely in the terminal as an interactive TUI application.

## Core Principles

1. **Terminal-First** — No HTTP server, no browser. Everything happens in the terminal via a rich TUI interface.
2. **Metalcraft-Powered** — Uses metalcraft as the agentic graph engine (stateful graph orchestrator with ReAct loops, checkpointing, and tool execution).
3. **StarkBot Feature Parity** — Brings the key capabilities of StarkBot (multi-agent orchestration, skill system, memory, scheduling) to the terminal.
4. **Visual Skill Graph** — Uses graf-rs for interactive terminal-based skill/knowledge graph visualization.
5. **Spice-Tested** — Full test coverage using the spice framework for nondeterministic agent testing.

## Architecture Stack

```
┌─────────────────────────────────────────────────────────┐
│                    TUI Layer (ratatui)                    │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐  │
│  │   Chat   │ │  Skills  │ │  Graph   │ │  Memory   │  │
│  │   View   │ │  Browser │ │  Viewer  │ │  Browser  │  │
│  └──────────┘ └──────────┘ └──────────┘ └───────────┘  │
├─────────────────────────────────────────────────────────┤
│                 Application Core                         │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐  │
│  │ Dispatch │ │  Persona │ │Scheduler │ │  Session  │  │
│  │  Engine  │ │  Manager │ │          │ │  Manager  │  │
│  └──────────┘ └──────────┘ └──────────┘ └───────────┘  │
├─────────────────────────────────────────────────────────┤
│               Metalcraft Agentic Engine                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐  │
│  │  Graph   │ │   Tool   │ │Checkpoint│ │   Step    │  │
│  │ Executor │ │ Registry │ │  Store   │ │  Guards   │  │
│  └──────────┘ └──────────┘ └──────────┘ └───────────┘  │
├─────────────────────────────────────────────────────────┤
│                  Data & Storage                           │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐  │
│  │  SQLite  │ │  Memory  │ │  Skills  │ │  Config   │  │
│  │    DB    │ │Embeddings│ │   (MD)   │ │   (RON)   │  │
│  └──────────┘ └──────────┘ └──────────┘ └───────────┘  │
└─────────────────────────────────────────────────────────┘
```

## Key Differences from StarkBot (Cloud)

| Aspect | StarkBot (Cloud) | StarkBot CLI |
|--------|-----------------|--------------|
| Runtime | Actix-web HTTP server | Terminal TUI (ratatui) |
| UI | React + TypeScript SPA | Ratatui panels + graf-rs graphs |
| Agent Engine | Custom tool loop | Metalcraft graph executor |
| Deployment | Docker / cloud | Local binary |
| Communication | WebSocket streaming | Direct async channels |
| Channels | Discord/Slack/Telegram/Twitter | Terminal + optional pipe/socket |
| Wallet/Web3 | Full DeFi operations | Optional (plugin) |
| Testing | Ad-hoc | Spice framework (comprehensive) |

## Key Similarities to StarkBot (Cloud)

- Markdown-based skill system with YAML frontmatter
- Multi-agent orchestration (Director + specialized agents)
- Three-tier memory (FTS + vector + graph)
- Persona/subtype system for different agent modes
- Tool registry with 30+ built-in tools
- Session management and conversation history
- Scheduling and automation (heartbeat/cron)
- Knowledge graph (impulse map) — rendered via graf-rs in TUI
- RON configuration files
- SQLite persistence

## Document Index

| Document | Contents |
|----------|----------|
| [01-tui-architecture.md](./01-tui-architecture.md) | TUI layout, views, navigation, rendering |
| [02-metalcraft-integration.md](./02-metalcraft-integration.md) | How metalcraft powers the agent loops |
| [03-skill-system.md](./03-skill-system.md) | Skill format, registry, loading, execution |
| [04-tool-system.md](./04-tool-system.md) | Built-in tools, registry, execution model |
| [05-memory-system.md](./05-memory-system.md) | Three-tier memory, embeddings, graph |
| [06-multi-agent.md](./06-multi-agent.md) | Director, subtypes, sub-agent spawning |
| [07-graf-rs-integration.md](./07-graf-rs-integration.md) | Skill graph viewer, knowledge graph TUI |
| [08-testing-with-spice.md](./08-testing-with-spice.md) | Test strategy using spice framework |
| [09-config-and-persistence.md](./09-config-and-persistence.md) | RON config, SQLite schema, data model |
| [10-project-structure.md](./10-project-structure.md) | Crate layout, dependencies, build |
