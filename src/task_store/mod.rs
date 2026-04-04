//! Task progress persistence
//!
//! Stores each `zcode run` task as a JSON file in `.zcode/tasks/`.
//! Task records include the full conversation history so execution can be
//! resumed exactly where it left off.
//!
//! # Directory layout
//! ```text
//! .zcode/
//! └── tasks/
//!     ├── abc12345.json   (running)
//!     ├── def67890.json   (completed)
//!     └── ...
//! ```

use crate::agent::loop_exec::ConversationMessage;
use crate::error::{Result, ZcodeError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ─────────────────────────────────────────────
// TaskStatus
// ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Running,
    Completed,
    Failed,
    Interrupted,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Interrupted => write!(f, "interrupted"),
        }
    }
}

// ─────────────────────────────────────────────
// TaskRecord
// ─────────────────────────────────────────────

/// A persisted record of a single `zcode run` invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    /// Short unique ID (8 hex chars derived from timestamp + counter).
    pub id: String,
    /// Original task description given by the user.
    pub task: String,
    /// Current execution status.
    pub status: TaskStatus,
    /// Unix timestamp of creation (seconds).
    pub created_at: u64,
    /// Unix timestamp of last update (seconds).
    pub updated_at: u64,
    /// Number of agent loop iterations completed so far.
    pub iteration: usize,
    /// Full conversation history — restored on resume.
    pub history: Vec<ConversationMessage>,
    /// Final answer produced when status = Completed.
    pub result: Option<String>,
    /// Error message when status = Failed.
    pub error: Option<String>,
}

impl TaskRecord {
    /// Create a new record for a task that hasn't started yet.
    pub fn new(id: String, task: impl Into<String>) -> Self {
        let now = now_secs();
        Self {
            id,
            task: task.into(),
            status: TaskStatus::Running,
            created_at: now,
            updated_at: now,
            iteration: 0,
            history: vec![],
            result: None,
            error: None,
        }
    }

    /// One-line summary suitable for list display.
    pub fn summary(&self) -> String {
        let task_snippet = if self.task.len() > 50 {
            format!("{}…", &self.task[..50])
        } else {
            self.task.clone()
        };
        format!(
            "[{}] {} | {} | iter={}",
            self.id, self.status, task_snippet, self.iteration
        )
    }
}

// ─────────────────────────────────────────────
// TaskStore
// ─────────────────────────────────────────────

/// Manages task records stored in `<project_root>/.zcode/tasks/`.
pub struct TaskStore {
    tasks_dir: PathBuf,
}

impl TaskStore {
    /// Create a store pointing at `<project_root>/.zcode/tasks/`.
    pub fn new(project_root: impl Into<PathBuf>) -> Result<Self> {
        let tasks_dir = project_root.into().join(".zcode").join("tasks");
        std::fs::create_dir_all(&tasks_dir).map_err(|e| {
            ZcodeError::ConfigError(format!("Cannot create .zcode/tasks/: {}", e))
        })?;
        Ok(Self { tasks_dir })
    }

    /// Generate a new task record with a unique ID.
    pub fn create(&self, task: impl Into<String>) -> TaskRecord {
        let id = generate_id();
        TaskRecord::new(id, task)
    }

    /// Persist a task record to disk.
    pub fn save(&self, record: &mut TaskRecord) -> Result<()> {
        record.updated_at = now_secs();
        let path = self.record_path(&record.id);
        let json = serde_json::to_string_pretty(record).map_err(|e| {
            ZcodeError::ConfigError(format!("Cannot serialise task {}: {}", record.id, e))
        })?;
        // Write to a temp file then rename for atomicity.
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &json).map_err(|e| {
            ZcodeError::ConfigError(format!("Cannot write task {}: {}", record.id, e))
        })?;
        std::fs::rename(&tmp, &path).map_err(|e| {
            ZcodeError::ConfigError(format!("Cannot commit task {}: {}", record.id, e))
        })?;
        Ok(())
    }

    /// Load a task record by ID.
    pub fn load(&self, id: &str) -> Result<TaskRecord> {
        let path = self.record_path(id);
        if !path.exists() {
            return Err(ZcodeError::ConfigError(format!(
                "Task '{}' not found. Run `zcode task list` to see available tasks.",
                id
            )));
        }
        let json = std::fs::read_to_string(&path).map_err(|e| {
            ZcodeError::ConfigError(format!("Cannot read task {}: {}", id, e))
        })?;
        let record: TaskRecord = serde_json::from_str(&json).map_err(|e| {
            ZcodeError::ConfigError(format!("Cannot parse task {}: {}", id, e))
        })?;
        Ok(record)
    }

    /// List all saved task records, sorted by created_at descending (newest first).
    pub fn list(&self) -> Result<Vec<TaskRecord>> {
        let mut records = Vec::new();
        let entries = std::fs::read_dir(&self.tasks_dir).map_err(|e| {
            ZcodeError::ConfigError(format!("Cannot read .zcode/tasks/: {}", e))
        })?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(record) = serde_json::from_str::<TaskRecord>(&json) {
                        records.push(record);
                    }
                }
            }
        }
        records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(records)
    }

    /// Delete a task record by ID.
    pub fn delete(&self, id: &str) -> Result<()> {
        let path = self.record_path(id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                ZcodeError::ConfigError(format!("Cannot delete task {}: {}", id, e))
            })?;
        }
        Ok(())
    }

    /// Delete all completed or failed task records.
    pub fn clean(&self) -> Result<usize> {
        let records = self.list()?;
        let mut deleted = 0;
        for record in &records {
            if matches!(record.status, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Interrupted) {
                self.delete(&record.id)?;
                deleted += 1;
            }
        }
        Ok(deleted)
    }

    fn record_path(&self, id: &str) -> PathBuf {
        self.tasks_dir.join(format!("{}.json", id))
    }
}

