//! Reviewer Agent
//!
//! Analyzes code diffs and produces review results with issues and suggestions.

use crate::error::{Result, ZcodeError};
use serde::{Deserialize, Serialize};

// ─── Review Types ─────────────────────────────────────────────────────────────

/// Severity of a review issue
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
    Suggestion,
}

/// A single issue found during review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    pub severity: IssueSeverity,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: String,
    pub category: ReviewCategory,
}

/// Category of the review issue
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewCategory {
    Logic,
    Security,
    Performance,
    Style,
    Documentation,
    Testing,
    Architecture,
}

/// Complete result of a code review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResult {
    /// Whether the reviewer approves the change
    pub approved: bool,
    /// Issues found (errors, warnings, suggestions)
    pub issues: Vec<ReviewIssue>,
    /// Free-form suggestions for improvement
    pub suggestions: Vec<String>,
    /// Summary paragraph
    pub summary: String,
    /// Quality score 0-100
    pub quality_score: u8,
}

impl ReviewResult {
    pub fn approved(summary: impl Into<String>) -> Self {
        Self {
            approved: true,
            issues: vec![],
            suggestions: vec![],
            summary: summary.into(),
            quality_score: 90,
        }
    }

    pub fn rejected(summary: impl Into<String>, issues: Vec<ReviewIssue>) -> Self {
        Self {
            approved: false,
            issues,
            suggestions: vec![],
            summary: summary.into(),
            quality_score: 40,
        }
    }

    /// Number of issues by severity
    pub fn error_count(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == IssueSeverity::Error).count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == IssueSeverity::Warning).count()
    }
}

// ─── ReviewConfig ─────────────────────────────────────────────────────────────

/// Configuration for the reviewer
#[derive(Debug, Clone)]
pub struct ReviewConfig {
    /// Focus areas for the review
    pub check_logic: bool,
    pub check_security: bool,
    pub check_performance: bool,
    pub check_style: bool,
    pub check_tests: bool,
    /// Maximum number of issues to return
    pub max_issues: usize,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            check_logic: true,
            check_security: true,
            check_performance: true,
            check_style: true,
            check_tests: true,
            max_issues: 20,
        }
    }
}

// ─── ReviewerAgent ────────────────────────────────────────────────────────────

/// Agent that reviews code changes and produces structured feedback
pub struct ReviewerAgent {
    config: ReviewConfig,
}

impl ReviewerAgent {
    pub fn new() -> Self {
        Self { config: ReviewConfig::default() }
    }

    pub fn with_config(config: ReviewConfig) -> Self {
        Self { config }
    }

