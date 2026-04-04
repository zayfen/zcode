//! TestVerifier — runs project tests and scores the result

use async_trait::async_trait;
use std::path::Path;
use tokio::process::Command;

use crate::verification::types::{IssueSeverity, VerificationContext, VerificationIssue, VerificationResult};
use super::Verifier;

/// Test verifier — runs the project's test suite and scores the result
pub struct TestVerifier {
    /// Optional override test command
    test_command: Option<String>,
}

impl TestVerifier {
    pub fn new() -> Self {
        Self { test_command: None }
    }

    pub fn with_command(command: impl Into<String>) -> Self {
        Self {
            test_command: Some(command.into()),
        }
    }

    /// Detect the appropriate test command based on project files
    fn detect_test_command(project_root: &Path) -> String {
        if project_root.join("Cargo.toml").exists() {
            "cargo test --no-fail-fast 2>&1".into()
        } else if project_root.join("package.json").exists() {
            "npm test 2>&1".into()
        } else if project_root.join("pytest.ini").exists() || project_root.join("pyproject.toml").exists() {
            "pytest --tb=short -q 2>&1".into()
        } else if project_root.join("go.mod").exists() {
            "go test ./... 2>&1".into()
        } else {
            "make test 2>&1".into()
        }
    }

    fn get_command(&self, project_root: &Path) -> String {
        self.test_command
            .clone()
            .unwrap_or_else(|| Self::detect_test_command(project_root))
    }

    /// Parse cargo test output for pass/fail counts
    fn parse_cargo_output(&self, output: &str) -> TestCounts {
        let mut passed = 0u32;
        let mut failed = 0u32;
        let mut total;

        for line in output.lines() {
            // Match: "test result: ok. 5 passed; 0 failed; 0 ignored;"
            // or:     "test result: FAILED. 3 passed; 2 failed; 0 ignored;"
            if line.contains("test result:") {
                if let Some(p) = Self::extract_count(line, "passed") {
                    passed += p;
                }
                if let Some(f) = Self::extract_count(line, "failed") {
                    failed += f;
                }
            }
        }

        total = passed + failed;
        if total == 0 {
            // Try to count individual test lines: "test foo ... ok" / "test bar ... FAILED"
            for line in output.lines() {
                if line.contains(" ... ok") {
                    passed += 1;
                    total += 1;
                } else if line.contains(" ... FAILED") {
                    failed += 1;
                    total += 1;
                }
            }
        }

        TestCounts { passed, failed, total }
    }

    fn extract_count(line: &str, keyword: &str) -> Option<u32> {
        let parts: Vec<&str> = line.split(&format!(" {}", keyword)).collect();
        if parts.len() < 2 {
            return None;
        }
        let before = parts[0];
        let num_str = before.trim().split_whitespace().last()?;
        num_str.parse().ok()
    }

    /// Parse npm test output
    fn parse_npm_output(&self, output: &str) -> TestCounts {
        // Look for patterns like "Tests: 5 passed, 2 failed"
        let mut passed = 0u32;
        let mut failed = 0u32;

        for line in output.lines() {
            if let Some(p) = Self::extract_count(line, "passed") {
                passed += p;
            }
            if let Some(f) = Self::extract_count(line, "failed") {
                failed += f;
            }
            // Jest: "Tests:       5 passed, 2 failed, 7 total"
            if line.contains("passed") && line.contains("failed") {
                // already captured above
            }
        }

        let total = if passed + failed > 0 { passed + failed } else { 0 };
        TestCounts { passed, failed, total }
    }
}

struct TestCounts {
    passed: u32,
    failed: u32,
    total: u32,
}

#[async_trait]
impl Verifier for TestVerifier {
    fn name(&self) -> &str {
        "test"
    }

    fn description(&self) -> &str {
        "Runs the project test suite and scores based on pass rate"
    }

    fn weight(&self) -> f64 {
        0.30
    }