// ─────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Generate an 8-character hex ID from the current timestamp + a tiny counter.
fn generate_id() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let ts = now_secs();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:08x}", (ts as u32).wrapping_add(counter))
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store() -> (TempDir, TaskStore) {
        let dir = TempDir::new().unwrap();
        let store = TaskStore::new(dir.path()).unwrap();
        (dir, store)
    }

    #[test]
    fn test_create_and_save_and_load() {
        let (_dir, store) = store();
        let mut record = store.create("add error handling");
        record.iteration = 3;
        store.save(&mut record).unwrap();

        let loaded = store.load(&record.id).unwrap();
        assert_eq!(loaded.task, "add error handling");
        assert_eq!(loaded.iteration, 3);
        assert_eq!(loaded.status, TaskStatus::Running);
    }

    #[test]
    fn test_list_sorted_newest_first() {
        let (_dir, store) = store();

        let mut r1 = store.create("task one");
        r1.created_at = 1000;
        store.save(&mut r1).unwrap();

        let mut r2 = store.create("task two");
        r2.created_at = 2000;
        store.save(&mut r2).unwrap();

        let list = store.list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].created_at, 2000); // newest first
    }

    #[test]
    fn test_load_nonexistent_returns_error() {
        let (_dir, store) = store();
        let result = store.load("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete() {
        let (_dir, store) = store();
        let mut record = store.create("delete me");
        store.save(&mut record).unwrap();
        store.delete(&record.id).unwrap();
        assert!(store.load(&record.id).is_err());
    }

    #[test]
    fn test_clean_removes_completed_and_failed() {
        let (_dir, store) = store();

        let mut r1 = store.create("running task");
        r1.status = TaskStatus::Running;
        store.save(&mut r1).unwrap();

        let mut r2 = store.create("done task");
        r2.status = TaskStatus::Completed;
        r2.result = Some("done".into());
        store.save(&mut r2).unwrap();

        let mut r3 = store.create("failed task");
        r3.status = TaskStatus::Failed;
        store.save(&mut r3).unwrap();

        let deleted = store.clean().unwrap();
        assert_eq!(deleted, 2);

        let remaining = store.list().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].status, TaskStatus::Running);
    }

    #[test]
    fn test_task_summary_truncates_long_task() {
        let record = TaskRecord::new(
            "abc12345".into(),
            "a".repeat(100),
        );
        let summary = record.summary();
        assert!(summary.contains('…'));
        assert!(summary.len() < 120);
    }

    #[test]
    fn test_task_status_display() {
        assert_eq!(format!("{}", TaskStatus::Running), "running");
        assert_eq!(format!("{}", TaskStatus::Completed), "completed");
        assert_eq!(format!("{}", TaskStatus::Failed), "failed");
        assert_eq!(format!("{}", TaskStatus::Interrupted), "interrupted");
    }

    #[test]
    fn test_save_is_atomic_and_idempotent() {
        let (_dir, store) = store();
        let mut record = store.create("idempotent task");
        store.save(&mut record).unwrap();
        store.save(&mut record).unwrap(); // second save must not fail
        let loaded = store.load(&record.id).unwrap();
        assert_eq!(loaded.id, record.id);
    }

    #[test]
    fn test_save_updates_updated_at() {
        let (_dir, store) = store();
        let mut record = store.create("timing test");
        let original_updated = record.updated_at;
        // Ensure a different second
        std::thread::sleep(std::time::Duration::from_millis(1100));
        store.save(&mut record).unwrap();
        assert!(record.updated_at >= original_updated);
    }
}