    /// Review a code diff string
    ///
    /// Performs static analysis on the diff and returns a ReviewResult.
    /// In a full implementation this would call an LLM; here we implement
    /// a rules-based analyzer for testing purposes.
    pub fn review_diff(&self, diff: &str) -> Result<ReviewResult> {
        if diff.trim().is_empty() {
            return Ok(ReviewResult::approved("No changes to review."));
        }

        let mut issues = Vec::new();
        let lines: Vec<&str> = diff.lines().collect();

        // Parse added lines (start with '+' but not '+++')
        let added_lines: Vec<(usize, &str)> = lines.iter().enumerate()
            .filter(|(_, l)| l.starts_with('+') && !l.starts_with("+++"))
            .map(|(i, l)| (i + 1, *l))
            .collect();

        // ── Logic checks ──────────────────────────────────────────────────────
        if self.config.check_logic {
            for (line_num, line) in &added_lines {
                // Detect .unwrap() without context
                if line.contains(".unwrap()") && !line.trim_start().starts_with("//") {
                    issues.push(ReviewIssue {
                        severity: IssueSeverity::Warning,
                        file: None,
                        line: Some(*line_num as u32),
                        message: "Avoid `.unwrap()` — use `?` or explicit error handling instead."
                            .to_string(),
                        category: ReviewCategory::Logic,
                    });
                }
                // Detect panic!
                if line.contains("panic!(") {
                    issues.push(ReviewIssue {
                        severity: IssueSeverity::Warning,
                        file: None,
                        line: Some(*line_num as u32),
                        message: "`panic!()` found — ensure this is intentional.".to_string(),
                        category: ReviewCategory::Logic,
                    });
                }
            }
        }

        // ── Security checks ───────────────────────────────────────────────────
        if self.config.check_security {
            for (line_num, line) in &added_lines {
                // Detect hardcoded secrets patterns
                let lower = line.to_lowercase();
                if (lower.contains("password") || lower.contains("secret") || lower.contains("api_key"))
                    && lower.contains('=')
                    && (lower.contains('"') || lower.contains('\''))
                    && !lower.trim_start().starts_with("//")
                {
                    issues.push(ReviewIssue {
                        severity: IssueSeverity::Error,
                        file: None,
                        line: Some(*line_num as u32),
                        message: "Possible hardcoded credential detected — use environment variables."
                            .to_string(),
                        category: ReviewCategory::Security,
                    });
                }

                // SQL injection risk
                if (line.contains("format!(") || line.contains("format_args!("))
                    && (lower.contains("select") || lower.contains("insert") || lower.contains("delete"))
                {
                    issues.push(ReviewIssue {
                        severity: IssueSeverity::Warning,
                        file: None,
                        line: Some(*line_num as u32),
                        message: "Possible SQL injection risk — use parameterized queries.".to_string(),
                        category: ReviewCategory::Security,
                    });
                }
            }
        }

        // ── Performance checks ────────────────────────────────────────────────
        if self.config.check_performance {
            for (line_num, line) in &added_lines {
                // Detect .clone() on large structures (heuristic)
                if line.contains(".clone()") && (line.contains("Vec<") || line.contains("HashMap<")) {
                    issues.push(ReviewIssue {
                        severity: IssueSeverity::Info,
                        file: None,
                        line: Some(*line_num as u32),
                        message: "Consider whether `.clone()` is necessary — prefer borrowing."
                            .to_string(),
                        category: ReviewCategory::Performance,
                    });
                }
            }
        }

        // ── Style checks ──────────────────────────────────────────────────────
        if self.config.check_style {
            // Detect very long lines in added code
            for (line_num, line) in &added_lines {
                if line.len() > 120 {
                    issues.push(ReviewIssue {
                        severity: IssueSeverity::Suggestion,
                        file: None,
                        line: Some(*line_num as u32),
                        message: format!("Line is {} chars — consider splitting at 100-120 chars.", line.len()),
                        category: ReviewCategory::Style,
                    });
                }
            }
        }

        // ── Test coverage ─────────────────────────────────────────────────────
        if self.config.check_tests {
            let has_new_fn = added_lines.iter().any(|(_, l)| l.contains("pub fn ") || l.contains("fn "));
            let has_new_test = diff.contains("#[test]") || diff.contains("#[cfg(test)]");

            if has_new_fn && !has_new_test {
                issues.push(ReviewIssue {
                    severity: IssueSeverity::Suggestion,
                    file: None,
                    line: None,
                    message: "New functions added without corresponding tests — consider adding tests."
                        .to_string(),
                    category: ReviewCategory::Testing,
                });
            }
        }

        // Truncate to max_issues
        issues.truncate(self.config.max_issues);

        let error_count = issues.iter().filter(|i| i.severity == IssueSeverity::Error).count();
        let warning_count = issues.iter().filter(|i| i.severity == IssueSeverity::Warning).count();

        let approved = error_count == 0;
        let quality_score = (100u32
            .saturating_sub((error_count * 20) as u32)
            .saturating_sub((warning_count * 5) as u32)
            .min(100)) as u8;

        let summary = if approved {
            if issues.is_empty() {
                "Code review passed — no issues found.".to_string()
            } else {
                format!(
                    "Code review passed with {} warning(s) and {} suggestion(s).",
                    warning_count,
                    issues.iter().filter(|i| matches!(i.severity, IssueSeverity::Suggestion | IssueSeverity::Info)).count()
                )
            }
        } else {
            format!(
                "Code review failed — {} error(s) found that must be addressed.",
                error_count
            )
        };

        let mut suggestions = Vec::new();
        if error_count > 0 {
            suggestions.push("Address all Error-level issues before merging.".to_string());
        }
        if warning_count > 0 {
            suggestions.push("Review Warning-level issues before merging.".to_string());
        }

        Ok(ReviewResult {
            approved,
            issues,
            suggestions,
            summary,
            quality_score,
        })
    }

    /// Review file content directly (for non-diff reviews)
    pub fn review_content(&self, content: &str, file_name: &str) -> Result<ReviewResult> {
        // Create a synthetic diff for the content
        let synthetic_diff = content.lines()
            .map(|l| format!("+{}", l))
            .collect::<Vec<_>>()
            .join("\n");
        let diff = format!("+++ {}\n{}", file_name, synthetic_diff);
        self.review_diff(&diff)
    }
}

impl Default for ReviewerAgent {
    fn default() -> Self { Self::new() }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn reviewer() -> ReviewerAgent { ReviewerAgent::new() }

