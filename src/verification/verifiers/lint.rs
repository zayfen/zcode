//! LintVerifier — runs linters and scores the result

use async_trait::async_trait;
use std::path::Path;
use tokio::process::Command;

use crate::verification::types::{IssueSeverity, VerificationContext, VerificationIssue, VerificationResult};
use super::Verifier;

/// Lint verifier — runs the project's linter and scores based on warnings/errors
pub struct LintVerifier;

impl LintVerifier {
    pub fn new() -> Self {
        Self
    }

    /// Detect lint command based on project files
    fn detect_lint_command(project_root: &Path) -> Option<String> {
        if project_root.join("Cargo.toml").exists() {
            Some("cargo clippy --message-format=json 2>&1".into())
        } else if project_root.join("package.json").exists() {
            Some("npx eslint --format json . 2>/dev/null".into())
        } else {
            None
        }
    }

    /// Parse cargo clippy JSON output
    fn parse_clippy_output(&self, output: &str) -> LintCounts {
        let mut warnings = 0u32;
        let mut errors = 0u32;

        for line in output.lines() {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
                let reason = msg.get("reason").and_then(|r| r.as_str()).unwrap_or("");
                if reason == "compiler-message" {
                    let level = msg
                        .get("message")
                        .and_then(|m| m.get("level"))
                        .and_then(|l| l.as_str())
                        .unwrap_or("");
                    match level {
                        "warning" => warnings += 1,
                        "error" => errors += 1,
                        _ => {}
                    }
                }
            }
        }

        LintCounts { warnings, errors }
    }

    /// Parse eslint JSON output
    fn parse_eslint_output(&self, output: &str) -> LintCounts {
        let mut warnings = 0u32;
        let mut errors = 0u32;

        // eslint outputs JSON array of file results
        // May have extra non-JSON lines, try to find the JSON
        if let Ok(results) = serde_json::from_str::<Vec<serde_json::Value>>(output) {
            for file_result in results {
                if let Some(msgs) = file_result.get("messages").and_then(|m| m.as_array()) {
                    for msg in msgs {
                        let severity = msg.get("severity").and_then(|s| s.as_i64()).unwrap_or(0);
                        match severity {
                            1 => warnings += 1, // warning
                            2 => errors += 1,   // error
                            _ => {}
                        }
                    }
                }
            }
        }

        LintCounts { warnings, errors }
    }
}

struct LintCounts {
    warnings: u32,
    errors: u32,
}

#[async_trait]
impl Verifier for LintVerifier {
    fn name(&self) -> &str {
        "lint"
    }

    fn description(&self) -> &str {
        "Runs the project's linter and scores based on warning/error counts"
    }

    fn weight(&self) -> f64 {
        0.15
    }

    async fn verify(&self, context: &VerificationContext) -> VerificationResult {
        let cmd = match Self::detect_lint_command(&context.project_root) {
            Some(c) => c,
            None => {
                return VerificationResult::skipped(
                    self.name(),
                    "No linter detected for this project",
                );
            }
        };

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
                        "lint",
                        format!("Failed to run linter: {}", e),
                    )],
                );
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let full_output = format!("{}\n{}", stdout, stderr);

        let is_clippy = cmd.starts_with("cargo clippy");
        let counts = if is_clippy {
            self.parse_clippy_output(&full_output)
        } else {
            self.parse_eslint_output(&full_output)
        };

        // Scoring: 0 errors, 0 warnings → 100
        //          warnings → 100 - (warnings * 2), min 40
        //          errors → 100 - (errors * 10), min 0
        let mut score = 100.0;
        if counts.errors > 0 {
            score -= (counts.errors as f64) * 10.0;
            score = score.max(0.0);
        }
        if counts.warnings > 0 && score > 0.0 {
            score -= (counts.warnings as f64) * 2.0;
            score = score.max(40.0);
        }

        let mut issues = Vec::new();
        if counts.errors > 0 {
            issues.push(
                VerificationIssue::new(
                    IssueSeverity::High,
                    "lint",
                    format!("Linter reported {} error(s)", counts.errors),
                )
                .with_suggestion("Fix all linter errors"),
            );
        }
        if counts.warnings > 0 {
            issues.push(
                VerificationIssue::new(
                    IssueSeverity::Medium,
                    "lint",
                    format!("Linter reported {} warning(s)", counts.warnings),
                )
                .with_suggestion("Consider fixing linter warnings"),
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

impl Default for LintVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_clippy() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        let cmd = LintVerifier::detect_lint_command(dir.path());
        assert!(cmd.unwrap().contains("clippy"));
    }

    #[test]
    fn test_detect_no_linter() {
        let dir = tempfile::tempdir().unwrap();
        let cmd = LintVerifier::detect_lint_command(dir.path());
        assert!(cmd.is_none());
    }

    #[test]
    fn test_parse_clippy_clean() {
        let v = LintVerifier::new();
        let output = r#"{"reason":"compiler-artifact"}"#;
        let counts = v.parse_clippy_output(output);
        assert_eq!(counts.warnings, 0);
        assert_eq!(counts.errors, 0);
    }

    #[test]
    fn test_parse_clippy_with_warnings() {
        let v = LintVerifier::new();
        let output = r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused variable"}}
{"reason":"compiler-message","message":{"level":"warning","message":"dead code"}}"#;
        let counts = v.parse_clippy_output(output);
        assert_eq!(counts.warnings, 2);
        assert_eq!(counts.errors, 0);
    }

    #[test]
    fn test_scoring_clean() {
        let counts = LintCounts { warnings: 0, errors: 0 };
        let score = {
            let mut s = 100.0;
            if counts.errors > 0 { s -= counts.errors as f64 * 10.0; s = s.max(0.0); }
            if counts.warnings > 0 && s > 0.0 { s -= counts.warnings as f64 * 2.0; s = s.max(40.0); }
            s
        };
        assert_eq!(score, 100.0);
    }

    #[test]
    fn test_verifier_properties() {
        let v = LintVerifier::new();
        assert_eq!(v.name(), "lint");
        assert_eq!(v.weight(), 0.15);
    }
}
