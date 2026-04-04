//! CI monitoring — polls CI status via `gh` CLI or custom commands

use crate::delivery::config::CiPlatform;
use crate::error::{Result, ZcodeError};
use std::time::{Duration, Instant};
use tokio::process::Command;

/// Current status of a CI pipeline
#[derive(Debug, Clone, PartialEq)]
pub enum CiStatus {
    /// All checks passed
    Passed,
    /// One or more checks failed
    Failed { reason: String },
    /// Checks are still running
    Running,
    /// Timed out waiting for checks
    Timeout,
    /// CI is not configured or not available
    NotAvailable,
}

impl std::fmt::Display for CiStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Passed => write!(f, "passed"),
            Self::Failed { reason } => write!(f, "failed: {}", reason),
            Self::Running => write!(f, "running"),
            Self::Timeout => write!(f, "timeout"),
            Self::NotAvailable => write!(f, "not available"),
        }
    }
}

/// CI monitor — polls CI status with configurable timeout
pub struct CiMonitor {
    platform: CiPlatform,
    timeout: Duration,
    /// Polling interval (default: 30 seconds)
    poll_interval: Duration,
}

impl CiMonitor {
    /// Create a new CI monitor
    pub fn new(platform: CiPlatform, timeout: Duration) -> Self {
        Self {
            platform,
            timeout,
            poll_interval: Duration::from_secs(30),
        }
    }

    /// Set a custom polling interval
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Wait for CI to complete on a given PR.
    /// Returns the final CI status.
    pub async fn wait_for_ci(&self, repo: &str, pr_number: u32) -> Result<CiStatus> {
        let start = Instant::now();

        loop {
            let status = self.check_ci_status(repo, pr_number).await?;
            match status {
                CiStatus::Passed => return Ok(CiStatus::Passed),
                CiStatus::Failed { reason } => return Ok(CiStatus::Failed { reason }),
                CiStatus::Running => {
                    if start.elapsed() > self.timeout {
                        return Ok(CiStatus::Timeout);
                    }
                    tokio::time::sleep(self.poll_interval).await;
                }
                CiStatus::NotAvailable => return Ok(CiStatus::NotAvailable),
                CiStatus::Timeout => return Ok(CiStatus::Timeout),
            }
        }
    }

    /// Check the current CI status for a PR (single poll)
    pub async fn check_ci_status(&self, repo: &str, pr_number: u32) -> Result<CiStatus> {
        match &self.platform {
            CiPlatform::GitHubActions => self.check_github_actions(repo, pr_number).await,
            CiPlatform::GitLabCI => self.check_gitlab_ci(repo, pr_number).await,
            CiPlatform::CircleCI => self.check_circle_ci(repo, pr_number).await,
            CiPlatform::Custom { check_command } => {
                self.check_custom(check_command, repo, pr_number).await
            }
        }
    }

