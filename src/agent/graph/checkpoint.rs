//! Checkpoint support for LangGraph-style graph execution
//!
//! Enables pause/resume of graph execution by serializing `DefaultState`
//! at any point and restoring it later.
//!
//! # Usage
//!
//! ```rust,no_run
//! use zcode::agent::graph::checkpoint::{Checkpoint, FsCheckpointStore, CheckpointStore};
//!
//! let store = FsCheckpointStore::new(".zcode/checkpoints");
//! let cp = Checkpoint::capture("my-graph", "reviewer", &state, &nodes_executed);
//! store.save(&cp)?;
//!
//! // Later ...
//! if let Some(cp) = store.load("my-graph")? {
//!     let state = cp.restore()?;
//! }
//! ```

use serde::{Deserialize, Serialize};

use crate::agent::graph::state::DefaultState;
use crate::error::{Result, ZcodeError};

// ─── Checkpoint ───────────────────────────────────────────────────────────────

/// A serializable snapshot of graph execution state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique graph execution ID (same as `CompiledGraph::graph_id`)
    pub graph_id: String,
    /// The node to resume from on restore
    pub current_node: String,
    /// Serialized `DefaultState` (JSON)
    pub state_json: String,
    /// Iteration count at checkpoint time
    pub iteration: usize,
    /// Nodes executed so far (in order)
    pub nodes_executed: Vec<String>,
    /// RFC 3339 timestamp
    pub timestamp: String,
}

impl Checkpoint {
    /// Capture a checkpoint from the current graph execution context.
    pub fn capture(
        graph_id: &str,
        current_node: &str,
        state: &DefaultState,
        nodes_executed: &[String],
    ) -> Result<Self> {
        let state_json = serde_json::to_string(state)
            .map_err(|e| ZcodeError::InternalError(format!("Failed to serialize state: {}", e)))?;
        Ok(Self {
            graph_id: graph_id.to_string(),
            current_node: current_node.to_string(),
            state_json,
            iteration: state.iteration,
            nodes_executed: nodes_executed.to_vec(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Restore the `DefaultState` from this checkpoint.
    pub fn restore(&self) -> Result<DefaultState> {
        serde_json::from_str(&self.state_json)
            .map_err(|e| ZcodeError::InternalError(format!("Failed to deserialize state: {}", e)))
    }
}

// ─── CheckpointStore trait ────────────────────────────────────────────────────

/// Pluggable storage backend for checkpoints.
pub trait CheckpointStore: Send + Sync {
    /// Persist a checkpoint (overwrites any existing one for the same `graph_id`)
    fn save(&self, checkpoint: &Checkpoint) -> Result<()>;
    /// Load the latest checkpoint for `graph_id`, or `None` if not found
    fn load(&self, graph_id: &str) -> Result<Option<Checkpoint>>;
    /// Delete the checkpoint for `graph_id`
    fn delete(&self, graph_id: &str) -> Result<()>;
}

// ─── FsCheckpointStore ───────────────────────────────────────────────────────

/// Filesystem-backed checkpoint store.
///
/// Checkpoints are stored as `{dir}/{graph_id}.json`.
pub struct FsCheckpointStore {
    dir: std::path::PathBuf,
}

impl FsCheckpointStore {
    /// Create a new store rooted at `dir`. The directory will be created
    /// automatically on the first `save` call.
    pub fn new(dir: impl Into<std::path::PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn path(&self, graph_id: &str) -> std::path::PathBuf {
        self.dir.join(format!("{}.json", graph_id))
    }
}

impl CheckpointStore for FsCheckpointStore {
    fn save(&self, checkpoint: &Checkpoint) -> Result<()> {
        std::fs::create_dir_all(&self.dir).map_err(|e| {
            ZcodeError::InternalError(format!("Failed to create checkpoint dir: {}", e))
        })?;
        let json = serde_json::to_string_pretty(checkpoint).map_err(|e| {
            ZcodeError::InternalError(format!("Failed to serialize checkpoint: {}", e))
        })?;
        std::fs::write(self.path(&checkpoint.graph_id), json).map_err(|e| {
            ZcodeError::InternalError(format!("Failed to write checkpoint: {}", e))
        })
    }

    fn load(&self, graph_id: &str) -> Result<Option<Checkpoint>> {
        let path = self.path(graph_id);
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(&path).map_err(|e| {
            ZcodeError::InternalError(format!("Failed to read checkpoint: {}", e))
        })?;
        let cp: Checkpoint = serde_json::from_str(&json).map_err(|e| {
            ZcodeError::InternalError(format!("Failed to parse checkpoint: {}", e))
        })?;
        Ok(Some(cp))
    }

    fn delete(&self, graph_id: &str) -> Result<()> {
        let path = self.path(graph_id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                ZcodeError::InternalError(format!("Failed to delete checkpoint: {}", e))
            })?;
        }
        Ok(())
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state() -> DefaultState {
        let mut s = DefaultState::default();
        s.metadata.insert("key".into(), serde_json::json!("value"));
        s.iteration = 3;
        s
    }

    #[test]
    fn test_checkpoint_capture_and_restore() {
        let state = sample_state();
        let cp = Checkpoint::capture("g1", "reviewer", &state, &["planner".into(), "coder".into()])
            .unwrap();
        assert_eq!(cp.graph_id, "g1");
        assert_eq!(cp.current_node, "reviewer");
        assert_eq!(cp.iteration, 3);
        assert_eq!(cp.nodes_executed.len(), 2);

        let restored = cp.restore().unwrap();
        assert_eq!(restored.iteration, 3);
        assert_eq!(
            restored.metadata.get("key").unwrap(),
            &serde_json::json!("value")
        );
    }

    #[test]
    fn test_fs_store_save_load_delete() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsCheckpointStore::new(dir.path());
        let state = sample_state();
        let cp = Checkpoint::capture("graph-abc", "coder", &state, &[]).unwrap();

        store.save(&cp).unwrap();

        let loaded = store.load("graph-abc").unwrap().unwrap();
        assert_eq!(loaded.graph_id, "graph-abc");
        assert_eq!(loaded.current_node, "coder");

        store.delete("graph-abc").unwrap();
        assert!(store.load("graph-abc").unwrap().is_none());
    }

    #[test]
    fn test_fs_store_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsCheckpointStore::new(dir.path());
        assert!(store.load("nonexistent").unwrap().is_none());
    }
}
