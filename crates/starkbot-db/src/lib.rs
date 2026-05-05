use rusqlite::{Connection, Result as SqlResult};
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    fn migrate(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                persona TEXT NOT NULL,
                title TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                archived INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES sessions(id),
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);

            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                content TEXT NOT NULL,
                importance INTEGER NOT NULL DEFAULT 50,
                source TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                expires_at TEXT
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                content,
                category,
                tokenize='porter'
            );

            CREATE TABLE IF NOT EXISTS memory_embeddings (
                memory_id TEXT PRIMARY KEY REFERENCES memories(id),
                embedding BLOB NOT NULL,
                model TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS memory_associations (
                id TEXT PRIMARY KEY,
                from_id TEXT NOT NULL REFERENCES memories(id),
                to_id TEXT NOT NULL REFERENCES memories(id),
                relation TEXT NOT NULL,
                strength REAL NOT NULL DEFAULT 0.5,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS skills (
                name TEXT PRIMARY KEY,
                description TEXT,
                version TEXT,
                tags TEXT,
                requires_tools TEXT,
                file_path TEXT NOT NULL,
                loaded_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS tool_executions (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                tool_name TEXT NOT NULL,
                args TEXT,
                result_preview TEXT,
                success INTEGER NOT NULL,
                duration_ms INTEGER,
                executed_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS checkpoints (
                thread_id TEXT PRIMARY KEY,
                state BLOB NOT NULL,
                next_node TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            ",
        )?;
        Ok(())
    }

    // -- Session operations --

    pub fn create_session(&self, id: &str, persona: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO sessions (id, persona) VALUES (?1, ?2)",
            rusqlite::params![id, persona],
        )?;
        Ok(())
    }

    pub fn save_message(&self, id: &str, session_id: &str, role: &str, content: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO messages (id, session_id, role, content) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, session_id, role, content],
        )?;
        Ok(())
    }

    pub fn list_sessions(&self) -> SqlResult<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, persona, created_at FROM sessions WHERE archived = 0 ORDER BY updated_at DESC LIMIT 50",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        rows.collect()
    }

    // -- Memory operations --

    pub fn store_memory(&self, id: &str, category: &str, content: &str, importance: i32) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO memories (id, category, content, importance, updated_at) VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            rusqlite::params![id, category, content, importance],
        )?;
        // Update FTS
        self.conn.execute(
            "INSERT INTO memories_fts (rowid, content, category) VALUES ((SELECT rowid FROM memories WHERE id = ?1), ?2, ?3)",
            rusqlite::params![id, content, category],
        ).ok(); // Ignore if already exists
        Ok(())
    }

    pub fn search_memories_fts(&self, query: &str, limit: usize) -> SqlResult<Vec<(String, String, String, i32)>> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.category, m.content, m.importance FROM memories m
             JOIN memories_fts ON m.rowid = memories_fts.rowid
             WHERE memories_fts MATCH ?1
             ORDER BY rank LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![query, limit as i64], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;
        rows.collect()
    }

    pub fn add_association(&self, id: &str, from_id: &str, to_id: &str, relation: &str, strength: f64) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO memory_associations (id, from_id, to_id, relation, strength) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, from_id, to_id, relation, strength],
        )?;
        Ok(())
    }

    pub fn get_associations(&self, memory_id: &str) -> SqlResult<Vec<(String, String, String, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT to_id, from_id, relation, strength FROM memory_associations WHERE from_id = ?1 OR to_id = ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![memory_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;
        rows.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let db = Database::open_in_memory().unwrap();
        db.create_session("s1", "coding-agent").unwrap();
        db.save_message("m1", "s1", "user", "hello").unwrap();
        let sessions = db.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn test_memory_store_and_search() {
        let db = Database::open_in_memory().unwrap();
        db.store_memory("mem1", "fact", "Rust is a systems programming language", 80).unwrap();
        let results = db.search_memories_fts("rust programming", 10).unwrap();
        assert!(!results.is_empty());
    }
}
