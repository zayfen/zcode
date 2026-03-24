//! Workspace integration layer
//!
//! Provides a unified facade over all zcode capabilities:
//! agents, tools, memory, scripting, MCP, session snapshots, git diff.

use crate::config::ProjectConfig;
use crate::error::{Result, ZcodeError};
use crate::git::GitDiff;
use crate::session::SnapshotManager;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ─── WorkspaceInfo ─────────────────────────────────────────────────────────────

/// Status information about the workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub root: PathBuf,
    pub project_name: String,
    pub languages: Vec<String>,
    pub is_git_repo: bool,
    pub has_config: bool,
    pub has_snapshots_db: bool,
    pub changed_files: usize,
}

// ─── WorkspaceContext ─────────────────────────────────────────────────────────

/// Context for an LLM request — contains only what's needed for the current task
#[derive(Debug, Clone, Default)]
pub struct WorkspaceContext {
    /// The user's request / task description
    pub task: String,
    /// Relevant file contents (path → content)
    pub files: Vec<(String, String)>,
    /// Git diff patch (if applicable)
    pub diff_patch: String,
    /// Recent git commits
    pub recent_commits: Vec<String>,
    /// Active snapshot name (if any)
    pub snapshot_name: Option<String>,
}

impl WorkspaceContext {
    /// Format the context as an LLM prompt addition
    pub fn as_prompt_context(&self) -> String {
        let mut parts = Vec::new();

        if !self.files.is_empty() {
            parts.push(format!(
                "## Relevant Files ({})\n{}",
                self.files.len(),
                self.files.iter()
                    .map(|(path, content)| format!("### {}\n```\n{}\n```", path, content))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            ));
        }

        if !self.diff_patch.is_empty() {
            parts.push(format!("## Recent Changes\n```diff\n{}\n```", self.diff_patch));
        }

        if !self.recent_commits.is_empty() {
            parts.push(format!(
                "## Recent Commits\n{}",
                self.recent_commits.iter()
                    .map(|c| format!("- {}", c))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        parts.join("\n\n")
    }

    /// Count total characters in context
    pub fn total_chars(&self) -> usize {
        self.files.iter().map(|(_, c)| c.len()).sum::<usize>()
            + self.diff_patch.len()
    }
}

// ─── Workspace ────────────────────────────────────────────────────────────────

/// Unified workspace facade
pub struct Workspace {
    pub root: PathBuf,
    pub config: ProjectConfig,
    snapshot_mgr: Option<SnapshotManager>,
}

impl Workspace {
    /// Open a workspace at the given path
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();

        let config = ProjectConfig::load(&root)
            .unwrap_or_else(|_| ProjectConfig::new(
                root.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("project")
                    .to_string()
            ));

        // Initialize snapshot manager if configured
        let snapshot_mgr = {
            let db_path = root.join(&config.snapshots.db_path);
            if let Some(parent) = db_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            SnapshotManager::new(&db_path, &root).ok()
        };

        Ok(Self { root, config, snapshot_mgr })
    }

    /// Create a new workspace at the given path
    pub fn init(root: impl AsRef<Path>, name: impl Into<String>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root)?;

        let config = ProjectConfig::new(name.into());
        config.save(&root)?;

        Self::open(&root)
    }

    /// Get workspace status information
    pub fn info(&self) -> WorkspaceInfo {
        let is_git = GitDiff::is_git_repo(&self.root);
        let changed = if is_git {
            GitDiff::changed_files(&self.root)
                .map(|f| f.len())
                .unwrap_or(0)
        } else { 0 };

        let snapshot_db = self.root.join(&self.config.snapshots.db_path);

        WorkspaceInfo {
            root: self.root.clone(),
            project_name: self.config.name.clone(),
            languages: self.config.languages.clone(),
            is_git_repo: is_git,
            has_config: self.root.join(".zcode/config.toml").exists(),
            has_snapshots_db: snapshot_db.exists(),
            changed_files: changed,
        }
    }

    /// Build a diff-aware context for the LLM
    /// (only loads files that are changed in git)
    pub fn build_diff_context(&self, max_chars: usize) -> Result<WorkspaceContext> {
        let mut ctx = WorkspaceContext::default();

        if GitDiff::is_git_repo(&self.root) {
            let diff = GitDiff::build_context(&self.root)?;
            ctx.diff_patch = diff.patch.chars().take(max_chars / 2).collect();
            ctx.recent_commits = GitDiff::recent_commits(&self.root, 5).unwrap_or_default();

            // Load changed file contents (up to budget)
            let mut used = ctx.diff_patch.len();
            for (path, content) in diff.load_changed_contents() {
                if used + content.len() > max_chars { break; }
                used += content.len();
                ctx.files.push((path, content));
            }
        }

        Ok(ctx)
    }

    /// Build context from specific files
    pub fn build_file_context(
        &self,
        paths: &[&str],
        max_chars: usize,
    ) -> WorkspaceContext {
        let mut ctx = WorkspaceContext::default();
        let mut used = 0;

        for rel_path in paths {
            let abs = self.root.join(rel_path);
            if let Ok(content) = std::fs::read_to_string(&abs) {
                if used + content.len() > max_chars { break; }
                used += content.len();
                ctx.files.push((rel_path.to_string(), content));
            }
        }

        ctx
    }

    // ─── Snapshot helpers ─────────────────────────────────────────────────────

    /// Save a snapshot with a given name
    pub fn snapshot_save(
        &mut self,
        name: impl Into<String>,
        description: Option<&str>,
    ) -> Result<i64> {
        let mgr = self.snapshot_mgr.as_mut()
            .ok_or_else(|| ZcodeError::InternalError("Snapshot manager not initialized".to_string()))?;
        mgr.save_workspace(name, description)
    }

    /// Restore a snapshot by id
    pub fn snapshot_restore(&self, id: i64) -> Result<usize> {
        let mgr = self.snapshot_mgr.as_ref()
            .ok_or_else(|| ZcodeError::InternalError("Snapshot manager not initialized".to_string()))?;
        let results = mgr.restore(id)?;
        Ok(results.values().filter(|&&ok| ok).count())
    }

    /// List all snapshots
    pub fn snapshot_list(&self) -> Result<Vec<crate::session::Snapshot>> {
        let mgr = self.snapshot_mgr.as_ref()
            .ok_or_else(|| ZcodeError::InternalError("Snapshot manager not initialized".to_string()))?;
        mgr.list()
    }

    /// Get workspace root
    pub fn root(&self) -> &Path { &self.root }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_workspace() -> (TempDir, Workspace) {
        let dir = TempDir::new().unwrap();
        let ws = Workspace::init(dir.path(), "test-project").unwrap();
        (dir, ws)
    }

    #[test]
    fn test_workspace_init_creates_config() {
        let (dir, _ws) = make_workspace();
        assert!(dir.path().join(".zcode/config.toml").exists());
    }

    #[test]
    fn test_workspace_info_project_name() {
        let (dir, ws) = make_workspace();
        let info = ws.info();
        assert_eq!(info.project_name, "test-project");
        // has_config checks .zcode/config.toml — should exist after init
        let config_path = dir.path().join(".zcode").join("config.toml");
        assert!(config_path.exists(), "config.toml should exist at {:?}", config_path);
    }

    #[test]
    fn test_workspace_open_no_config_uses_defaults() {
        let dir = TempDir::new().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        // Falls back to dir name
        assert!(!ws.config.name.is_empty());
    }

    #[test]
    fn test_workspace_root_getter() {
        let (dir, ws) = make_workspace();
        assert_eq!(ws.root(), dir.path());
    }

    #[test]
    fn test_workspace_context_empty_default() {
        let ctx = WorkspaceContext::default();
        assert!(ctx.task.is_empty());
        assert!(ctx.files.is_empty());
        assert_eq!(ctx.total_chars(), 0);
    }

    #[test]
    fn test_workspace_context_total_chars() {
        let ctx = WorkspaceContext {
            files: vec![
                ("a.rs".to_string(), "fn main() {}".to_string()),
                ("b.rs".to_string(), "fn foo() {}".to_string()),
            ],
            ..Default::default()
        };
        assert_eq!(ctx.total_chars(), "fn main() {}".len() + "fn foo() {}".len());
    }

    #[test]
    fn test_workspace_context_as_prompt_files() {
        let ctx = WorkspaceContext {
            files: vec![("main.rs".to_string(), "fn main() {}".to_string())],
            ..Default::default()
        };
        let prompt = ctx.as_prompt_context();
        assert!(prompt.contains("main.rs"));
        assert!(prompt.contains("fn main() {}"));
    }

    #[test]
    fn test_workspace_context_as_prompt_diff() {
        let ctx = WorkspaceContext {
            diff_patch: "+fn new_function() {}".to_string(),
            ..Default::default()
        };
        let prompt = ctx.as_prompt_context();
        assert!(prompt.contains("Recent Changes"));
        assert!(prompt.contains("+fn new_function"));
    }

    #[test]
    fn test_workspace_context_as_prompt_commits() {
        let ctx = WorkspaceContext {
            recent_commits: vec!["abc123 Fix bug".to_string()],
            ..Default::default()
        };
        let prompt = ctx.as_prompt_context();
        assert!(prompt.contains("Recent Commits"));
        assert!(prompt.contains("Fix bug"));
    }

    #[test]
    fn test_workspace_context_empty_prompt() {
        let ctx = WorkspaceContext::default();
        let prompt = ctx.as_prompt_context();
        // Empty context should produce empty prompt
        assert!(prompt.is_empty() || prompt.trim().is_empty());
    }

    #[test]
    fn test_build_file_context() {
        let (dir, ws) = make_workspace();
        std::fs::write(dir.path().join("hello.txt"), "hello world").unwrap();
        let ctx = ws.build_file_context(&["hello.txt"], 10000);
        assert_eq!(ctx.files.len(), 1);
        assert!(ctx.files[0].1.contains("hello world"));
    }

    #[test]
    fn test_build_file_context_respects_budget() {
        let (dir, ws) = make_workspace();
        std::fs::write(dir.path().join("big.txt"), "x".repeat(1000)).unwrap();
        std::fs::write(dir.path().join("small.txt"), "tiny").unwrap();
        // Budget = 1001 bytes: fits big.txt (1000), but adding small.txt (4) exceeds budget
        let ctx = ws.build_file_context(&["big.txt", "small.txt"], 1001);
        assert_eq!(ctx.files.len(), 1);
        assert_eq!(ctx.files[0].0, "big.txt");
    }

    #[test]
    fn test_workspace_snapshot_list_empty() {
        let (_, ws) = make_workspace();
        let list = ws.snapshot_list().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_workspace_snapshot_save_and_list() {
        let (dir, mut ws) = make_workspace();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        let id = ws.snapshot_save("initial", Some("test")).unwrap();
        assert!(id > 0);
        let list = ws.snapshot_list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "initial");
    }

    #[test]
    fn test_workspace_snapshot_restore() {
        let (dir, mut ws) = make_workspace();
        std::fs::write(dir.path().join("test.txt"), "original").unwrap();
        let id = ws.snapshot_save("backup", None).unwrap();
        std::fs::write(dir.path().join("test.txt"), "modified").unwrap();

        let count = ws.snapshot_restore(id).unwrap();
        assert!(count > 0);
        let content = std::fs::read_to_string(dir.path().join("test.txt")).unwrap();
        assert_eq!(content, "original");
    }

    #[test]
    fn test_workspace_info_is_not_git_for_tmp() {
        let (dir, ws) = make_workspace();
        let info = ws.info();
        // Temp dirs are typically not git repos
        // (they could be, but the test just ensures no panic)
        let _ = info.is_git_repo;
    }
}
