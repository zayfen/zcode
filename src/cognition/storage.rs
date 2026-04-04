//! SQLite storage for cognition engine
//!
//! Persistent storage for code blocks, embeddings, session memories,
//! and knowledge graph.

use crate::error::Result;
use rusqlite::{params, Connection};
use std::path::Path;

/// Cognition storage backend using SQLite
#[derive(Debug)]
pub struct CognitionStorage {
    conn: Connection,
}

impl CognitionStorage {
    /// Open or create a cognition database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        let storage = Self { conn };
        storage.initialize_schema()?;
        Ok(storage)
    }

    /// Create an in-memory database for testing
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let storage = Self { conn };
        storage.initialize_schema()?;
        Ok(storage)
    }

    /// Initialize database schema
    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS code_blocks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                identifier TEXT,
                content TEXT NOT NULL,
                kind TEXT NOT NULL,
                embedding BLOB,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS file_hashes (
                path TEXT PRIMARY KEY,
                hash TEXT NOT NULL,
                indexed_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS session_memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                summary TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS decisions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_memory_id INTEGER NOT NULL,
                description TEXT NOT NULL,
                rationale TEXT NOT NULL,
                context_files TEXT,
                embedding BLOB,
                FOREIGN KEY (session_memory_id) REFERENCES session_memories(id) ON DELETE CASCADE
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS learned_patterns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_memory_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                examples TEXT,
                embedding BLOB,
                FOREIGN KEY (session_memory_id) REFERENCES session_memories(id) ON DELETE CASCADE
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS knowledge_nodes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                source TEXT NOT NULL,
                embedding BLOB,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(kind, title, source)
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS knowledge_edges (
                from_node INTEGER NOT NULL,
                to_node INTEGER NOT NULL,
                relation TEXT NOT NULL,
                weight REAL DEFAULT 1.0,
                PRIMARY KEY (from_node, to_node, relation),
                FOREIGN KEY (from_node) REFERENCES knowledge_nodes(id) ON DELETE CASCADE,
                FOREIGN KEY (to_node) REFERENCES knowledge_nodes(id) ON DELETE CASCADE
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS external_knowledge (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                kind TEXT NOT NULL,
                content TEXT NOT NULL,
                relevance_score REAL DEFAULT 0.0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS project_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        // Create indexes for common queries
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_code_blocks_path ON code_blocks(path)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_session_memories_session_id ON session_memories(session_id)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_knowledge_nodes_kind ON knowledge_nodes(kind)",
            [],
        )?;

        Ok(())
    }

    /// Store a code block with optional embedding
    pub fn store_block(
        &self,
        path: &str,
        start_line: usize,
        end_line: usize,
        identifier: Option<&str>,
        content: &str,
        kind: &str,
        embedding: Option<&[u8]>,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT OR REPLACE INTO code_blocks (path, start_line, end_line, identifier, content, kind, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                path,
                start_line as i64,
                end_line as i64,
                identifier,
                content,
                kind,
                embedding,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Load all blocks for a file
    pub fn load_blocks_for_file(&self, path: &str) -> Result<Vec<CodeBlockRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, start_line, end_line, identifier, content, kind, embedding
             FROM code_blocks WHERE path = ?1",
        )?;

        let blocks = stmt
            .query_map(params![path], |row| {
                Ok(CodeBlockRecord {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    start_line: row.get(2)?,
                    end_line: row.get(3)?,
                    identifier: row.get(4)?,
                    content: row.get(5)?,
                    kind: row.get(6)?,
                    embedding: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(blocks)
    }

    /// Store a file hash to track changes
    pub fn store_file_hash(&self, path: &str, hash: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO file_hashes (path, hash) VALUES (?1, ?2)",
            params![path, hash],
        )?;
        Ok(())
    }

    /// Get stored hash for a file
    pub fn get_file_hash(&self, path: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT hash FROM file_hashes WHERE path = ?1")?;

        let mut rows = stmt.query_map(params![path], |row| row.get(0))?;

        Ok(rows.next().transpose()?.unwrap_or(None))
    }

    /// Store a session memory
    pub fn store_session_memory(&self, session_id: &str, summary: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO session_memories (session_id, summary) VALUES (?1, ?2)",
            params![session_id, summary],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Store a decision record
    pub fn store_decision(
        &self,
        session_memory_id: i64,
        description: &str,
        rationale: &str,
        context_files: Option<&str>,
        embedding: Option<&[u8]>,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO decisions (session_memory_id, description, rationale, context_files, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_memory_id, description, rationale, context_files, embedding],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Store a learned pattern
    pub fn store_learned_pattern(
        &self,
        session_memory_id: i64,
        name: &str,
        description: &str,
        examples: Option<&str>,
        embedding: Option<&[u8]>,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO learned_patterns (session_memory_id, name, description, examples, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_memory_id, name, description, examples, embedding],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Recall session memories with optional limit
    pub fn recall_memories(&self, limit: usize) -> Result<Vec<SessionMemoryRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, summary, created_at FROM session_memories
             ORDER BY created_at DESC LIMIT ?1",
        )?;

        let memories = stmt
            .query_map(params![limit as i64], |row| {
                Ok(SessionMemoryRecord {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    summary: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(memories)
    }

    /// Store a knowledge node
    pub fn store_knowledge_node(
        &self,
        kind: &str,
        title: &str,
        content: &str,
        source: &str,
        embedding: Option<&[u8]>,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT OR REPLACE INTO knowledge_nodes (kind, title, content, source, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![kind, title, content, source, embedding],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get a knowledge node by kind and title
    pub fn get_knowledge_node(&self, kind: &str, title: &str) -> Result<Option<KnowledgeNodeRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, title, content, source, embedding, created_at
             FROM knowledge_nodes WHERE kind = ?1 AND title = ?2",
        )?;

        let mut rows = stmt.query_map(params![kind, title], |row| {
            Ok(KnowledgeNodeRecord {
                id: row.get(0)?,
                kind: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                source: row.get(4)?,
                embedding: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        match rows.next() {
            Some(Ok(record)) => Ok(Some(record)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Store project metadata
    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO project_meta (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Get project metadata
    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM project_meta WHERE key = ?1")?;

        let mut rows = stmt.query_map(params![key], |row| row.get(0))?;

        Ok(rows.next().transpose()?.unwrap_or(None))
    }
}

/// Code block record from storage
#[derive(Debug, Clone)]
pub struct CodeBlockRecord {
    pub id: i64,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub identifier: Option<String>,
    pub content: String,
    pub kind: String,
    pub embedding: Option<Vec<u8>>,
}

/// Session memory record from storage
#[derive(Debug, Clone)]
pub struct SessionMemoryRecord {
    pub id: i64,
    pub session_id: String,
    pub summary: String,
    pub created_at: String,
}

/// Knowledge node record from storage
#[derive(Debug, Clone)]
pub struct KnowledgeNodeRecord {
    pub id: i64,
    pub kind: String,
    pub title: String,
    pub content: String,
    pub source: String,
    pub embedding: Option<Vec<u8>>,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_in_memory() {
        let storage = CognitionStorage::in_memory().unwrap();
        assert!(storage.set_meta("test", "value").is_ok());
        assert_eq!(storage.get_meta("test").unwrap(), Some("value".to_string()));
    }

    #[test]
    fn test_store_and_load_block() {
        let storage = CognitionStorage::in_memory().unwrap();

        storage
            .store_block("test.rs", 1, 10, Some("foo"), "content", "Function", None)
            .unwrap();

        let blocks = storage.load_blocks_for_file("test.rs").unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].path, "test.rs");
        assert_eq!(blocks[0].start_line, 1);
        assert_eq!(blocks[0].end_line, 10);
        assert_eq!(blocks[0].identifier, Some("foo".to_string()));
    }

    #[test]
    fn test_file_hash_tracking() {
        let storage = CognitionStorage::in_memory().unwrap();

        assert_eq!(storage.get_file_hash("test.rs").unwrap(), None);

        storage.store_file_hash("test.rs", "abc123").unwrap();
        assert_eq!(
            storage.get_file_hash("test.rs").unwrap(),
            Some("abc123".to_string())
        );

        storage.store_file_hash("test.rs", "def456").unwrap();
        assert_eq!(
            storage.get_file_hash("test.rs").unwrap(),
            Some("def456".to_string())
        );
    }

    #[test]
    fn test_session_memory() {
        let storage = CognitionStorage::in_memory().unwrap();

        let id = storage
            .store_session_memory("session-1", "Test session")
            .unwrap();
        assert!(id >= 0);

        let memories = storage.recall_memories(10).unwrap();
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].session_id, "session-1");
        assert_eq!(memories[0].summary, "Test session");
    }

    #[test]
    fn test_knowledge_node() {
        let storage = CognitionStorage::in_memory().unwrap();

        storage
            .store_knowledge_node("architecture", "layered", "Layered architecture", "project", None)
            .unwrap();

        let node = storage.get_knowledge_node("architecture", "layered").unwrap();
        assert!(node.is_some());
        assert_eq!(node.unwrap().content, "Layered architecture");
    }

    #[test]
    fn test_meta_storage() {
        let storage = CognitionStorage::in_memory().unwrap();

        storage.set_meta("version", "1.0").unwrap();
        assert_eq!(storage.get_meta("version").unwrap(), Some("1.0".to_string()));

        storage.set_meta("version", "2.0").unwrap();
        assert_eq!(storage.get_meta("version").unwrap(), Some("2.0".to_string()));
    }
}