    async fn check_github_actions(&self, repo: &str, pr_number: u32) -> Result<CiStatus> {
        let output = Command::new("gh")
            .args([
                "pr",
                "checks",
                &pr_number.to_string(),
                "--repo",
                repo,
            ])
            .output()
            .await
            .map_err(|e| ZcodeError::InternalError(format!("gh pr checks failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // If the command fails, CI might not be set up
        if !output.status.success() && !stderr.is_empty() {
            return Ok(CiStatus::NotAvailable);
        }

        Self::parse_gh_checks_output(&stdout)
    }

    fn parse_gh_checks_output(output: &str) -> Result<CiStatus> {
        let mut all_passed = true;
        let mut has_any = false;
        let mut failures = Vec::new();

        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            has_any = true;

            if trimmed.contains("fail") || trimmed.contains("error") {
                all_passed = false;
                failures.push(trimmed.to_string());
            } else if !trimmed.contains("pass") && !trimmed.contains("neutral") {
                // If there's a line that's neither pass nor fail, it's pending/running
                all_passed = false;
            }
        }

        if !has_any {
            return Ok(CiStatus::NotAvailable);
        }

        if all_passed {
            Ok(CiStatus::Passed)
        } else if !failures.is_empty() {
            Ok(CiStatus::Failed {
                reason: failures.join("; "),
            })
        } else {
            Ok(CiStatus::Running)
        }
    }

    async fn check_gitlab_ci(&self, repo: &str, pr_number: u32) -> Result<CiStatus> {
        let output = Command::new("glab")
            .args([
                "mr",
                "status",
                &pr_number.to_string(),
                "--repo",
                repo,
            ])
            .output()
            .await
            .map_err(|e| ZcodeError::InternalError(format!("glab mr status failed: {}", e)))?;

        if !output.status.success() {
            return Ok(CiStatus::NotAvailable);
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
        if stdout.contains("passed") || stdout.contains("success") {
            Ok(CiStatus::Passed)
        } else if stdout.contains("failed") || stdout.contains("error") {
            Ok(CiStatus::Failed {
                reason: "GitLab CI pipeline failed".into(),
            })
        } else {
            Ok(CiStatus::Running)
        }
    }

    async fn check_circle_ci(&self, _repo: &str, _pr_number: u32) -> Result<CiStatus> {
        // CircleCI would require a different CLI or API call
        Ok(CiStatus::NotAvailable)
    }

    async fn check_custom(
        &self,
        check_command: &str,
        repo: &str,
        pr_number: u32,
    ) -> Result<CiStatus> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!("{} {} {}", check_command, repo, pr_number))
            .output()
            .await
            .map_err(|e| ZcodeError::InternalError(format!("Custom CI check failed: {}", e)))?;

        if output.status.success() {
            Ok(CiStatus::Passed)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(CiStatus::Failed {
                reason: if stderr.is_empty() {
                    "Custom check failed".into()
                } else {
                    stderr.to_string()
                },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ci_status_display() {
        assert_eq!(format!("{}", CiStatus::Passed), "passed");
        assert_eq!(
            format!("{}", CiStatus::Failed { reason: "test fail".into() }),
            "failed: test fail"
        );
        assert_eq!(format!("{}", CiStatus::Running), "running");
        assert_eq!(format!("{}", CiStatus::Timeout), "timeout");
        assert_eq!(format!("{}", CiStatus::NotAvailable), "not available");
    }

    #[test]
    fn test_ci_status_equality() {
        assert_eq!(CiStatus::Passed, CiStatus::Passed);
        assert_eq!(CiStatus::Running, CiStatus::Running);
        assert_eq!(CiStatus::Timeout, CiStatus::Timeout);
        assert_eq!(CiStatus::NotAvailable, CiStatus::NotAvailable);
        assert_eq!(
            CiStatus::Failed { reason: "x".into() },
            CiStatus::Failed { reason: "x".into() }
        );
    }

    #[test]
    fn test_ci_monitor_new() {
        let monitor = CiMonitor::new(
            CiPlatform::GitHubActions,
            Duration::from_secs(300),
        );
        assert_eq!(monitor.poll_interval, Duration::from_secs(30));
    }

    #[test]
    fn test_ci_monitor_custom_poll_interval() {
        let monitor = CiMonitor::new(
            CiPlatform::GitHubActions,
            Duration::from_secs(600),
        )
        .with_poll_interval(Duration::from_secs(10));
        assert_eq!(monitor.poll_interval, Duration::from_secs(10));
    }

    #[test]
    fn test_parse_gh_checks_all_pass() {
        let output = "test-suite  pass  42s\nlint         pass  10s\n";
        let status = CiMonitor::parse_gh_checks_output(output).unwrap();
        assert_eq!(status, CiStatus::Passed);
    }

    #[test]
    fn test_parse_gh_checks_failure() {
        let output = "test-suite  fail  42s\nlint         pass  10s\n";
        let status = CiMonitor::parse_gh_checks_output(output).unwrap();
        assert!(matches!(status, CiStatus::Failed { .. }));
    }

    #[test]
    fn test_parse_gh_checks_running() {
        // "pending" or lines without pass/fail indicate running
        let output = "test-suite  pending\nlint         pass  10s\n";
        let status = CiMonitor::parse_gh_checks_output(output).unwrap();
        assert_eq!(status, CiStatus::Running);
    }

    #[test]
    fn test_parse_gh_checks_empty() {
        let output = "";
        let status = CiMonitor::parse_gh_checks_output(output).unwrap();
        assert_eq!(status, CiStatus::NotAvailable);
    }

    #[test]
    fn test_parse_gh_checks_error() {
        let output = "build  error  30s\n";
        let status = CiMonitor::parse_gh_checks_output(output).unwrap();
        assert!(matches!(status, CiStatus::Failed { .. }));
    }

    #[test]
    fn test_parse_gh_checks_neutral_counts_as_pass() {
        let output = "skip-check  neutral  0s\nlint        pass     5s\n";
        let status = CiMonitor::parse_gh_checks_output(output).unwrap();
        assert_eq!(status, CiStatus::Passed);
    }
}
