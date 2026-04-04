//! Gate check executor — runs configured checks before delivery

use crate::delivery::config::{GateCheck, GateCheckType};
use std::path::Path;
use tokio::process::Command;

/// Result of a single gate check
#[derive(Debug, Clone)]
pub struct GateResult {
    /// Name of the check
    pub name: String,
    /// Whether the check passed
    pub passed: bool,
    /// Whether this check is required
    pub required: bool,
    /// Human-readable message about the result
    pub message: String,
}

impl GateResult {
    /// Create a passing gate result
    pub fn passed(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            required: true,
            message: message.into(),
        }
    }

    /// Create a failing gate result
    pub fn failed(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            required: true,
            message: message.into(),
        }
    }
}

/// Gate check executor — runs all configured gate checks
pub struct GateChecker;

impl GateChecker {
    /// Run all gate checks. Returns a list of results.
    /// The caller should inspect each GateResult to determine if delivery can proceed.
    pub async fn run_checks(
        checks: &[GateCheck],
        project_root: &Path,
        task_scores: &[(String, f64)],
    ) -> Vec<GateResult> {
        let mut results = Vec::with_capacity(checks.len());

        for check in checks {
            let result = Self::run_single_check(check, project_root, task_scores).await;
            results.push(GateResult {
                name: check.name.clone(),
                required: check.required,
                passed: result.passed,
                message: result.message,
            });
        }

        results
    }

    /// Run a single gate check
    pub async fn run_single_check(
        check: &GateCheck,
        project_root: &Path,
        task_scores: &[(String, f64)],
    ) -> GateResult {
        match &check.check_type {
            GateCheckType::Command {
                command,
                expected_exit_code,
            } => Self::check_command(command, *expected_exit_code, project_root).await,
            GateCheckType::FileExists { path } => Self::check_file_exists(path, project_root),
            GateCheckType::MinVerificationScore { min_score } => {
                Self::check_min_score(task_scores, *min_score)
            }
            GateCheckType::CleanWorkingTree => {
                Self::check_clean_working_tree(project_root).await
            }
            GateCheckType::CanFastForward { target_branch } => {
                Self::check_fast_forward(target_branch, project_root).await
            }
        }
    }

    async fn check_command(
        command: &str,
        expected_exit_code: i32,
        project_root: &Path,
    ) -> GateResult {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(project_root)
            .output()
            .await;

        match output {
            Ok(output) => {
                let actual_code = output.status.code().unwrap_or(-1);
                if actual_code == expected_exit_code {
                    GateResult::passed(
                        "command",
                        format!("`{}` exited with code {}", command, actual_code),
                    )
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    GateResult::failed(
                        "command",
                        format!(
                            "`{}` exited with code {} (expected {}): {}",
                            command,
                            actual_code,
                            expected_exit_code,
                            stderr.trim()
                        ),
                    )
                }
            }
            Err(e) => GateResult::failed(
                "command",
                format!("Failed to run `{}`: {}", command, e),
            ),
        }
    }

    fn check_file_exists(path: &str, project_root: &Path) -> GateResult {
        let full_path = project_root.join(path);
        if full_path.exists() {
            GateResult::passed("file_exists", format!("{} exists", path))
        } else {
            GateResult::failed("file_exists", format!("{} does not exist", path))
        }
    }

    fn check_min_score(task_scores: &[(String, f64)], min_score: f64) -> GateResult {
        if task_scores.is_empty() {
            return GateResult::passed(
                "min_score",
                "No tasks to verify (vacuously true)".to_string(),
            );
        }

        let min_found = task_scores
            .iter()
            .map(|(_, s)| *s)
            .fold(f64::INFINITY, f64::min);
        let avg: f64 = task_scores.iter().map(|(_, s)| *s).sum::<f64>()
            / task_scores.len() as f64;

        if min_found >= min_score {
            GateResult::passed(
                "min_score",
                format!(
                    "All scores >= {:.0} (min: {:.1}, avg: {:.1})",
                    min_score, min_found, avg
                ),
            )
        } else {
            let below: Vec<&str> = task_scores
                .iter()
                .filter(|(_, s)| *s < min_score)
                .map(|(n, _)| n.as_str())
                .collect();
            GateResult::failed(
                "min_score",
                format!(
                    "Scores below {:.0}: {} (min: {:.1}, avg: {:.1})",
                    min_score,
                    below.join(", "),
                    min_found,
                    avg
                ),
            )
        }
    }

