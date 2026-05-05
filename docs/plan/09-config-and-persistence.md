# 09 - Configuration & Persistence

## Configuration System

### RON Format (from StarkBot)

```ron
// config/bot_config.ron
BotConfig(
    bot_name: "starkbot-cli",
    max_tool_iterations: 100,
    max_response_tokens: 4096,
    max_context_tokens: 128000,
    default_persona: "director",

    heartbeat: HeartbeatConfig(
        enabled: false,
        interval_minutes: 30,
        active_hours_start: "09:00",
        active_hours_end: "18:00",
    ),

    compaction: CompactionConfig(
        context_window: 128000,
        compact_threshold: 0.6,
        keep_recent_messages: 10,
    ),

    memory: MemoryConfig(
        embedding_model: "all-MiniLM-L6-v2",
        decay_halflife_days: 30.0,
        max_context_memories: 10,
        auto_store_enabled: true,
    ),

    tui: TuiConfig(
        default_view: "chat",
        graph_layout: "force-directed",
        color_scheme: "dark",
        show_tool_activity: true,
        show_token_count: true,
    ),
)
```

```ron
// config/ai_endpoints.ron
AiEndpoints(
    providers: [
        Provider(
            name: "anthropic",
            model: "claude-sonnet-4-20250514",
            api_key_env: "ANTHROPIC_API_KEY",
            base_url: "https://api.anthropic.com",
            max_tokens: 8192,
        ),
        Provider(
            name: "openai",
            model: "gpt-4o",
            api_key_env: "OPENAI_API_KEY",
            base_url: "https://api.openai.com/v1",
            max_tokens: 4096,
        ),
    ],
    default_provider: "anthropic",
)
```

### Environment Variables

```bash
# .env
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-proj-...

# Optional
STARKBOT_CONFIG_DIR=~/.config/starkbot-cli
STARKBOT_DATA_DIR=~/.local/share/starkbot-cli
STARKBOT_LOG_LEVEL=info
```

### Directory Layout

```
~/.config/starkbot-cli/           # XDG config
├── config/
│   ├── bot_config.ron
│   ├── ai_endpoints.ron
│   └── agents/
│       ├── director.json
│       ├── coding-agent.json
│       ├── research-agent.json
│       └── devops-agent.json
├── skills/
│   ├── planning.md
│   ├── debugging.md
│   └── ...
└── soul/                         # Agent personality
    └── SOUL.md

~/.local/share/starkbot-cli/      # XDG data
├── starkbot.db                   # SQLite database
├── embeddings/                   # Embedding model cache
├── notes/                        # Persistent notes
├── journal/                      # Session journals
└── traces/                       # Execution traces
```

## SQLite Schema

### Core Tables

```sql
-- Session management
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    persona TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    title TEXT,
    archived INTEGER DEFAULT 0
);

-- Message history
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    role TEXT NOT NULL,  -- 'user', 'assistant', 'tool_call', 'tool_result'
    content TEXT NOT NULL,
    metadata TEXT,  -- JSON
    created_at TEXT NOT NULL
);

-- Checkpoints (metalcraft)
CREATE TABLE checkpoints (
    thread_id TEXT PRIMARY KEY,
    state BLOB NOT NULL,  -- Serialized AgentState
    next_node TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### Memory Tables

```sql
-- Three-tier memory
CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    content TEXT NOT NULL,
    importance INTEGER NOT NULL DEFAULT 50,
    source TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    expires_at TEXT
);

-- FTS5 index
CREATE VIRTUAL TABLE memories_fts USING fts5(
    content,
    category,
    content='memories',
    content_rowid='rowid',
    tokenize='porter'
);

-- Vector embeddings
CREATE TABLE memory_embeddings (
    memory_id TEXT PRIMARY KEY REFERENCES memories(id),
    embedding BLOB NOT NULL,  -- f32 array as bytes
    model TEXT NOT NULL
);

-- Graph associations
CREATE TABLE memory_associations (
    id TEXT PRIMARY KEY,
    from_id TEXT NOT NULL REFERENCES memories(id),
    to_id TEXT NOT NULL REFERENCES memories(id),
    relation TEXT NOT NULL,
    strength REAL NOT NULL DEFAULT 0.5,
    created_at TEXT NOT NULL
);
```

### Skill Tables

```sql
-- Skill metadata cache
CREATE TABLE skills (
    name TEXT PRIMARY KEY,
    description TEXT,
    version TEXT,
    tags TEXT,  -- JSON array
    requires_tools TEXT,  -- JSON array
    file_path TEXT NOT NULL,
    loaded_at TEXT NOT NULL
);

