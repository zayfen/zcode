//! Pipeline state persistence for crash recovery

use crate::error::{Result, ZcodeError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Persistent pipeline state for crash recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    /// Unique ID for this pipeline run
    pub id: String,
    /// Original requirement
    pub requirement: String,
    /// Project root path
    pub project_root: String,
    /// Completed phase names
    pub completed_phases: Vec<String>,
    /// Current phase name
    pub current_phase: Option<String>,
    /// Serialized phase outputs
    pub phase_contexts: HashMap<String, serde_json::Value>,
    /// Retry iteration
    pub retry_iteration: u32,
    /// Creation timestamp
    pub created_at: i64,
    /// Last update timestamp
    pub updated_at: i64,
}

impl PipelineState {
    /// Create a new pipeline state
    pub fn new(id: &str, requirement: &str, project_root: &Path) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: id.to_string(),
            requirement: requirement.to_string(),
            project_root: project_root.to_string_lossy().to_string(),
            completed_phases: Vec::new(),
            current_phase: None,
            phase_contexts: HashMap::new(),
            retry_iteration: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Save state to `.zcode/pipeline-state.json`
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let dir = project_root.join(".zcode");
        std::fs::create_dir_all(&dir).map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        let path = dir.join("pipeline-state.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        std::fs::write(&path, json).map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        Ok(())
    }

    /// Load state from `.zcode/pipeline-state.json`
    pub fn load(project_root: &Path) -> Result<Option<Self>> {
        let path = project_root.join(".zcode").join("pipeline-state.json");
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(&path).map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        let state: PipelineState =
            serde_json::from_str(&json).map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        Ok(Some(state))
    }

    /// Clear persisted state (after successful completion)
    pub fn clear(project_root: &Path) -> Result<()> {
        let path = project_root.join(".zcode").join("pipeline-state.json");
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        }
        Ok(())
    }

    /// Mark a phase as completed and save
    pub fn complete_phase(&mut self, phase_name: &str, project_root: &Path) -> Result<()> {
        if !self.completed_phases.contains(&phase_name.to_string()) {
            self.completed_phases.push(phase_name.to_string());
        }
        self.current_phase = None;
        self.updated_at = chrono::Utc::now().timestamp();
        self.save(project_root)
    }

    /// Set the current phase and save
    pub fn set_current_phase(&mut self, phase_name: &str, project_root: &Path) -> Result<()> {
        self.current_phase = Some(phase_name.to_string());
        self.updated_at = chrono::Utc::now().timestamp();
        self.save(project_root)
    }

    /// Whether a phase has been completed
    pub fn is_phase_completed(&self, phase_name: &str) -> bool {
        self.completed_phases.contains(&phase_name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_state_new() {
        let s = PipelineState::new("id1", "Add auth", Path::new("/project"));
        assert_eq!(s.id, "id1");
        assert_eq!(s.requirement, "Add auth");
        assert!(s.completed_phases.is_empty());
        assert!(s.current_phase.is_none());
    }

    #[test]
    fn test_state_save_load_clear() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut s = PipelineState::new("test", "requirement", root);
        s.save(root).unwrap();

        let loaded = PipelineState::load(root).unwrap().unwrap();
        assert_eq!(loaded.id, "test");
        assert_eq!(loaded.requirement, "requirement");

        PipelineState::clear(root).unwrap();
        assert!(PipelineState::load(root).unwrap().is_none());
    }

    #[test]
    fn test_state_complete_phase() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut s = PipelineState::new("test", "req", root);

        s.complete_phase("cognition", root).unwrap();
        assert!(s.is_phase_completed("cognition"));
        assert!(!s.is_phase_completed("planning"));

        let loaded = PipelineState::load(root).unwrap().unwrap();
        assert!(loaded.is_phase_completed("cognition"));
    }

    #[test]
    fn test_state_set_current_phase() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut s = PipelineState::new("test", "req", root);

        s.set_current_phase("execution", root).unwrap();
        assert_eq!(s.current_phase, Some("execution".to_string()));
    }

    #[test]
    fn test_load_no_file() {
        let dir = TempDir::new().unwrap();
        assert!(PipelineState::load(dir.path()).unwrap().is_none());
    }

    #[test]
    fn test_clear_no_file() {
        let dir = TempDir::new().unwrap();
        // Should not error even if file doesn't exist
        PipelineState::clear(dir.path()).unwrap();
    }
}
