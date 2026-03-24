//! Session snapshot management
//!
//! Saves and restores workspace file snapshots using SQLite.

use crate::error::{Result, ZcodeError};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

// ─── Types ─────────────────────────────────────────────────────────────────────

/// A single file captured in a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub path: String,
    pub content_hash: String,
    pub content: String,
}

/// A saved workspace snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: i64,
    pub name: String,
    pub timestamp: String,
    pub file_count: usize,
    pub description: Option<String>,
}

/// Full snapshot including file contents
#[derive(Debug, Clone)]
pub struct SnapshotDetail {
    pub snapshot: Snapshot,
    pub files: Vec<FileSnapshot>,
}

// ─── SnapshotManager ───────────────────────────────────────────────────────────

/// Manages workspace snapshots in SQLite
pub struct SnapshotManager {
    conn: Connection,
    /// Root directory to snapshot
    workspace_root: PathBuf,
}

impl SnapshotManager {
    /// Create a new SnapshotManager backed by an in-memory or file database
    pub fn new(db_path: impl AsRef<Path>, workspace_root: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(db_path)
            .map_err(|e| ZcodeError::InternalError(format!("Snapshot DB error: {}", e)))?;

        let mgr = Self {
            conn,
            workspace_root: workspace_root.as_ref().to_path_buf(),
        };
        mgr.init_schema()?;
        Ok(mgr)
    }

    /// In-memory snapshot manager for tests
    pub fn in_memory(workspace_root: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        let mgr = Self {
            conn,
            workspace_root: workspace_root.as_ref().to_path_buf(),
        };
        mgr.init_schema()?;
        Ok(mgr)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS snapshots (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT NOT NULL,
                timestamp   TEXT NOT NULL DEFAULT (datetime('now')),
                description TEXT
            );
            CREATE TABLE IF NOT EXISTS snapshot_files (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id  INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                path         TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                content      TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_snapshot_files_snapshot_id
                ON snapshot_files(snapshot_id);",
        ).map_err(|e| ZcodeError::InternalError(format!("Schema init error: {}", e)))?;
        Ok(())
    }

    // ─── Save ──────────────────────────────────────────────────────────────────

    /// Save a snapshot of specific files (relative to workspace_root)
    pub fn save_files(
        &mut self,
        name: impl Into<String>,
        description: Option<&str>,
        file_paths: &[&str],
    ) -> Result<i64> {
        let name = name.into();

        // Insert snapshot record
        self.conn.execute(
            "INSERT INTO snapshots (name, description) VALUES (?1, ?2)",
            rusqlite::params![name, description],
        ).map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        let snapshot_id = self.conn.last_insert_rowid();

        // Insert each file
        for rel_path in file_paths {
            let abs_path = self.workspace_root.join(rel_path);
            if !abs_path.exists() {
                continue;
            }
            let content = std::fs::read_to_string(&abs_path)
                .unwrap_or_default();
            let hash = Self::hash_content(&content);

            self.conn.execute(
                "INSERT INTO snapshot_files (snapshot_id, path, content_hash, content)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![snapshot_id, rel_path, hash, content],
            ).map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        }

        Ok(snapshot_id)
    }

    /// Save a snapshot of all tracked text files in the workspace
    pub fn save_workspace(
        &mut self,
        name: impl Into<String>,
        description: Option<&str>,
    ) -> Result<i64> {
        let files = Self::collect_text_files(&self.workspace_root)?;
        let rel_paths: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
        self.save_files(name, description, &rel_paths)
    }

    // ─── Restore ───────────────────────────────────────────────────────────────

    /// Restore files from a snapshot to the workspace
    ///
    /// Returns a map of `path -> restored` for files that were written
    pub fn restore(&self, snapshot_id: i64) -> Result<HashMap<String, bool>> {
        let files = self.get_files(snapshot_id)?;
        let mut results = HashMap::new();

        for file in &files {
            let abs_path = self.workspace_root.join(&file.path);
            if let Some(parent) = abs_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let ok = std::fs::write(&abs_path, &file.content).is_ok();
            results.insert(file.path.clone(), ok);
        }

        Ok(results)
    }

    // ─── List / Get ────────────────────────────────────────────────────────────

