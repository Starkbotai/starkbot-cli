use starkbot_db::Database;

#[derive(Debug, Clone)]
pub enum MemoryCategory {
    Fact,
    Preference,
    Entity,
    Observation,
    Task,
    DailyLog,
    LongTerm,
    Identity,
}

impl MemoryCategory {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Fact => "fact",
            Self::Preference => "preference",
            Self::Entity => "entity",
            Self::Observation => "observation",
            Self::Task => "task",
            Self::DailyLog => "daily_log",
            Self::LongTerm => "long_term",
            Self::Identity => "identity",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "fact" => Self::Fact,
            "preference" => Self::Preference,
            "entity" => Self::Entity,
            "observation" => Self::Observation,
            "task" => Self::Task,
            "daily_log" => Self::DailyLog,
            "long_term" => Self::LongTerm,
            "identity" => Self::Identity,
            _ => Self::Fact,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Memory {
    pub id: String,
    pub category: MemoryCategory,
    pub content: String,
    pub importance: i32,
}

#[derive(Debug, Clone)]
pub struct ScoredMemory {
    pub memory: Memory,
    pub score: f32,
}

pub struct MemoryStore<'a> {
    db: &'a Database,
}

impl<'a> MemoryStore<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn store(&self, memory: &Memory) -> Result<(), String> {
        self.db
            .store_memory(&memory.id, memory.category.as_str(), &memory.content, memory.importance)
            .map_err(|e| format!("Failed to store memory: {}", e))
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<ScoredMemory>, String> {
        let results = self.db
            .search_memories_fts(query, limit)
            .map_err(|e| format!("Failed to search memories: {}", e))?;

        Ok(results
            .into_iter()
            .enumerate()
            .map(|(rank, (id, category, content, importance))| ScoredMemory {
                memory: Memory {
                    id,
                    category: MemoryCategory::from_str(&category),
                    content,
                    importance,
                },
                score: 1.0 / (60.0 + rank as f32), // RRF-style scoring
            })
            .collect())
    }
}

/// Build memory context string for agent system prompt injection.
pub fn build_memory_context(memories: &[ScoredMemory]) -> String {
    if memories.is_empty() {
        return String::new();
    }

    let mut ctx = String::from("\n## Relevant Memories\n\n");
    for mem in memories {
        ctx.push_str(&format!(
            "- [{}] {}\n",
            mem.memory.category.as_str(),
            mem.memory.content
        ));
    }
    ctx
}
