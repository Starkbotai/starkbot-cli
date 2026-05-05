# 05 - Memory System

## Overview

Three-tier hybrid memory system matching StarkBot's architecture, optimized for terminal use.

## Architecture

```
┌──────────────────────────────────────────────────────┐
│              Memory Retrieval (RRF Fusion)             │
├──────────────────────────────────────────────────────┤
│                                                       │
│  Tier 1: FTS        Tier 2: Vector     Tier 3: Graph │
│  ┌──────────┐      ┌──────────┐      ┌──────────┐   │
│  │  SQLite  │      │ Embedded │      │   Typed  │   │
│  │   FTS5   │      │  384-dim │      │Relations │   │
│  └──────────┘      └──────────┘      └──────────┘   │
│                                                       │
├──────────────────────────────────────────────────────┤
│                    SQLite Storage                      │
└──────────────────────────────────────────────────────┘
```

## Memory Types

```rust
pub enum MemoryCategory {
    Fact,           // Objective knowledge
    Preference,     // User preferences
    Entity,         // Person, place, thing
    Observation,    // Contextual note
    Task,           // Action item
    DailyLog,       // Session-ephemeral
    LongTerm,       // Persistent knowledge
    Identity,       // Core identity (no decay)
}

pub struct Memory {
    pub id: String,
    pub category: MemoryCategory,
    pub content: String,
    pub importance: u8,         // 0-100
    pub source: String,         // How it was created
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub embedding: Option<Vec<f32>>,  // 384-dim
}
```

## Tier 1: Full-Text Search (FTS5)

```sql
CREATE VIRTUAL TABLE memories_fts USING fts5(
    content,
    category,
    tokenize='porter'
);

-- Query
SELECT m.*, rank FROM memories m
JOIN memories_fts ON m.id = memories_fts.rowid
WHERE memories_fts MATCH ?
ORDER BY rank;
```

## Tier 2: Vector Embeddings

```rust
pub struct EmbeddingEngine {
    // Local embedding model (e.g., all-MiniLM-L6-v2 via candle or ort)
    model: EmbeddingModel,
}

impl EmbeddingEngine {
    /// Generate 384-dim embedding for text
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Find top-k similar memories by cosine similarity
    pub async fn search(&self, query_embedding: &[f32], k: usize) -> Result<Vec<(String, f32)>>;
}
```

Options for local embedding:
- **candle** (Rust-native, fast, no external deps)
- **ort** (ONNX Runtime, broader model support)
- **fastembed-rs** (high-level wrapper)

## Tier 3: Graph Associations

```rust
pub enum RelationType {
    RelatedTo,
    Updates,
    Contradicts,
    CausedBy,
    ResultOf,
    PartOf,
}

pub struct MemoryAssociation {
    pub from_id: String,
    pub to_id: String,
    pub relation: RelationType,
    pub strength: f32,  // 0.0 - 1.0
    pub created_at: DateTime<Utc>,
}
```

Graph traversal for context expansion:
```rust
/// Given a set of memory IDs, expand via graph associations
pub fn expand_context(
    seeds: &[String],
    max_hops: usize,
    min_strength: f32,
) -> Vec<Memory>;
```

## Retrieval: Reciprocal Rank Fusion (RRF)

```rust
pub struct HybridSearch {
    fts: FtsEngine,
    vector: EmbeddingEngine,
    graph: GraphEngine,
}

impl HybridSearch {
    pub async fn search(&self, query: &str, k: usize) -> Vec<ScoredMemory> {
        // Run all three in parallel
        let (fts_results, vec_results, graph_results) = tokio::join!(
            self.fts.search(query, k * 2),
            self.vector.search(query, k * 2),
            self.graph.search(query, k),
        );

        // Reciprocal Rank Fusion
        let mut scores: HashMap<String, f32> = HashMap::new();
        let rrf_k = 60.0; // Standard RRF constant

        for (rank, result) in fts_results.iter().enumerate() {
            *scores.entry(result.id.clone()).or_default() += 1.0 / (rrf_k + rank as f32);
        }
        for (rank, result) in vec_results.iter().enumerate() {
            *scores.entry(result.id.clone()).or_default() += 1.0 / (rrf_k + rank as f32);
        }
        for (rank, result) in graph_results.iter().enumerate() {
            *scores.entry(result.id.clone()).or_default() += 1.0 / (rrf_k + rank as f32);
        }

        // Sort by fused score, return top-k
        let mut results: Vec<_> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(k);

        results.into_iter()
            .map(|(id, score)| ScoredMemory { id, score, ..load(id) })
            .collect()
    }
}
```

## Memory Decay

```rust
pub struct DecayConfig {
    pub base_halflife_days: f64,      // 30 days default
    pub importance_multiplier: f64,    // Higher importance = slower decay
    pub exempt_categories: Vec<MemoryCategory>,  // Identity never decays
}

pub fn calculate_relevance(memory: &Memory, config: &DecayConfig) -> f32 {
    if config.exempt_categories.contains(&memory.category) {
        return 1.0;
    }

    let age_days = (Utc::now() - memory.created_at).num_days() as f64;
    let halflife = config.base_halflife_days * (1.0 + memory.importance as f64 / 100.0);
    let decay = 0.5_f64.powf(age_days / halflife);

    decay as f32
}
```

## Auto-Storage

The agent can store memories explicitly via tool, or the system can auto-detect:

```rust
pub fn detect_auto_store(assistant_message: &str) -> Vec<AutoMemory> {
    let mut memories = vec![];

    // Pattern: [REMEMBER: content]
    for cap in REMEMBER_RE.captures_iter(assistant_message) {
        memories.push(AutoMemory {
            content: cap[1].to_string(),
            category: MemoryCategory::LongTerm,
        });
    }

    // Pattern: [DAILY_LOG: content]
    for cap in DAILY_LOG_RE.captures_iter(assistant_message) {
        memories.push(AutoMemory {
            content: cap[1].to_string(),
            category: MemoryCategory::DailyLog,
        });
    }

    memories
}
```

## Memory Tools

### `memory_store`
```json
{
  "content": "The auth module uses JWT with 24h expiry",
  "category": "fact",
  "importance": 75
}
```

### `memory_search`
```json
{
  "query": "auth token refresh",
  "category": "fact",
  "limit": 10
}
```

### `memory_graph`
```json
{
  "memory_id": "mem_abc123",
  "operation": "get_associations",
  "max_hops": 2
}
```

## Context Injection

Before each agent turn, relevant memories are injected:

```rust
pub async fn build_memory_context(
    memory: &HybridSearch,
    user_message: &str,
    max_memories: usize,
) -> String {
    let results = memory.search(user_message, max_memories).await;

    if results.is_empty() {
        return String::new();
    }

    let mut context = String::from("\n## Relevant Memories\n\n");
    for mem in results {
        context.push_str(&format!("- [{}] {}\n", mem.category, mem.content));
    }
    context
}
```

## Visualization in TUI

The memory graph view uses graf-rs to render associations:

```
[JWT auth] ──RelatedTo── [session management]
     │                          │
  PartOf                     PartOf
     │                          │
     ▼                          ▼
[auth module]              [user system]
     │
  Updates
     │
     ▼
[old cookie auth]  (decayed, low relevance)
```
