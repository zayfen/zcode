//! Delivery pipeline — orchestrates gate checks, changelog, branch/commit, push, PR, and CI

use crate::delivery::changelog::{ChangelogGenerator, TaskRecord};
use crate::delivery::ci_monitor::{CiMonitor, CiStatus};
use crate::delivery::config::DeliveryConfig;
use crate::delivery::gate::GateChecker;
use crate::delivery::pull_request::{PullRequestCreator, PrOptions};
use crate::error::{Result, ZcodeError};
use crate::git::DiffContext;
use chrono::{DateTime, Utc};
use std::path::Path;
use tokio::process::Command;

/// Result of a complete delivery pipeline run
#[derive(Debug, Clone)]
pub struct DeliveryResult {
    /// Branch name that was created/pushed
    pub branch: String,
    /// PR URL (if a PR was created)
    pub pr_url: Option<String>,
    /// Generated changelog content
    pub changelog: String,
    /// Version string (if version was bumped)
    pub version: Option<String>,
    /// CI status
    pub ci_status: Option<CiStatus>,
    /// Timestamp of delivery
    pub delivered_at: DateTime<Utc>,
}

/// Context needed to run a delivery pipeline
#[derive(Debug, Clone)]
pub struct DeliveryContext {
    /// Task records for changelog and PR body
    pub tasks: Vec<TaskRecord>,
    /// Git commit messages since branching
    pub commits: Vec<String>,
    /// Diff context for changed files
    pub diff: DiffContext,
    /// Verification scores per task (task_name, score)
    pub scores: Vec<(String, f64)>,
    /// Project root path
    pub project_root: std::path::PathBuf,
    /// Optional branch name override (if None, generated from template)
    pub branch_name: Option<String>,
    /// Optional commit message override
    pub commit_message: Option<String>,
}

/// Main delivery pipeline orchestrator
pub struct DeliveryPipeline {
    config: DeliveryConfig,
}

impl DeliveryPipeline {
    /// Create a new delivery pipeline with the given configuration
    pub fn new(config: DeliveryConfig) -> Self {
        Self { config }
    }

    /// Get a reference to the configuration
    pub fn config(&self) -> &DeliveryConfig {
        &self.config
    }