-- Skill usage tracking
CREATE TABLE skill_usage (
    id TEXT PRIMARY KEY,
    skill_name TEXT NOT NULL,
    session_id TEXT,
    used_at TEXT NOT NULL
);
```

### Scheduling Tables

```sql
-- Cron jobs
CREATE TABLE cron_jobs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    cron_expression TEXT NOT NULL,
    task TEXT NOT NULL,
    persona TEXT NOT NULL DEFAULT 'director',
    enabled INTEGER DEFAULT 1,
    last_run TEXT,
    next_run TEXT,
    created_at TEXT NOT NULL
);

-- Cron job execution history
CREATE TABLE cron_runs (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES cron_jobs(id),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    status TEXT NOT NULL,  -- 'running', 'success', 'error'
    result TEXT,
    error TEXT
);
```

### Tool Execution Tracking

```sql
-- Tool call history (for analytics and debugging)
CREATE TABLE tool_executions (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES sessions(id),
    tool_name TEXT NOT NULL,
    args TEXT,  -- JSON
    result_preview TEXT,
    success INTEGER NOT NULL,
    duration_ms INTEGER,
    executed_at TEXT NOT NULL
);
```

## Persistence Layer

```rust
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(path: &Path) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite:{}?mode=rwc", path.display()))
            .await?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    // Session management
    pub async fn create_session(&self, persona: &str) -> Result<String>;
    pub async fn save_message(&self, session_id: &str, msg: &Message) -> Result<()>;
    pub async fn load_session_messages(&self, session_id: &str) -> Result<Vec<Message>>;
    pub async fn list_sessions(&self) -> Result<Vec<SessionSummary>>;

    // Memory operations
    pub async fn store_memory(&self, memory: &Memory) -> Result<()>;
    pub async fn search_memories_fts(&self, query: &str, limit: usize) -> Result<Vec<Memory>>;
    pub async fn store_embedding(&self, memory_id: &str, embedding: &[f32]) -> Result<()>;
    pub async fn add_association(&self, assoc: &MemoryAssociation) -> Result<()>;
    pub async fn get_associations(&self, memory_id: &str, max_hops: usize) -> Result<Vec<MemoryAssociation>>;

    // Checkpointing
    pub async fn save_checkpoint(&self, thread_id: &str, state: &[u8], next_node: &str) -> Result<()>;
    pub async fn load_checkpoint(&self, thread_id: &str) -> Result<Option<(Vec<u8>, String)>>;
}
```

## Configuration Loading

```rust
pub struct AppConfig {
    pub bot: BotConfig,
    pub ai: AiEndpoints,
    pub personas: HashMap<String, Persona>,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let config_dir = config_dir()?;

        let bot: BotConfig = ron::from_str(
            &std::fs::read_to_string(config_dir.join("config/bot_config.ron"))?
        )?;

        let ai: AiEndpoints = ron::from_str(
            &std::fs::read_to_string(config_dir.join("config/ai_endpoints.ron"))?
        )?;

        let mut personas = HashMap::new();
        for entry in std::fs::read_dir(config_dir.join("config/agents"))? {
            let entry = entry?;
            if entry.path().extension() == Some("json".as_ref()) {
                let persona: Persona = serde_json::from_str(
                    &std::fs::read_to_string(entry.path())?
                )?;
                personas.insert(persona.slug(), persona);
            }
        }

        Ok(Self { bot, ai, personas })
    }
}

fn config_dir() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("STARKBOT_CONFIG_DIR") {
        return Ok(PathBuf::from(dir));
    }
    dirs::config_dir()
        .map(|d| d.join("starkbot-cli"))
        .ok_or_else(|| anyhow!("Cannot determine config directory"))
}

fn data_dir() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("STARKBOT_DATA_DIR") {
        return Ok(PathBuf::from(dir));
    }
    dirs::data_dir()
        .map(|d| d.join("starkbot-cli"))
        .ok_or_else(|| anyhow!("Cannot determine data directory"))
}
```

## Migrations

Using sqlx migrations in `migrations/` directory:

```
migrations/
├── 001_initial_schema.sql
├── 002_memory_system.sql
├── 003_scheduling.sql
└── 004_tool_tracking.sql
```
