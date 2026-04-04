//! Git diff-aware context building
//!
//! Uses git subprocess to identify changed files, so we only load relevant
//! files into the LLM context — saving tokens.

use crate::error::{Result, ZcodeError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

// ─── Types ─────────────────────────────────────────────────────────────────────

/// The diff status of a single file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed { from: String },
    Untracked,
}

/// A changed file with its diff status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedFile {
    pub path: String,
    pub status: FileStatus,
}

/// Full diff output for context building
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiffContext {
    /// Files changed since HEAD
    pub changed_files: Vec<ChangedFile>,
    /// Files staged for commit
    pub staged_files: Vec<ChangedFile>,
    /// Actual diff content (patch text) for changed files
    pub patch: String,
    /// Repository root
    pub repo_root: PathBuf,
}

impl DiffContext {
    /// Load the content of all changed files (for LLM context)
    pub fn load_changed_contents(&self) -> Vec<(String, String)> {
        self.changed_files
            .iter()
            .filter(|f| f.status != FileStatus::Deleted)
            .map(|f| {
                let abs = self.repo_root.join(&f.path);
                let content = std::fs::read_to_string(&abs).unwrap_or_default();
                (f.path.clone(), content)
            })
            .collect()
    }
}

// ─── GitDiff ───────────────────────────────────────────────────────────────────

/// Git diff utilities — all operations via git subprocess
pub struct GitDiff;