    async fn check_clean_working_tree(project_root: &Path) -> GateResult {
        // Check for unstaged changes
        let output = Command::new("git")
            .args(["diff", "--quiet"])
            .current_dir(project_root)
            .output()
            .await;

        let unstaged_clean = match output {
            Ok(o) => o.status.success(),
            Err(e) => {
                return GateResult::failed(
                    "clean_tree",
                    format!("Failed to check git status: {}", e),
                )
            }
        };

        // Check for staged changes
        let output = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(project_root)
            .output()
            .await;

        let staged_clean = match output {
            Ok(o) => o.status.success(),
            Err(e) => {
                return GateResult::failed(
                    "clean_tree",
                    format!("Failed to check staged changes: {}", e),
                )
            }
        };

        // Check for untracked files
        let output = Command::new("git")
            .args(["ls-files", "--others", "--exclude-standard"])
            .current_dir(project_root)
            .output()
            .await;

        let untracked_clean = match output {
            Ok(o) => String::from_utf8_lossy(&o.stdout).trim().is_empty(),
            Err(e) => {
                return GateResult::failed(
                    "clean_tree",
                    format!("Failed to check untracked files: {}", e),
                )
            }
        };

        if unstaged_clean && staged_clean && untracked_clean {
            GateResult::passed("clean_tree", "Working tree is clean")
        } else {
            let mut issues = Vec::new();
            if !unstaged_clean {
                issues.push("unstaged changes");
            }
            if !staged_clean {
                issues.push("staged changes");
            }
            if !untracked_clean {
                issues.push("untracked files");
            }
            GateResult::failed(
                "clean_tree",
                format!("Working tree has: {}", issues.join(", ")),
            )
        }
    }

    async fn check_fast_forward(target_branch: &str, project_root: &Path) -> GateResult {
        let output = Command::new("git")
            .args(["merge-base", "--is-ancestor", target_branch, "HEAD"])
            .current_dir(project_root)
            .output()
            .await;

        match output {
            Ok(o) => {
                if o.status.success() {
                    GateResult::passed(
                        "fast_forward",
                        format!("Can fast-forward to {}", target_branch),
                    )
                } else {
                    GateResult::failed(
                        "fast_forward",
                        format!("Cannot fast-forward to {}", target_branch),
                    )
                }
            }
            Err(e) => GateResult::failed(
                "fast_forward",
                format!("Failed to check fast-forward: {}", e),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delivery::config::GateCheckType;

    fn make_check(name: &str, check_type: GateCheckType, required: bool) -> GateCheck {
        GateCheck {
            name: name.to_string(),
            check_type,
            required,
        }
    }

    #[test]
    fn test_gate_result_passed() {
        let result = GateResult::passed("test", "all good");
        assert!(result.passed);
        assert_eq!(result.message, "all good");
    }

    #[test]
    fn test_gate_result_failed() {
        let result = GateResult::failed("test", "something wrong");
        assert!(!result.passed);
        assert_eq!(result.message, "something wrong");
    }

    #[tokio::test]
    async fn test_check_file_exists_true() {
        let project_root = Path::new("/Users/riven/Github/zcode");
        let result = GateChecker::check_file_exists("Cargo.toml", project_root);
        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_check_file_exists_false() {
        let project_root = Path::new("/Users/riven/Github/zcode");
        let result = GateChecker::check_file_exists("nonexistent_file.xyz", project_root);
        assert!(!result.passed);
    }

    #[test]
    fn test_check_min_score_all_pass() {
        let scores = vec![
            ("task1".to_string(), 80.0),
            ("task2".to_string(), 90.0),
        ];
        let result = GateChecker::check_min_score(&scores, 70.0);
        assert!(result.passed);
    }

    #[test]
    fn test_check_min_score_some_fail() {
        let scores = vec![
            ("task1".to_string(), 60.0),
            ("task2".to_string(), 90.0),
        ];
        let result = GateChecker::check_min_score(&scores, 70.0);
        assert!(!result.passed);
        assert!(result.message.contains("task1"));
    }

    #[test]
    fn test_check_min_score_empty() {
        let scores: Vec<(String, f64)> = vec![];
        let result = GateChecker::check_min_score(&scores, 70.0);
        assert!(result.passed);
    }

    #[test]
    fn test_check_min_score_exactly_at_threshold() {
        let scores = vec![("task1".to_string(), 70.0)];
        let result = GateChecker::check_min_score(&scores, 70.0);
        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_run_checks_empty() {
        let results = GateChecker::run_checks(
            &[],
            Path::new("/Users/riven/Github/zcode"),
            &[],
        )
        .await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_run_checks_with_file_exists() {
        let checks = vec![make_check(
            "cargo_toml",
            GateCheckType::FileExists {
                path: "Cargo.toml".into(),
            },
            true,
        )];
        let results = GateChecker::run_checks(
            &checks,
            Path::new("/Users/riven/Github/zcode"),
            &[],
        )
        .await;
        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
        assert!(results[0].required);
    }

    #[tokio::test]
    async fn test_run_checks_with_scores() {
        let checks = vec![make_check(
            "score_check",
            GateCheckType::MinVerificationScore { min_score: 50.0 },
            true,
        )];
        let scores = vec![("task1".to_string(), 85.0)];
        let results = GateChecker::run_checks(
            &checks,
            Path::new("/Users/riven/Github/zcode"),
            &scores,
        )
        .await;
        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
    }

    #[tokio::test]
    async fn test_check_command_true_command() {
        let result = GateChecker::check_command("true", 0, Path::new("/tmp")).await;
        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_check_command_false_command() {
        let result = GateChecker::check_command("false", 0, Path::new("/tmp")).await;
        assert!(!result.passed);
    }

    #[tokio::test]
    async fn test_check_command_expected_nonzero() {
        let result = GateChecker::check_command("false", 1, Path::new("/tmp")).await;
        assert!(result.passed);
    }
}
