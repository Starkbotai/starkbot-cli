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

            CREATE TABLE IF NOT EXISTS external_api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                service_name TEXT NOT NULL UNIQUE,
                api_key TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS skill_embeddings (
                skill_name TEXT PRIMARY KEY,
                embedding BLOB NOT NULL,
                model TEXT NOT NULL,
                dimensions INTEGER NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS skills_fts USING fts5(
                name,
                description,
                tags,
                content,
                tokenize='porter'
            );

            CREATE TABLE IF NOT EXISTS channels (
                id TEXT PRIMARY KEY,
                channel_type TEXT NOT NULL,
                name TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 0,
                safe_mode INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS channel_settings (
                channel_id TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                PRIMARY KEY (channel_id, key)
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

    // -- API Key operations --

    pub fn upsert_api_key(&self, service_name: &str, api_key: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO external_api_keys (service_name, api_key) VALUES (?1, ?2)
             ON CONFLICT(service_name) DO UPDATE SET api_key = excluded.api_key, updated_at = datetime('now')",
            rusqlite::params![service_name, api_key],
        )?;
        Ok(())
    }

    pub fn get_api_key(&self, service_name: &str) -> SqlResult<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT api_key FROM external_api_keys WHERE service_name = ?1",
        )?;
        let mut rows = stmt.query(rusqlite::params![service_name])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    pub fn delete_api_key(&self, service_name: &str) -> SqlResult<bool> {
        let count = self.conn.execute(
            "DELETE FROM external_api_keys WHERE service_name = ?1",
            rusqlite::params![service_name],
        )?;
        Ok(count > 0)
    }

    /// List all API keys with masked values: (service_name, masked_key, updated_at)
    pub fn list_api_keys(&self) -> SqlResult<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT service_name, api_key, updated_at FROM external_api_keys ORDER BY service_name",
        )?;
        let rows = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let key: String = row.get(1)?;
            let updated: String = row.get(2)?;
            Ok((name, mask_key(&key), updated))
        })?;
        rows.collect()
    }

    /// List all API keys with raw (unmasked) values: (service_name, api_key).
    /// Used for migration to keys.json.
    pub fn list_api_keys_raw(&self) -> SqlResult<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT service_name, api_key FROM external_api_keys ORDER BY service_name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        rows.collect()
    }

    /// Returns names of all configured API keys.
    pub fn get_configured_key_names(&self) -> SqlResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT service_name FROM external_api_keys ORDER BY service_name",
        )?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect()
    }

    // -- Skill FTS operations --

    pub fn upsert_skill_fts(&self, name: &str, description: &str, tags: &str, content: &str) -> SqlResult<()> {
        // Delete existing entry first (FTS5 doesn't support ON CONFLICT)
        self.conn.execute(
            "DELETE FROM skills_fts WHERE name = ?1",
            rusqlite::params![name],
        ).ok();
        self.conn.execute(
            "INSERT INTO skills_fts (name, description, tags, content) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![name, description, tags, content],
        )?;
        Ok(())
    }

    pub fn search_skills_fts(&self, query: &str, limit: usize) -> SqlResult<Vec<(String, String, String, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, description, tags, rank FROM skills_fts WHERE skills_fts MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![query, limit as i64], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;
        rows.collect()
    }

    pub fn upsert_skill_embedding(&self, skill_name: &str, embedding: &[u8], model: &str, dimensions: i32) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO skill_embeddings (skill_name, embedding, model, dimensions) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(skill_name) DO UPDATE SET embedding = excluded.embedding, model = excluded.model, dimensions = excluded.dimensions, updated_at = datetime('now')",
            rusqlite::params![skill_name, embedding, model, dimensions],
        )?;
        Ok(())
    }

    pub fn all_skill_embeddings(&self) -> SqlResult<Vec<(String, Vec<u8>, String, i32)>> {
        let mut stmt = self.conn.prepare(
            "SELECT skill_name, embedding, model, dimensions FROM skill_embeddings",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;
        rows.collect()
    }

    // -- Channel operations --

    pub fn create_channel(&self, id: &str, channel_type: &str, name: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO channels (id, channel_type, name) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, channel_type, name],
        )?;
        Ok(())
    }

    /// List all channels: (id, channel_type, name, enabled, safe_mode, created_at)
    pub fn list_channels(&self) -> SqlResult<Vec<(String, String, String, bool, bool, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, channel_type, name, enabled, safe_mode, created_at FROM channels ORDER BY created_at",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get::<_, i32>(3)? != 0,
                row.get::<_, i32>(4)? != 0,
                row.get(5)?,
            ))
        })?;
        rows.collect()
    }

    pub fn delete_channel(&self, id: &str) -> SqlResult<bool> {
        let count = self.conn.execute(
            "DELETE FROM channels WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(count > 0)
    }

    pub fn set_channel_enabled(&self, id: &str, enabled: bool) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE channels SET enabled = ?2 WHERE id = ?1",
            rusqlite::params![id, enabled as i32],
        )?;
        Ok(())
    }

    pub fn set_channel_setting(&self, channel_id: &str, key: &str, value: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO channel_settings (channel_id, key, value) VALUES (?1, ?2, ?3)
             ON CONFLICT(channel_id, key) DO UPDATE SET value = excluded.value",
            rusqlite::params![channel_id, key, value],
        )?;
        Ok(())
    }

    pub fn get_channel_settings(&self, channel_id: &str) -> SqlResult<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT key, value FROM channel_settings WHERE channel_id = ?1 ORDER BY key",
        )?;
        let rows = stmt.query_map(rusqlite::params![channel_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        rows.collect()
    }

    pub fn delete_channel_settings(&self, channel_id: &str) -> SqlResult<()> {
        self.conn.execute(
            "DELETE FROM channel_settings WHERE channel_id = ?1",
            rusqlite::params![channel_id],
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

/// Mask an API key, showing first 4 and last 4 chars.
pub fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    format!("{}...{}", &key[..4], &key[key.len() - 4..])
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

    #[test]
    fn test_api_key_crud() {
        let db = Database::open_in_memory().unwrap();
        db.upsert_api_key("CLOUDFLARE_API_TOKEN", "cf-abc123def456ghi789").unwrap();
        db.upsert_api_key("GITHUB_TOKEN", "ghp_xxxxxxxxxxxx1234").unwrap();

        // Get
        let key = db.get_api_key("CLOUDFLARE_API_TOKEN").unwrap();
        assert_eq!(key, Some("cf-abc123def456ghi789".to_string()));

        // List (masked)
        let keys = db.list_api_keys().unwrap();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].0, "CLOUDFLARE_API_TOKEN");
        assert!(keys[0].1.contains("..."));

        // Names
        let names = db.get_configured_key_names().unwrap();
        assert_eq!(names, vec!["CLOUDFLARE_API_TOKEN", "GITHUB_TOKEN"]);

        // Update
        db.upsert_api_key("CLOUDFLARE_API_TOKEN", "cf-newkey999").unwrap();
        let key = db.get_api_key("CLOUDFLARE_API_TOKEN").unwrap();
        assert_eq!(key, Some("cf-newkey999".to_string()));

        // Delete
        assert!(db.delete_api_key("GITHUB_TOKEN").unwrap());
        assert!(!db.delete_api_key("NONEXISTENT").unwrap());
        assert_eq!(db.get_configured_key_names().unwrap().len(), 1);
    }

    #[test]
    fn test_mask_key() {
        assert_eq!(mask_key("abcdefghijklmnop"), "abcd...mnop");
        assert_eq!(mask_key("short"), "*****");
        assert_eq!(mask_key("12345678"), "********");
        assert_eq!(mask_key("123456789"), "1234...6789");
    }

    #[test]
    fn test_skill_fts() {
        let db = Database::open_in_memory().unwrap();
        db.upsert_skill_fts("cloudflare-dns", "Manage Cloudflare DNS records", "infrastructure dns cloudflare", "DNS management skill content").unwrap();
        db.upsert_skill_fts("github", "GitHub operations", "development github git", "GitHub operations content").unwrap();

        let results = db.search_skills_fts("cloudflare", 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "cloudflare-dns");

        // Upsert should replace
        db.upsert_skill_fts("cloudflare-dns", "Updated description", "infrastructure dns", "Updated content").unwrap();
        let results = db.search_skills_fts("cloudflare", 10).unwrap();
        assert_eq!(results.len(), 1);
    }
}