    #[test]
    fn test_empty_diff_approved() {
        let result = reviewer().review_diff("").unwrap();
        assert!(result.approved);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn test_clean_diff_approved() {
        let diff = "+fn add(a: i32, b: i32) -> i32 { a + b }\n+#[test]\n+fn test_add() { assert_eq!(add(1,2), 3); }";
        let result = reviewer().review_diff(diff).unwrap();
        assert!(result.approved);
    }

    #[test]
    fn test_detects_unwrap() {
        let diff = "+let value = some_option.unwrap();\n";
        let result = reviewer().review_diff(diff).unwrap();
        let unwrap_issues: Vec<_> = result.issues.iter()
            .filter(|i| i.message.contains("unwrap"))
            .collect();
        assert!(!unwrap_issues.is_empty());
        assert_eq!(unwrap_issues[0].severity, IssueSeverity::Warning);
        assert_eq!(unwrap_issues[0].category, ReviewCategory::Logic);
    }

    #[test]
    fn test_detects_panic() {
        let diff = "+panic!(\"something went wrong\");";
        let result = reviewer().review_diff(diff).unwrap();
        let panic_issues: Vec<_> = result.issues.iter()
            .filter(|i| i.message.contains("panic"))
            .collect();
        assert!(!panic_issues.is_empty());
    }

    #[test]
    fn test_detects_hardcoded_password() {
        let diff = "+let password = \"s3cr3t_p@ss\";";
        let result = reviewer().review_diff(diff).unwrap();
        let sec_issues: Vec<_> = result.issues.iter()
            .filter(|i| i.category == ReviewCategory::Security && i.severity == IssueSeverity::Error)
            .collect();
        assert!(!sec_issues.is_empty());
        assert!(!result.approved); // error → rejected
    }

    #[test]
    fn test_detects_hardcoded_api_key() {
        let diff = "+const API_KEY: &str = \"sk-abcdef123\";";
        let result = reviewer().review_diff(diff).unwrap();
        let issues: Vec<_> = result.issues.iter()
            .filter(|i| i.category == ReviewCategory::Security)
            .collect();
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_missing_tests_suggestion() {
        let diff = "+pub fn compute(x: i32) -> i32 { x * 2 }";
        let result = reviewer().review_diff(diff).unwrap();
        let test_issues: Vec<_> = result.issues.iter()
            .filter(|i| i.category == ReviewCategory::Testing)
            .collect();
        assert!(!test_issues.is_empty());
    }

    #[test]
    fn test_no_test_warning_when_tests_present() {
        let diff = "+pub fn compute(x: i32) -> i32 { x * 2 }\n+#[test]\n+fn test_compute() {}";
        let result = reviewer().review_diff(diff).unwrap();
        let test_issues: Vec<_> = result.issues.iter()
            .filter(|i| i.category == ReviewCategory::Testing)
            .collect();
        assert!(test_issues.is_empty());
    }

    #[test]
    fn test_long_line_suggestion() {
        let long_line = format!("+{}", "x".repeat(130));
        let result = reviewer().review_diff(&long_line).unwrap();
        let style_issues: Vec<_> = result.issues.iter()
            .filter(|i| i.category == ReviewCategory::Style)
            .collect();
        assert!(!style_issues.is_empty());
    }

    #[test]
    fn test_quality_score_clean() {
        let diff = "+fn clean() -> bool { true }";
        let result = reviewer().review_diff(diff).unwrap();
        assert!(result.quality_score >= 90);
    }

    #[test]
    fn test_quality_score_with_errors() {
        let diff = "+let password = \"secret\";";
        let result = reviewer().review_diff(diff).unwrap();
        assert!(result.quality_score < 90);
    }

    #[test]
    fn test_error_and_warning_counts() {
        let result = ReviewResult {
            approved: false,
            issues: vec![
                ReviewIssue { severity: IssueSeverity::Error, file: None, line: None, message: "e1".to_string(), category: ReviewCategory::Security },
                ReviewIssue { severity: IssueSeverity::Error, file: None, line: None, message: "e2".to_string(), category: ReviewCategory::Logic },
                ReviewIssue { severity: IssueSeverity::Warning, file: None, line: None, message: "w1".to_string(), category: ReviewCategory::Style },
            ],
            suggestions: vec![],
            summary: String::new(),
            quality_score: 50,
        };
        assert_eq!(result.error_count(), 2);
        assert_eq!(result.warning_count(), 1);
    }

    #[test]
    fn test_approved_result_factory() {
        let r = ReviewResult::approved("Looks good");
        assert!(r.approved);
        assert!(r.issues.is_empty());
        assert_eq!(r.quality_score, 90);
    }

    #[test]
    fn test_rejected_result_factory() {
        let issues = vec![ReviewIssue {
            severity: IssueSeverity::Error,
            file: None, line: None,
            message: "bad".to_string(),
            category: ReviewCategory::Security,
        }];
        let r = ReviewResult::rejected("Needs fixes", issues);
        assert!(!r.approved);
        assert_eq!(r.error_count(), 1);
    }

    #[test]
    fn test_review_content() {
        let reviewer = ReviewerAgent::new();
        let content = "fn main() { let x = some_thing.unwrap(); }";
        let result = reviewer.review_content(content, "main.rs").unwrap();
        // Should detect unwrap
        assert!(!result.issues.is_empty());
    }

    #[test]
    fn test_config_disable_security() {
        let mut config = ReviewConfig::default();
        config.check_security = false;
        let reviewer = ReviewerAgent::with_config(config);
        let diff = "+let password = \"secret\";";
        let result = reviewer.review_diff(diff).unwrap();
        // Security disabled → should be approved
        assert!(result.approved);
    }

    #[test]
    fn test_max_issues_truncated() {
        let config = ReviewConfig {
            max_issues: 2,
            ..Default::default()
        };
        let reviewer = ReviewerAgent::with_config(config);
        // Create many unwrap lines
        let diff: String = (0..10).map(|i| format!("+let v{} = x.unwrap();\n", i)).collect();
        let result = reviewer.review_diff(&diff).unwrap();
        assert!(result.issues.len() <= 2);
    }
}
