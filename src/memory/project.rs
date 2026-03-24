//! Project Memory — SQLite-backed persistent knowledge store
//!
//! Stores architecture decisions, code patterns, and code chunks
//! across sessions using rusqlite with bundled SQLite.

use crate::error::{Result, ZcodeError};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

// ─── MemoryEntry ───────────────────────────────────────────────────────────────

/// A key-value memory entry with a category label
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: i64,
    pub key: String,
    pub value: String,
    pub category: String,
    pub created_at: i64,
    pub updated_at: i64,
}

// ─── CodeChunk ─────────────────────────────────────────────────────────────────

/// A stored code chunk with optional embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub id: i64,
    pub path: String,
    pub chunk_text: String,
    /// Embedding stored as JSON array of f32
    pub embedding_json: Option<String>,
    pub created_at: i64,
}

impl CodeChunk {
    /// Deserialize the embedding from JSON
    pub fn embedding(&self) -> Option<Vec<f32>> {
        self.embedding_json
            .as_ref()
            .and_then(|j| serde_json::from_str(j).ok())
    }
}

// ─── ProjectMemory ─────────────────────────────────────────────────────────────

/// SQLite-backed project-level memory
pub struct ProjectMemory {
    conn: Connection,
}

impl ProjectMemory {
    /// Open (or create) a project memory database at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| ZcodeError::InternalError(format!("SQLite open error: {}", e)))?;
        let mut pm = Self { conn };
        pm.init_schema()?;
        Ok(pm)
    }

    /// Create an in-memory database (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| ZcodeError::InternalError(format!("SQLite in-memory error: {}", e)))?;
        let mut pm = Self { conn };
        pm.init_schema()?;
        Ok(pm)
    }

    /// Initialize database schema
    fn init_schema(&mut self) -> Result<()> {
        self.conn
            .execute_batch(
                "PRAGMA journal_mode=WAL;

                CREATE TABLE IF NOT EXISTS memories (
                    id          INTEGER PRIMARY KEY AUTOINCREMENT,
                    key         TEXT    NOT NULL,
                    value       TEXT    NOT NULL,
                    category    TEXT    NOT NULL DEFAULT 'general',
                    created_at  INTEGER NOT NULL DEFAULT (strftime('%s','now')),
                    updated_at  INTEGER NOT NULL DEFAULT (strftime('%s','now'))
                );

                CREATE UNIQUE INDEX IF NOT EXISTS idx_memories_key ON memories(key);
                CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);

                CREATE TABLE IF NOT EXISTS code_chunks (
                    id              INTEGER PRIMARY KEY AUTOINCREMENT,
                    path            TEXT    NOT NULL,
                    chunk_text      TEXT    NOT NULL,
                    embedding_json  TEXT,
                    created_at      INTEGER NOT NULL DEFAULT (strftime('%s','now'))
                );

                CREATE INDEX IF NOT EXISTS idx_chunks_path ON code_chunks(path);
                ",
            )
            .map_err(|e| ZcodeError::InternalError(format!("Schema init error: {}", e)))?;
        Ok(())
    }

    // ── Memory CRUD ──

    /// Store a key-value memory entry (upsert)
    pub fn store(
        &self,
        key: impl Into<String>,
        value: impl Into<String>,
        category: impl Into<String>,
    ) -> Result<()> {
        let key = key.into();
        let value = value.into();
        let category = category.into();
        self.conn
            .execute(
                "INSERT INTO memories (key, value, category, updated_at)
                 VALUES (?1, ?2, ?3, strftime('%s','now'))
                 ON CONFLICT(key) DO UPDATE SET
                    value      = excluded.value,
                    category   = excluded.category,
                    updated_at = strftime('%s','now')",
                params![key, value, category],
            )
            .map_err(|e| ZcodeError::InternalError(format!("Store error: {}", e)))?;
        Ok(())
    }

    /// Get a memory entry by key
    pub fn get(&self, key: &str) -> Result<Option<MemoryEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, key, value, category, created_at, updated_at
                 FROM memories WHERE key = ?1",
            )
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        let result = stmt
            .query_row(params![key], |row| {
                Ok(MemoryEntry {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    value: row.get(2)?,
                    category: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .optional()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        Ok(result)
    }

    /// Delete a memory entry by key
    pub fn delete(&self, key: &str) -> Result<bool> {
        let n = self
            .conn
            .execute("DELETE FROM memories WHERE key = ?1", params![key])
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        Ok(n > 0)
    }

    /// Get all memory entries in a category
    pub fn search_by_category(&self, category: &str) -> Result<Vec<MemoryEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, key, value, category, created_at, updated_at
                 FROM memories WHERE category = ?1
                 ORDER BY updated_at DESC",
            )
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        let rows = stmt
            .query_map(params![category], |row| {
                Ok(MemoryEntry {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    value: row.get(2)?,
                    category: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))
    }

    /// List all memory entries (most recently updated first)
    pub fn list_all(&self, limit: usize) -> Result<Vec<MemoryEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, key, value, category, created_at, updated_at
                 FROM memories ORDER BY updated_at DESC LIMIT ?1",
            )
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(MemoryEntry {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    value: row.get(2)?,
                    category: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))
    }

    /// Count total memory entries
    pub fn count_memories(&self) -> Result<usize> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        Ok(n as usize)
    }

    // ── Code Chunks ──

    /// Store a code chunk with optional embedding
    pub fn store_code_chunk(
        &self,
        path: impl Into<String>,
        chunk_text: impl Into<String>,
        embedding: Option<&[f32]>,
    ) -> Result<i64> {
        let path = path.into();
        let text = chunk_text.into();
        let embedding_json = embedding.map(|e| serde_json::to_string(e).unwrap_or_default());

        self.conn
            .execute(
                "INSERT INTO code_chunks (path, chunk_text, embedding_json) VALUES (?1, ?2, ?3)",
                params![path, text, embedding_json],
            )
            .map_err(|e| ZcodeError::InternalError(format!("Store chunk error: {}", e)))?;

        let id = self.conn.last_insert_rowid();
        Ok(id)
    }

    /// Get all code chunks for a file path
    pub fn get_chunks_for_path(&self, path: &str) -> Result<Vec<CodeChunk>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, path, chunk_text, embedding_json, created_at
                 FROM code_chunks WHERE path = ?1 ORDER BY id",
            )
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        let rows = stmt
            .query_map(params![path], |row| {
                Ok(CodeChunk {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    chunk_text: row.get(2)?,
                    embedding_json: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))
    }

    /// Find code chunks with embeddings for semantic search
    pub fn get_all_chunks_with_embeddings(&self) -> Result<Vec<CodeChunk>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, path, chunk_text, embedding_json, created_at
                 FROM code_chunks WHERE embedding_json IS NOT NULL",
            )
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(CodeChunk {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    chunk_text: row.get(2)?,
                    embedding_json: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))
    }

    /// Delete all code chunks for a file path
    pub fn delete_chunks_for_path(&self, path: &str) -> Result<usize> {
        let n = self
            .conn
            .execute(
                "DELETE FROM code_chunks WHERE path = ?1",
                params![path],
            )
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        Ok(n)
    }

    /// Count total code chunks
    pub fn count_chunks(&self) -> Result<usize> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM code_chunks", [], |row| row.get(0))
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        Ok(n as usize)
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_db() -> ProjectMemory {
        ProjectMemory::in_memory().unwrap()
    }

    #[test]
    fn test_in_memory_open() {
        let db = make_db();
        assert_eq!(db.count_memories().unwrap(), 0);
    }

    #[test]
    fn test_store_and_get() {
        let db = make_db();
        db.store("arch/overview", "Layered architecture with tools layer", "architecture").unwrap();
        let entry = db.get("arch/overview").unwrap().unwrap();
        assert_eq!(entry.key, "arch/overview");
        assert_eq!(entry.value, "Layered architecture with tools layer");
        assert_eq!(entry.category, "architecture");
    }

    #[test]
    fn test_store_upsert() {
        let db = make_db();
        db.store("key1", "value1", "test").unwrap();
        db.store("key1", "updated_value", "test").unwrap();
        let entry = db.get("key1").unwrap().unwrap();
        assert_eq!(entry.value, "updated_value");
        assert_eq!(db.count_memories().unwrap(), 1);
    }

    #[test]
    fn test_get_nonexistent() {
        let db = make_db();
        assert!(db.get("nonexistent_key").unwrap().is_none());
    }

    #[test]
    fn test_delete() {
        let db = make_db();
        db.store("to_delete", "val", "test").unwrap();
        assert!(db.delete("to_delete").unwrap());
        assert!(db.get("to_delete").unwrap().is_none());
        assert!(!db.delete("to_delete").unwrap()); // already gone
    }

    #[test]
    fn test_search_by_category() {
        let db = make_db();
        db.store("arch/1", "Decision 1", "architecture").unwrap();
        db.store("arch/2", "Decision 2", "architecture").unwrap();
        db.store("bug/1", "Bug 1", "bugs").unwrap();

        let arch = db.search_by_category("architecture").unwrap();
        assert_eq!(arch.len(), 2);
        assert!(arch.iter().all(|e| e.category == "architecture"));

        let bugs = db.search_by_category("bugs").unwrap();
        assert_eq!(bugs.len(), 1);

        let empty = db.search_by_category("nonexistent").unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_list_all() {
        let db = make_db();
        for i in 0..5 {
            db.store(format!("key{}", i), format!("val{}", i), "test").unwrap();
        }
        let all = db.list_all(10).unwrap();
        assert_eq!(all.len(), 5);
        let limited = db.list_all(3).unwrap();
        assert_eq!(limited.len(), 3);
    }

    #[test]
    fn test_count_memories() {
        let db = make_db();
        assert_eq!(db.count_memories().unwrap(), 0);
        db.store("k1", "v1", "c1").unwrap();
        db.store("k2", "v2", "c2").unwrap();
        assert_eq!(db.count_memories().unwrap(), 2);
    }

    #[test]
    fn test_store_code_chunk() {
        let db = make_db();
        let id = db.store_code_chunk("src/main.rs", "fn main() {}", None).unwrap();
        assert!(id > 0);
        assert_eq!(db.count_chunks().unwrap(), 1);
    }

    #[test]
    fn test_store_code_chunk_with_embedding() {
        let db = make_db();
        let embedding = vec![0.1_f32, 0.2, 0.3];
        let id = db.store_code_chunk("src/lib.rs", "pub mod foo;", Some(&embedding)).unwrap();
        let chunks = db.get_chunks_for_path("src/lib.rs").unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].id, id);
        let emb = chunks[0].embedding().unwrap();
        assert!((emb[0] - 0.1_f32).abs() < 1e-5);
    }

    #[test]
    fn test_get_chunks_for_path_empty() {
        let db = make_db();
        let chunks = db.get_chunks_for_path("nonexistent.rs").unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_get_all_chunks_with_embeddings() {
        let db = make_db();
        db.store_code_chunk("a.rs", "code a", Some(&[0.1, 0.2])).unwrap();
        db.store_code_chunk("b.rs", "code b", None).unwrap(); // no embedding
        let with_emb = db.get_all_chunks_with_embeddings().unwrap();
        assert_eq!(with_emb.len(), 1);
        assert_eq!(with_emb[0].path, "a.rs");
    }

    #[test]
    fn test_delete_chunks_for_path() {
        let db = make_db();
        db.store_code_chunk("target.rs", "chunk 1", None).unwrap();
        db.store_code_chunk("target.rs", "chunk 2", None).unwrap();
        db.store_code_chunk("other.rs", "chunk 3", None).unwrap();
        let deleted = db.delete_chunks_for_path("target.rs").unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(db.count_chunks().unwrap(), 1);
    }
}
