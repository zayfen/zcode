//! Verification feedback — injected into next execution round

use crate::verification::types::{IssueSeverity, VerificationIssue};
use serde::{Deserialize, Serialize};

/// Feedback from verification to be injected into next execution round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationFeedback {
    /// Total score achieved
    pub score: f64,
    /// Minimum passing score
    pub min_score: f64,
    /// Score gap to pass
    pub gap: f64,
    /// Top issues sorted by severity
    pub issues: Vec<VerificationIssue>,
    /// Current retry iteration
    pub retry_iteration: u32,
    /// Maximum retries allowed
    pub max_retries: u32,
    /// Test output (if available)
    pub test_output: Option<String>,
    /// Lint output (if available)
    pub lint_output: Option<String>,
}

impl VerificationFeedback {
    /// Create feedback from verification results
    pub fn new(
        score: f64,
        min_score: f64,
        issues: Vec<VerificationIssue>,
        retry_iteration: u32,
        max_retries: u32,
    ) -> Self {
        Self {
            gap: (min_score - score).max(0.0),
            score,
            min_score,
            issues,
            retry_iteration,
            max_retries,
            test_output: None,
            lint_output: None,
        }
    }

    /// Format as LLM-friendly prompt context
    pub fn as_prompt_context(&self) -> String {
        let issues_text = self
            .issues
            .iter()
            .enumerate()
            .map(|(i, issue)| {
                let severity_icon = match issue.severity {
                    IssueSeverity::Critical => "CRITICAL",
                    IssueSeverity::High => "HIGH",
                    IssueSeverity::Medium => "MEDIUM",
                    IssueSeverity::Low => "LOW",
                    IssueSeverity::Info => "INFO",
                };
                format!(
                    "{}. [{}] {} — {}",
                    i + 1,
                    severity_icon,
                    issue.message,
                    if issue.suggestion.is_empty() {
                        "no suggestion"
                    } else {
                        &issue.suggestion
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let test_section = self
            .test_output
            .as_deref()
            .map(|t| format!("### Test Output\n```\n{}\n```\n", t))
            .unwrap_or_default();

        let lint_section = self
            .lint_output
            .as_deref()
            .map(|l| format!("### Lint Output\n```\n{}\n```\n", l))
            .unwrap_or_default();

        format!(
            "## Verification Feedback (round {}/{})\n\
             Score: {:.1}/100 (need {:.1} to pass)\n\n\
             ### Issues to Fix:\n{}\n\n\
             {}{}\n\
             Please fix the above issues. Prioritize Critical and High severity.",
            self.retry_iteration,
            self.max_retries,
            self.score,
            self.min_score,
            issues_text,
            test_section,
            lint_section,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_creation() {
        let fb = VerificationFeedback::new(45.0, 70.0, vec![], 1, 3);
        assert_eq!(fb.score, 45.0);
        assert_eq!(fb.gap, 25.0);
        assert_eq!(fb.retry_iteration, 1);
    }

    #[test]
    fn test_feedback_gap_zero_when_passed() {
        let fb = VerificationFeedback::new(80.0, 70.0, vec![], 1, 3);
        assert_eq!(fb.gap, 0.0);
    }

    #[test]
    fn test_feedback_prompt_context() {
        let issues = vec![
            VerificationIssue::new(IssueSeverity::Critical, "security", "hardcoded secret")
                .with_suggestion("use env var"),
            VerificationIssue::new(IssueSeverity::Medium, "style", "long line"),
        ];
        let fb = VerificationFeedback::new(50.0, 70.0, issues, 2, 3);
        let prompt = fb.as_prompt_context();

        assert!(prompt.contains("round 2/3"));
        assert!(prompt.contains("50.0/100"));
        assert!(prompt.contains("CRITICAL"));
        assert!(prompt.contains("hardcoded secret"));
        assert!(prompt.contains("MEDIUM"));
    }

    #[test]
    fn test_feedback_with_test_output() {
        let mut fb = VerificationFeedback::new(50.0, 70.0, vec![], 1, 3);
        fb.test_output = Some("2 tests failed".into());
        let prompt = fb.as_prompt_context();
        assert!(prompt.contains("Test Output"));
        assert!(prompt.contains("2 tests failed"));
    }
}
