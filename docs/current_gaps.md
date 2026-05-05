# starkbot-cli Gap Analysis vs stark-bot

**Baseline commit:** `611e351` (init)
**Date:** 2026-05-05

---

## What starkbot-cli HAS (parity with stark-bot)

- ReAct agent loop via metalcraft
- Tools: bash, read/write/edit file, grep, find_files, list_files, web_fetch, load_skill, sub_agent (10 tools)
- Persona/subtype system (4 personas: coding-agent, research-agent, director, devops-agent)
- Skill system with markdown + YAML frontmatter (4 skills)
- SQLite DB with FTS5, embeddings table, associations table
- Memory store with categories and hybrid search scaffolding
- Context compaction (sliding window, LLM summarization)
- Approval hooks (interactive / auto-approve)
- Step guard (error spiral + loop detection)
- TUI with 4 views: Chat, Skills, Graph, Memory
- Force-directed skill graph widget (custom ratatui Widget)
- Spice test harness integration
- One-shot + interactive TUI modes

---

## Tier 1 — Core Agent Capabilities

These gaps make the biggest difference to day-to-day usefulness.

| Gap | stark-bot | starkbot-cli |
|-----|-----------|--------------|
| **Filesystem tools** | `delete_file`, `rename_file`, `glob`, `apply_patch`, `read_symbol` | Missing |
| **Git tool** | Full git operations tool | Missing |
| **Dev tools** | `committer`, `deploy`, `index_project`, `pr_quality`, `verify_changes` | Missing |
| **Hybrid memory search** | FTS5 + vector cosine + graph expansion + RRF fusion, all wired | Schema exists, vector search & graph expansion not implemented |
| **Memory decay** | Time-based half-life (30d), access boost, auto-pruning | Not implemented |
| **Memory PII redaction** | Auto-redacts private keys, JWTs, API keys before storing | Not implemented |
| **Memory consolidation** | Duplicate detection via cosine similarity before insert | Not implemented |
| **Embedding server integration** | Configurable embedding endpoint for vector search | Not implemented |
| **Parallel sub-agents** | `spawn_subagents` with dependency chains, concurrency limits, status/cancel | Single sub-agent spawn only |
| **Register store** | CPU-register-like store passing data between tools without LLM routing | Not implemented |
| **ask_user / say_to_user** | Agent can request confirmation or send non-blocking messages | Not implemented |
| **Pre-compaction memory flush** | AI extracts memories from context before compaction | Not implemented |

---

## Tier 2 — Autonomy & Self-Organization

Features that enable the agent to operate independently over time.

| Gap | stark-bot | starkbot-cli |
|-----|-----------|--------------|
| **Cron / scheduling** | Full cron + interval + one-shot with error backoff, concurrent job limits | Not implemented |
| **Heartbeat system** | Periodic autonomous runs with active hours/days window | Not implemented |
| **Impulse map** | Directed graph of idea nodes, heartbeat traversal, full CRUD tool | Not implemented (graph widget exists but for skills only) |
| **Soul system** | Self-modifying identity doc (`SOUL.md`), `modify_soul` tool | Not implemented |
| **Notes system** | Obsidian-compatible markdown notes with FTS5, wikilinks, tags | Not implemented |
| **Kanban / workstream** | Task board with CRUD via `modify_kanban` tool | Not implemented |
| **Persona hooks** | Event-driven triggers (discord_message, heartbeat, twitter_mention, etc.) | Not implemented |
| **Task planning mode** | TaskPlanner → Assistant mode transition | Not implemented |

---

## Tier 3 — Ecosystem Integration

External services and protocol support.

### Multi-Model Support

| Gap | Status |
|-----|--------|
| Anthropic Claude provider | Missing (OpenAI only) |
| Kimi / Moonshot AI | Missing |
| MiniMax M2.5 | Missing |
| Llama / generic endpoints | Missing |
| Per-subtype model override | Missing |
| Whisper speech-to-text | Missing |

### Web3 / DeFi

| Gap | Status |
|-----|--------|
| Token lookup / swap / bridge | Missing |
| Web3 tx signing / broadcasting / queue | Missing |
| ERC-8128, EIP-8004, x402 payment protocol | Missing |
| Multi-chain EVM network selection | Missing |
| Wallet management (local + Privy) | Missing |

### Communication / Social

| Gap | Status |
|-----|--------|
| Telegram bot integration | Missing |
| Discord integration | Missing |
| Twitter/X posting + monitoring | Missing |
| Slack integration | Missing |
| Gmail / Pub-Sub | Missing |
| WebSocket gateway | Missing (TUI is local-only) |
| Gateway channel management | Missing |

### Plugin / Package System

| Gap | Status |
|-----|--------|
| Module system (dynamic microservice plugins) | Missing |
| HyperPack package manager (git/registry) | Missing |
| Module dashboards | Missing |
| Module backup/restore integration | Missing |

---

## Tier 4 — Production Hardening

| Gap | stark-bot | starkbot-cli |
|-----|-----------|--------------|
| **Safe mode** | Restricted tool set for untrusted/external input | Not implemented |
| **Special roles / RBAC** | Named permission grants per user/channel | Not implemented |
| **Cloud backup** | ECIES-encrypted backup/restore | Not implemented |
| **Session lane manager** | Serializes concurrent requests per channel | Not implemented |
| **Rollout / telemetry / spans** | Structured execution traces, reward signals | Not implemented |
| **Retry policies** | Configurable conditions with exponential backoff | Not implemented |
| **Disk quota management** | Storage limit enforcement | Not implemented |
| **Rogue / Partner modes** | Operating mode with different confirmation requirements | Approval hooks exist but no formal mode system |

---

## By the Numbers

| Metric | stark-bot | starkbot-cli |
|--------|-----------|--------------|
| Tools | 70+ | 10 |
| Skills | 50+ | 4 |
| Personas | Many (dynamic) | 4 (static JSON) |
| AI providers | 5+ (OpenAI, Claude, Kimi, MiniMax, Llama) | 1 (OpenAI) |
| Lines of Rust | ~60,000 | ~2,800 |
| Feature coverage | 100% | ~15% |

---

## Notes

- starkbot-cli's TUI, graph widget, and metalcraft integration have no direct equivalent in stark-bot (which uses a web UI). These are net-new.
- The spice test harness is also net-new; stark-bot has no equivalent agent testing framework.
- Many Tier 3 gaps (web3, social) may not apply if starkbot-cli targets a developer-tools-only use case.