    /// List all snapshots (without file contents)
    pub fn list(&self) -> Result<Vec<Snapshot>> {
        let mut stmt = self.conn
            .prepare(
                "SELECT s.id, s.name, s.timestamp, s.description,
                        COUNT(f.id) as file_count
                 FROM snapshots s
                 LEFT JOIN snapshot_files f ON f.snapshot_id = s.id
                 GROUP BY s.id
                 ORDER BY s.id DESC",
            )
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        let snapshots = stmt.query_map([], |row| {
            Ok(Snapshot {
                id: row.get(0)?,
                name: row.get(1)?,
                timestamp: row.get(2)?,
                file_count: row.get::<_, i64>(4)? as usize,
                description: row.get(3)?,
            })
        })
        .map_err(|e| ZcodeError::InternalError(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

        Ok(snapshots)
    }

    /// Get all files for a snapshot
    pub fn get_files(&self, snapshot_id: i64) -> Result<Vec<FileSnapshot>> {
        let mut stmt = self.conn
            .prepare("SELECT path, content_hash, content FROM snapshot_files WHERE snapshot_id = ?1")
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        let files = stmt.query_map(rusqlite::params![snapshot_id], |row| {
            Ok(FileSnapshot {
                path: row.get(0)?,
                content_hash: row.get(1)?,
                content: row.get(2)?,
            })
        })
        .map_err(|e| ZcodeError::InternalError(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

        Ok(files)
    }

    /// Get snapshot metadata by id
    pub fn get(&self, snapshot_id: i64) -> Result<Option<Snapshot>> {
        let result = self.conn
            .query_row(
                "SELECT s.id, s.name, s.timestamp, s.description,
                        COUNT(f.id) as file_count
                 FROM snapshots s
                 LEFT JOIN snapshot_files f ON f.snapshot_id = s.id
                 WHERE s.id = ?1
                 GROUP BY s.id",
                rusqlite::params![snapshot_id],
                |row| Ok(Snapshot {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    timestamp: row.get(2)?,
                    file_count: row.get::<_, i64>(4)? as usize,
                    description: row.get(3)?,
                }),
            )
            .optional()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        Ok(result)
    }

    // ─── Delete ────────────────────────────────────────────────────────────────

    /// Delete a snapshot and its files
    pub fn delete(&mut self, snapshot_id: i64) -> Result<bool> {
        // snapshot_files will be cascade-deleted
        let count = self.conn
            .execute("DELETE FROM snapshots WHERE id = ?1", rusqlite::params![snapshot_id])
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        Ok(count > 0)
    }

    // ─── Diff ──────────────────────────────────────────────────────────────────

    /// Compare a snapshot to the current workspace
    ///
    /// Returns `(path, snapshot_hash, current_hash)` for changed files
    pub fn diff_from_current(&self, snapshot_id: i64) -> Result<Vec<(String, String, String)>> {
        let snap_files = self.get_files(snapshot_id)?;
        let mut changes = Vec::new();

        for file in snap_files {
            let abs_path = self.workspace_root.join(&file.path);
            let current_content = std::fs::read_to_string(&abs_path).unwrap_or_default();
            let current_hash = Self::hash_content(&current_content);

            if current_hash != file.content_hash {
                changes.push((file.path, file.content_hash, current_hash));
            }
        }

        Ok(changes)
    }

    // ─── Helpers ───────────────────────────────────────────────────────────────

    fn hash_content(content: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    fn collect_text_files(root: &Path) -> Result<Vec<String>> {
        let mut files = Vec::new();
        if !root.exists() {
            return Ok(files);
        }
        Self::walk_dir(root, root, &mut files);
        Ok(files)
    }

    fn walk_dir(root: &Path, current: &Path, files: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(current) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // Skip hidden dirs and common non-source dirs
            if name.starts_with('.') || matches!(name, "target" | "node_modules" | ".git") {
                continue;
            }
            if path.is_dir() {
                Self::walk_dir(root, &path, files);
            } else if path.is_file() {
                // Only include text files by extension
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "rs" | "toml" | "json" | "yaml" | "yml" | "md" | "txt" | "py" | "js" | "ts" | "lua" | "sh") {
                    if let Ok(rel) = path.strip_prefix(root) {
                        files.push(rel.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::io::Write;

    fn setup() -> (TempDir, SnapshotManager) {
        let dir = TempDir::new().unwrap();
        let mgr = SnapshotManager::in_memory(dir.path()).unwrap();
        (dir, mgr)
    }

    fn write_file(dir: &TempDir, name: &str, content: &str) {
        std::fs::write(dir.path().join(name), content).unwrap();
    }

    #[test]
    fn test_snapshot_manager_creates_schema() {
        let (_, mgr) = setup();
        let list = mgr.list().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_save_and_list_snapshot() {
        let (dir, mut mgr) = setup();
        write_file(&dir, "hello.rs", "fn main() {}");

        let id = mgr.save_files("initial", None, &["hello.rs"]).unwrap();
        assert!(id > 0);

        let list = mgr.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "initial");
        assert_eq!(list[0].file_count, 1);
    }

    #[test]
    fn test_save_multiple_snapshots() {
        let (dir, mut mgr) = setup();
        write_file(&dir, "a.rs", "fn foo() {}");
        write_file(&dir, "b.rs", "fn bar() {}");

        mgr.save_files("snap1", Some("first"), &["a.rs"]).unwrap();
        mgr.save_files("snap2", Some("second"), &["a.rs", "b.rs"]).unwrap();

        let list = mgr.list().unwrap();
        assert_eq!(list.len(), 2);
        // Ordered by id DESC
        assert_eq!(list[0].name, "snap2");
        assert_eq!(list[0].file_count, 2);
        assert_eq!(list[1].name, "snap1");
    }

    #[test]
    fn test_get_files() {
        let (dir, mut mgr) = setup();
        write_file(&dir, "main.rs", "fn main() { println!(\"hello\"); }");
        let id = mgr.save_files("test", None, &["main.rs"]).unwrap();

        let files = mgr.get_files(id).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "main.rs");
        assert!(files[0].content.contains("println"));
    }

    #[test]
    fn test_get_snapshot_metadata() {
        let (dir, mut mgr) = setup();
        write_file(&dir, "x.rs", "");
        let id = mgr.save_files("metadata_test", Some("desc"), &["x.rs"]).unwrap();

        let snap = mgr.get(id).unwrap().unwrap();
        assert_eq!(snap.name, "metadata_test");
        assert_eq!(snap.description, Some("desc".to_string()));
        assert_eq!(snap.file_count, 1);
    }

    #[test]
    fn test_get_nonexistent_snapshot() {
        let (_, mgr) = setup();
        let snap = mgr.get(9999).unwrap();
        assert!(snap.is_none());
    }

    #[test]
    fn test_restore_snapshot() {
        let (dir, mut mgr) = setup();
        write_file(&dir, "restore_me.rs", "original content");
        let id = mgr.save_files("backup", None, &["restore_me.rs"]).unwrap();

        // Overwrite the file
        write_file(&dir, "restore_me.rs", "modified content");
        let current = std::fs::read_to_string(dir.path().join("restore_me.rs")).unwrap();
        assert_eq!(current, "modified content");

        // Restore
        let results = mgr.restore(id).unwrap();
        assert!(results["restore_me.rs"]);

        let restored = std::fs::read_to_string(dir.path().join("restore_me.rs")).unwrap();
        assert_eq!(restored, "original content");
    }

    #[test]
    fn test_delete_snapshot() {
        let (dir, mut mgr) = setup();
        write_file(&dir, "del.rs", "");
        let id = mgr.save_files("to_delete", None, &["del.rs"]).unwrap();

        assert_eq!(mgr.list().unwrap().len(), 1);
        let deleted = mgr.delete(id).unwrap();
        assert!(deleted);
        assert_eq!(mgr.list().unwrap().len(), 0);
    }

    #[test]
    fn test_delete_nonexistent_snapshot() {
        let (_, mut mgr) = setup();
        let deleted = mgr.delete(9999).unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_diff_from_current_unchanged() {
        let (dir, mut mgr) = setup();
        write_file(&dir, "same.rs", "unchanged");
        let id = mgr.save_files("base", None, &["same.rs"]).unwrap();

        let changes = mgr.diff_from_current(id).unwrap();
        assert!(changes.is_empty());
    }

    #[test]
    fn test_diff_from_current_changed() {
        let (dir, mut mgr) = setup();
        write_file(&dir, "changed.rs", "original");
        let id = mgr.save_files("base", None, &["changed.rs"]).unwrap();

        // Modify the file
        write_file(&dir, "changed.rs", "modified");
        let changes = mgr.diff_from_current(id).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].0, "changed.rs");
    }

    #[test]
    fn test_hash_content_deterministic() {
        let h1 = SnapshotManager::hash_content("hello world");
        let h2 = SnapshotManager::hash_content("hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_different_content() {
        let h1 = SnapshotManager::hash_content("hello");
        let h2 = SnapshotManager::hash_content("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_skip_nonexistent_files() {
        let (dir, mut mgr) = setup();
        // "ghost.rs" does not exist
        let id = mgr.save_files("ghost", None, &["ghost.rs"]).unwrap();
        let files = mgr.get_files(id).unwrap();
        assert!(files.is_empty()); // file skipped
    }

    #[test]
    fn test_snapshot_description() {
        let (dir, mut mgr) = setup();
        write_file(&dir, "a.rs", "");
        let id = mgr.save_files("tagged", Some("v1.0 release"), &["a.rs"]).unwrap();
        let snap = mgr.get(id).unwrap().unwrap();
        assert_eq!(snap.description.as_deref(), Some("v1.0 release"));
    }
}