impl GitDiff {
    /// Check if a path is inside a git repository
    pub fn is_git_repo(path: impl AsRef<Path>) -> bool {
        Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .current_dir(path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Find the git repository root for the given path
    pub fn repo_root(path: impl AsRef<Path>) -> Result<PathBuf> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(path)
            .output()
            .map_err(|e| ZcodeError::InternalError(format!("git error: {}", e)))?;

        if !output.status.success() {
            return Err(ZcodeError::InternalError("Not a git repository".to_string()));
        }

        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(PathBuf::from(root))
    }

    /// List files changed since HEAD (unstaged + staged)
    pub fn changed_files(repo_path: impl AsRef<Path>) -> Result<Vec<ChangedFile>> {
        let root = repo_path.as_ref();
        let mut files = Vec::new();

        // Unstaged changes
        let output = Command::new("git")
            .args(["diff", "--name-status"])
            .current_dir(root)
            .output()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        files.extend(Self::parse_name_status(&String::from_utf8_lossy(&output.stdout)));

        // Staged changes
        let output = Command::new("git")
            .args(["diff", "--name-status", "--cached"])
            .current_dir(root)
            .output()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        let staged = Self::parse_name_status(&String::from_utf8_lossy(&output.stdout));

        // Merge, deduplicating by path
        for sf in staged {
            if !files.iter().any(|f: &ChangedFile| f.path == sf.path) {
                files.push(sf);
            }
        }

        Ok(files)
    }

    /// List only staged files
    pub fn staged_files(repo_path: impl AsRef<Path>) -> Result<Vec<ChangedFile>> {
        let output = Command::new("git")
            .args(["diff", "--name-status", "--cached"])
            .current_dir(repo_path)
            .output()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        Ok(Self::parse_name_status(&String::from_utf8_lossy(&output.stdout)))
    }

    /// List untracked files
    pub fn untracked_files(repo_path: impl AsRef<Path>) -> Result<Vec<String>> {
        let output = Command::new("git")
            .args(["ls-files", "--others", "--exclude-standard"])
            .current_dir(repo_path)
            .output()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect();
        Ok(files)
    }

    /// Get the diff patch for a specific file
    pub fn file_diff(repo_path: impl AsRef<Path>, file_path: &str) -> Result<String> {
        let output = Command::new("git")
            .args(["diff", "HEAD", "--", file_path])
            .current_dir(&repo_path)
            .output()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get the full diff patch for all changes
    pub fn full_diff(repo_path: impl AsRef<Path>) -> Result<String> {
        let output = Command::new("git")
            .args(["diff", "HEAD"])
            .current_dir(&repo_path)
            .output()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get recent commit log messages (for context)
    pub fn recent_commits(repo_path: impl AsRef<Path>, count: usize) -> Result<Vec<String>> {
        let output = Command::new("git")
            .args(["log", &format!("-{}", count), "--oneline"])
            .current_dir(&repo_path)
            .output()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        let commits = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect();
        Ok(commits)
    }

    /// Build a DiffContext for an LLM session
    pub fn build_context(repo_path: impl AsRef<Path>) -> Result<DiffContext> {
        let root = repo_path.as_ref().to_path_buf();
        let changed_files = Self::changed_files(&root).unwrap_or_default();
        let staged_files = Self::staged_files(&root).unwrap_or_default();
        let patch = Self::full_diff(&root).unwrap_or_default();

        Ok(DiffContext { changed_files, staged_files, patch, repo_root: root })
    }

    // ─── Parsing ─────────────────────────────────────────────────────────────

    fn parse_name_status(output: &str) -> Vec<ChangedFile> {
        output
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|line| {
                let mut parts = line.splitn(3, '\t');
                let status_char = parts.next()?.trim_start_matches('R');
                let path = parts.next()?.to_string();
                let renamed_from = parts.next().map(str::to_string);

                let status = match status_char.chars().next()? {
                    'A' => FileStatus::Added,
                    'M' => FileStatus::Modified,
                    'D' => FileStatus::Deleted,
                    'R' => FileStatus::Renamed {
                        from: renamed_from.unwrap_or_default(),
                    },
                    _ => FileStatus::Modified,
                };

                Some(ChangedFile { path, status })
            })
            .collect()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_name_status_added() {
        let output = "A\tsrc/new_file.rs\n";
        let files = GitDiff::parse_name_status(output);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/new_file.rs");
        assert_eq!(files[0].status, FileStatus::Added);
    }

    #[test]
    fn test_parse_name_status_modified() {
        let output = "M\tsrc/main.rs\n";
        let files = GitDiff::parse_name_status(output);
        assert_eq!(files[0].status, FileStatus::Modified);
        assert_eq!(files[0].path, "src/main.rs");
    }

    #[test]
    fn test_parse_name_status_deleted() {
        let output = "D\told_file.rs\n";
        let files = GitDiff::parse_name_status(output);
        assert_eq!(files[0].status, FileStatus::Deleted);
    }

    #[test]
    fn test_parse_name_status_multiple() {
        let output = "M\tsrc/a.rs\nA\tsrc/b.rs\nD\tsrc/c.rs\n";
        let files = GitDiff::parse_name_status(output);
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_parse_name_status_empty() {
        let files = GitDiff::parse_name_status("");
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_name_status_no_changes() {
        let output = "\n\n";
        let files = GitDiff::parse_name_status(output);
        assert!(files.is_empty());
    }

    #[test]
    fn test_is_git_repo_project_root() {
        // zcode itself is a git repo
        let result = GitDiff::is_git_repo("/Users/riven/Github/zcode");
        assert!(result);
    }

    #[test]
    fn test_is_git_repo_tmp() {
        // /tmp likely isn't a git repo
        let result = GitDiff::is_git_repo("/tmp");
        // We don't assert a specific value, but it shouldn't panic
        let _ = result;
    }

    #[test]
    fn test_repo_root_project() {
        // zcode project should have a repo root
        if GitDiff::is_git_repo("/Users/riven/Github/zcode") {
            let root = GitDiff::repo_root("/Users/riven/Github/zcode");
            assert!(root.is_ok());
            assert!(root.unwrap().ends_with("zcode"));
        }
    }

    #[test]
    fn test_changed_files_returns_vec() {
        if GitDiff::is_git_repo("/Users/riven/Github/zcode") {
            let files = GitDiff::changed_files("/Users/riven/Github/zcode");
            assert!(files.is_ok());
            // May be empty or have files — just ensure no panic
        }
    }

    #[test]
    fn test_full_diff_returns_string() {
        if GitDiff::is_git_repo("/Users/riven/Github/zcode") {
            let patch = GitDiff::full_diff("/Users/riven/Github/zcode");
            assert!(patch.is_ok());
            // Patch may be empty if there are no changes
        }
    }

    #[test]
    fn test_recent_commits() {
        if GitDiff::is_git_repo("/Users/riven/Github/zcode") {
            let commits = GitDiff::recent_commits("/Users/riven/Github/zcode", 3);
            assert!(commits.is_ok());
            let commits = commits.unwrap();
            assert!(commits.len() <= 3);
        }
    }

    #[test]
    fn test_diff_context_build() {
        if GitDiff::is_git_repo("/Users/riven/Github/zcode") {
            let ctx = GitDiff::build_context("/Users/riven/Github/zcode");
            assert!(ctx.is_ok());
        }
    }

    #[test]
    fn test_diff_context_load_changed_contents() {
        let ctx = DiffContext {
            changed_files: vec![ChangedFile {
                path: "Cargo.toml".to_string(),
                status: FileStatus::Modified,
            }],
            staged_files: vec![],
            patch: String::new(),
            repo_root: PathBuf::from("/Users/riven/Github/zcode"),
        };
        let contents = ctx.load_changed_contents();
        assert_eq!(contents.len(), 1);
        assert!(!contents[0].1.is_empty()); // Cargo.toml has content
    }

    #[test]
    fn test_diff_context_skip_deleted_files() {
        let ctx = DiffContext {
            changed_files: vec![ChangedFile {
                path: "deleted_file.rs".to_string(),
                status: FileStatus::Deleted,
            }],
            staged_files: vec![],
            patch: String::new(),
            repo_root: PathBuf::from("/tmp"),
        };
        let contents = ctx.load_changed_contents();
        assert!(contents.is_empty());
    }

    #[test]
    fn test_file_status_equality() {
        assert_eq!(FileStatus::Added, FileStatus::Added);
        assert_ne!(FileStatus::Added, FileStatus::Deleted);
        assert_eq!(
            FileStatus::Renamed { from: "old.rs".to_string() },
            FileStatus::Renamed { from: "old.rs".to_string() }
        );
    }
}