    async fn verify(&self, context: &VerificationContext) -> VerificationResult {
        let cmd = self.get_command(&context.project_root);

        let output = match Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .current_dir(&context.project_root)
            .output()
            .await
        {
            Ok(o) => o,
            Err(e) => {
                return VerificationResult::with_issues(
                    self.name(),
                    0.0,
                    vec![VerificationIssue::new(
                        IssueSeverity::Critical,
                        "test",
                        format!("Failed to run test command: {}", e),
                    )],
                );
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let full_output = format!("{}\n{}", stdout, stderr);

        let is_cargo = cmd.starts_with("cargo");
        let counts = if is_cargo {
            self.parse_cargo_output(&full_output)
        } else {
            self.parse_npm_output(&full_output)
        };

        if counts.total == 0 && !output.status.success() {
            // Compilation failure or no tests found but command failed
            return VerificationResult::with_issues(
                self.name(),
                0.0,
                vec![VerificationIssue::new(
                    IssueSeverity::Critical,
                    "test",
                    "Test command failed — possible compilation error",
                )
                .with_snippet(full_output.chars().take(500).collect::<String>())],
            );
        }

        if counts.total == 0 {
            // No tests found — give neutral score
            return VerificationResult {
                verifier_name: self.name().into(),
                score: 50.0,
                issues: vec![VerificationIssue::new(
                    IssueSeverity::Info,
                    "test",
                    "No tests detected for this project",
                )],
                log: full_output,
            };
        }

        let score = if counts.failed == 0 {
            100.0
        } else {
            (counts.passed as f64 / counts.total as f64) * 100.0
        };

        let mut issues = Vec::new();
        if counts.failed > 0 {
            issues.push(
                VerificationIssue::new(
                    IssueSeverity::High,
                    "test",
                    format!("{} out of {} tests failed", counts.failed, counts.total),
                )
                .with_suggestion("Fix failing tests before proceeding"),
            );
        }

        VerificationResult {
            verifier_name: self.name().into(),
            score,
            issues,
            log: full_output,
        }
    }
}

impl Default for TestVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_cargo() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        let cmd = TestVerifier::detect_test_command(dir.path());
        assert!(cmd.starts_with("cargo test"));
    }

    #[test]
    fn test_detect_npm() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        let cmd = TestVerifier::detect_test_command(dir.path());
        assert!(cmd.starts_with("npm test"));
    }

    #[test]
    fn test_parse_cargo_output_all_passed() {
        let v = TestVerifier::new();
        let output = "running 3 tests\ntest foo ... ok\ntest bar ... ok\ntest baz ... ok\n\ntest result: ok. 3 passed; 0 failed; 0 ignored;";
        let counts = v.parse_cargo_output(output);
        assert_eq!(counts.passed, 3);
        assert_eq!(counts.failed, 0);
        assert_eq!(counts.total, 3);
    }

    #[test]
    fn test_parse_cargo_output_some_failed() {
        let v = TestVerifier::new();
        let output = "running 3 tests\ntest foo ... ok\ntest bar ... FAILED\ntest baz ... FAILED\n\ntest result: FAILED. 1 passed; 2 failed;";
        let counts = v.parse_cargo_output(output);
        assert_eq!(counts.passed, 1);
        assert_eq!(counts.failed, 2);
        assert_eq!(counts.total, 3);
    }

    #[test]
    fn test_parse_cargo_output_no_summary() {
        let v = TestVerifier::new();
        let output = "test foo ... ok\ntest bar ... FAILED";
        let counts = v.parse_cargo_output(output);
        assert_eq!(counts.passed, 1);
        assert_eq!(counts.failed, 1);
        assert_eq!(counts.total, 2);
    }

    #[test]
    fn test_verifier_properties() {
        let v = TestVerifier::new();
        assert_eq!(v.name(), "test");
        assert!(!v.description().is_empty());
        assert_eq!(v.weight(), 0.30);
    }

    #[test]
    fn test_custom_command() {
        let v = TestVerifier::with_command("pytest -x");
        assert_eq!(v.test_command.as_deref(), Some("pytest -x"));
    }
}