    /// Run the full delivery pipeline:
    /// 1. Gate checks
    /// 2. Changelog generation
    /// 3. Branch creation + commit
    /// 4. Push
    /// 5. PR creation
    /// 6. CI monitoring
    pub async fn deliver(&self, ctx: &DeliveryContext) -> Result<DeliveryResult> {
        // Step 1: Gate checks
        let gate_results = GateChecker::run_checks(
            &self.config.gate_checks,
            &ctx.project_root,
            &ctx.scores,
        )
        .await;

        let required_failures: Vec<_> = gate_results
            .iter()
            .filter(|r| !r.passed && r.required)
            .collect();

        if !required_failures.is_empty() {
            let failure_messages: Vec<String> = required_failures
                .iter()
                .map(|r| format!("  - {}: {}", r.name, r.message))
                .collect();
            return Err(ZcodeError::InternalError(format!(
                "Gate checks failed:\n{}",
                failure_messages.join("\n")
            )));
        }

        // Step 2: Generate changelog
        let changelog = if self.config.auto_changelog {
            ChangelogGenerator::generate(&ctx.tasks, &ctx.commits, &ctx.diff)
        } else {
            String::new()
        };

        // Step 3: Create branch
        let branch_name = ctx
            .branch_name
            .clone()
            .unwrap_or_else(|| Self::generate_branch_name(&self.config.branch_template, &ctx.tasks));

        self.create_branch(&branch_name, &ctx.project_root).await?;

        // Step 4: Stage and commit all changes
        let commit_msg = ctx
            .commit_message
            .clone()
            .unwrap_or_else(|| Self::generate_commit_message(&ctx.tasks));

        self.stage_and_commit(&commit_msg, &ctx.project_root).await?;

        // Step 5: Push
        self.push_branch(&branch_name, &ctx.project_root).await?;

        // Step 6: Create PR (if configured)
        let pr_url = if self.config.auto_pr {
            let pr_body = if let Some(template) = &self.config.pr_template {
                template.clone()
            } else {
                PullRequestCreator::build_pr_body(&ctx.tasks, &changelog, &ctx.scores)
            };

            let creator = PullRequestCreator::new(self.config.platform.clone());
            let opts = PrOptions {
                title: Self::generate_pr_title(&ctx.tasks),
                body: pr_body,
                base_branch: self.config.base_branch.clone(),
                labels: None,
                draft: false,
            };

            match creator.create(&opts).await {
                Ok(result) => Some(result.url),
                Err(e) => {
                    tracing::warn!("PR creation failed: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Step 7: CI monitoring (if configured)
        let ci_status = if let Some(ci_config) = &self.config.ci {
            if let Some(url) = &pr_url {
                let (repo, pr_number) = Self::parse_repo_and_pr(url)
                    .unwrap_or((String::new(), 0));

                if pr_number > 0 {
                    let monitor = CiMonitor::new(ci_config.platform.clone(), ci_config.timeout);
                    match monitor.wait_for_ci(&repo, pr_number).await {
                        Ok(status) => Some(status),
                        Err(e) => {
                            tracing::warn!("CI monitoring failed: {}", e);
                            Some(CiStatus::NotAvailable)
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(DeliveryResult {
            branch: branch_name,
            pr_url,
            changelog,
            version: None, // Version bump not yet implemented
            ci_status,
            delivered_at: Utc::now(),
        })
    }

    /// Generate a branch name from the template and task descriptions
    pub fn generate_branch_name(template: &str, tasks: &[TaskRecord]) -> String {
        let date = Utc::now().format("%Y%m%d").to_string();
        let summary = tasks
            .first()
            .map(|t| {
                let desc: String = t.task.chars().take(30).collect();
                desc.to_lowercase()
                    .replace(' ', "-")
                    .replace(|c: char| !c.is_alphanumeric() && c != '-', "")
            })
            .unwrap_or_else(|| "changes".to_string());

        template
            .replace("{{date}}", &date)
            .replace("{{task_summary}}", &summary)
    }

    /// Generate a commit message from tasks
    pub fn generate_commit_message(tasks: &[TaskRecord]) -> String {
        if tasks.is_empty() {
            return "chore: automated delivery".to_string();
        }

        if tasks.len() == 1 {
            return tasks[0].task.clone();
        }

        format!(
            "chore: deliver {} tasks\n\n{}",
            tasks.len(),
            tasks
                .iter()
                .map(|t| format!("- {}", t.task))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    /// Generate a PR title from tasks
    pub fn generate_pr_title(tasks: &[TaskRecord]) -> String {
        if tasks.is_empty() {
            return "Automated delivery".to_string();
        }

        if tasks.len() == 1 {
            let title: String = tasks[0].task.chars().take(72).collect();
            return title;
        }

        format!("Automated delivery ({} tasks)", tasks.len())
    }

    async fn create_branch(&self, branch_name: &str, project_root: &Path) -> Result<()> {
        let output = Command::new("git")
            .args(["checkout", "-b", branch_name])
            .current_dir(project_root)
            .output()
            .await
            .map_err(|e| ZcodeError::InternalError(format!("git checkout -b failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ZcodeError::InternalError(format!(
                "Failed to create branch '{}': {}",
                branch_name,
                stderr.trim()
            )));
        }

        Ok(())
    }

    async fn stage_and_commit(&self, message: &str, project_root: &Path) -> Result<()> {
        // Stage all changes
        let output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(project_root)
            .output()
            .await
            .map_err(|e| ZcodeError::InternalError(format!("git add failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ZcodeError::InternalError(format!(
                "Failed to stage changes: {}",
                stderr.trim()
            )));
        }

        // Commit
        let output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(project_root)
            .output()
            .await
            .map_err(|e| ZcodeError::InternalError(format!("git commit failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ZcodeError::InternalError(format!(
                "Failed to commit: {}",
                stderr.trim()
            )));
        }

        Ok(())
    }

    async fn push_branch(&self, branch_name: &str, project_root: &Path) -> Result<()> {
        let output = Command::new("git")
            .args(["push", "-u", "origin", branch_name])
            .current_dir(project_root)
            .output()
            .await
            .map_err(|e| ZcodeError::InternalError(format!("git push failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ZcodeError::InternalError(format!(
                "Failed to push branch '{}': {}",
                branch_name,
                stderr.trim()
            )));
        }

        Ok(())
    }

    /// Parse repo owner/name and PR number from a GitHub PR URL
    fn parse_repo_and_pr(url: &str) -> Option<(String, u32)> {
        // https://github.com/owner/repo/pull/42
        let parts: Vec<&str> = url.split('/').collect();
        let pr_number = parts.last()?.parse::<u32>().ok()?;
        // Find "repo" part: ... owner / repo / pull / 42
        if parts.len() >= 5 {
            let repo = format!("{}/{}", parts.get(parts.len() - 4)?, parts.get(parts.len() - 3)?);
            Some((repo, pr_number))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{ChangedFile, FileStatus};
    use std::path::PathBuf;

    fn make_task(desc: &str, score: Option<f64>) -> TaskRecord {
        TaskRecord {
            task: desc.to_string(),
            final_score: score,
            status: "completed".to_string(),
        }
    }

    fn make_diff_context() -> DiffContext {
        DiffContext {
            changed_files: vec![ChangedFile {
                path: "src/main.rs".to_string(),
                status: FileStatus::Modified,
            }],
            staged_files: vec![],
            patch: String::new(),
            repo_root: PathBuf::from("/tmp/repo"),
        }
    }

    #[test]
    fn test_generate_branch_name_single_task() {
        let tasks = vec![make_task("Add auth module", Some(90.0))];
        let name =
            DeliveryPipeline::generate_branch_name("zcode/{{date}}-{{task_summary}}", &tasks);
        assert!(name.starts_with("zcode/"));
        assert!(name.contains("add-auth-module"));
    }

    #[test]
    fn test_generate_branch_name_empty_tasks() {
        let name = DeliveryPipeline::generate_branch_name(
            "zcode/{{date}}-{{task_summary}}",
            &[],
        );
        assert!(name.contains("changes"));
    }

    #[test]
    fn test_generate_branch_name_custom_template() {
        let tasks = vec![make_task("Fix bug", None)];
        let name = DeliveryPipeline::generate_branch_name("feature/{{date}}", &tasks);
        assert!(name.starts_with("feature/"));
    }

    #[test]
    fn test_generate_commit_message_single() {
        let tasks = vec![make_task("Add auth module", None)];
        let msg = DeliveryPipeline::generate_commit_message(&tasks);
        assert_eq!(msg, "Add auth module");
    }

    #[test]
    fn test_generate_commit_message_multiple() {
        let tasks = vec![
            make_task("Task 1", None),
            make_task("Task 2", None),
        ];
        let msg = DeliveryPipeline::generate_commit_message(&tasks);
        assert!(msg.starts_with("chore: deliver 2 tasks"));
        assert!(msg.contains("- Task 1"));
        assert!(msg.contains("- Task 2"));
    }

    #[test]
    fn test_generate_commit_message_empty() {
        let msg = DeliveryPipeline::generate_commit_message(&[]);
        assert_eq!(msg, "chore: automated delivery");
    }

    #[test]
    fn test_generate_pr_title_single() {
        let tasks = vec![make_task("Add auth module", None)];
        let title = DeliveryPipeline::generate_pr_title(&tasks);
        assert_eq!(title, "Add auth module");
    }

    #[test]
    fn test_generate_pr_title_multiple() {
        let tasks = vec![make_task("Task 1", None), make_task("Task 2", None)];
        let title = DeliveryPipeline::generate_pr_title(&tasks);
        assert!(title.contains("2 tasks"));
    }

    #[test]
    fn test_generate_pr_title_empty() {
        let title = DeliveryPipeline::generate_pr_title(&[]);
        assert_eq!(title, "Automated delivery");
    }

    #[test]
    fn test_parse_repo_and_pr() {
        let url = "https://github.com/owner/repo/pull/42";
        let result = DeliveryPipeline::parse_repo_and_pr(url);
        assert_eq!(result, Some(("owner/repo".to_string(), 42)));
    }

    #[test]
    fn test_parse_repo_and_pr_invalid() {
        let url = "https://example.com/something";
        let result = DeliveryPipeline::parse_repo_and_pr(url);
        assert!(result.is_none());
    }

    #[test]
    fn test_delivery_pipeline_new() {
        let config = DeliveryConfig::default();
        let pipeline = DeliveryPipeline::new(config);
        assert!(pipeline.config().auto_pr);
    }

    #[test]
    fn test_delivery_result_fields() {
        let result = DeliveryResult {
            branch: "feature/test".into(),
            pr_url: Some("https://github.com/o/r/pull/1".into()),
            changelog: "# Changelog".into(),
            version: Some("1.0.0".into()),
            ci_status: Some(CiStatus::Passed),
            delivered_at: Utc::now(),
        };
        assert_eq!(result.branch, "feature/test");
        assert!(result.pr_url.is_some());
        assert!(result.ci_status.is_some());
    }

    #[test]
    fn test_delivery_context_fields() {
        let ctx = DeliveryContext {
            tasks: vec![make_task("Test task", Some(80.0))],
            commits: vec!["abc123 commit".into()],
            diff: make_diff_context(),
            scores: vec![("task1".into(), 80.0)],
            project_root: PathBuf::from("/tmp/repo"),
            branch_name: Some("feature/test".into()),
            commit_message: Some("test commit".into()),
        };
        assert_eq!(ctx.tasks.len(), 1);
        assert_eq!(ctx.commits.len(), 1);
        assert!(ctx.branch_name.is_some());
    }
}
