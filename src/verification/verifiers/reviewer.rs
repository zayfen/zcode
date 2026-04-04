//! ReviewerVerifier — wraps existing ReviewerAgent as a Verifier

use async_trait::async_trait;

use crate::agent::reviewer::{IssueSeverity as ReviewSeverity, ReviewCategory, ReviewerAgent};
use crate::verification::types::{IssueSeverity, VerificationContext, VerificationIssue, VerificationResult};
use super::Verifier;

/// Wraps the existing ReviewerAgent as a Verifier in the verification pipeline
pub struct ReviewerVerifier {
    reviewer: ReviewerAgent,
}

impl ReviewerVerifier {
    pub fn new() -> Self {
        Self {
            reviewer: ReviewerAgent::new(),
        }
    }

    /// Convert reviewer severity to verification severity
    fn convert_severity(severity: &ReviewSeverity) -> IssueSeverity {
        match severity {
            ReviewSeverity::Error => IssueSeverity::Critical,
            ReviewSeverity::Warning => IssueSeverity::High,
            ReviewSeverity::Info => IssueSeverity::Low,
            ReviewSeverity::Suggestion => IssueSeverity::Low,
        }
    }

    /// Convert reviewer category to string
    fn category_name(category: &ReviewCategory) -> &'static str {
        match category {
            ReviewCategory::Logic => "logic",
            ReviewCategory::Security => "security",
            ReviewCategory::Performance => "performance",
            ReviewCategory::Style => "style",
            ReviewCategory::Documentation => "documentation",
            ReviewCategory::Testing => "testing",
            ReviewCategory::Architecture => "architecture",
        }
    }
}

#[async_trait]
impl Verifier for ReviewerVerifier {
    fn name(&self) -> &str {
        "reviewer"
    }

    fn description(&self) -> &str {
        "Static analysis using built-in code review rules"
    }

    fn weight(&self) -> f64 {
        0.20
    }

    async fn verify(&self, context: &VerificationContext) -> VerificationResult {
        if context.diff_patch.is_empty() {
            return VerificationResult::passed(self.name());
        }

        let review = match self.reviewer.review_diff(&context.diff_patch) {
            Ok(r) => r,
            Err(e) => {
                return VerificationResult::with_issues(
                    self.name(),
                    0.0,
                    vec![VerificationIssue::new(
                        IssueSeverity::Critical,
                        "reviewer",
                        format!("Review failed: {}", e),
                    )],
                );
            }
        };

        // Convert issues
        let issues: Vec<VerificationIssue> = review
            .issues
            .iter()
            .map(|ri| {
                let severity = Self::convert_severity(&ri.severity);
                let category = Self::category_name(&ri.category);
                let mut issue = VerificationIssue::new(severity, category, &ri.message);

                if let Some(ref file) = ri.file {
                    issue = issue.with_location(file, ri.line.map(|l| l as usize));
                }

                issue
            })
            .collect();

        // Calculate score: start at 100, deduct per issue
        let mut deductions = 0.0;
        for issue in &issues {
            deductions += issue.deduction();
        }
        let score = (100.0 - deductions).max(0.0);

        VerificationResult {
            verifier_name: self.name().into(),
            score,
            issues,
            log: review.summary,
        }
    }
}

impl Default for ReviewerVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_severity() {
        assert_eq!(ReviewerVerifier::convert_severity(&ReviewSeverity::Error), IssueSeverity::Critical);
        assert_eq!(ReviewerVerifier::convert_severity(&ReviewSeverity::Warning), IssueSeverity::High);
        assert_eq!(ReviewerVerifier::convert_severity(&ReviewSeverity::Info), IssueSeverity::Low);
        assert_eq!(ReviewerVerifier::convert_severity(&ReviewSeverity::Suggestion), IssueSeverity::Low);
    }

    #[test]
    fn test_category_names() {
        assert_eq!(ReviewerVerifier::category_name(&ReviewCategory::Logic), "logic");
        assert_eq!(ReviewerVerifier::category_name(&ReviewCategory::Security), "security");
        assert_eq!(ReviewerVerifier::category_name(&ReviewCategory::Testing), "testing");
    }

    #[test]
    fn test_verifier_properties() {
        let v = ReviewerVerifier::new();
        assert_eq!(v.name(), "reviewer");
        assert_eq!(v.weight(), 0.20);
    }

    #[tokio::test]
    async fn test_verify_clean_diff() {
        let v = ReviewerVerifier::new();
        let ctx = VerificationContext {
            requirement: "test".into(),
            task_description: "test".into(),
            pre_snapshot_id: None,
            diff_patch: "+fn clean() -> bool { true }\n+#[test]\n+fn test_clean() {}".into(),
            changed_files: vec![],
            project_root: std::path::PathBuf::from("/tmp"),
        };
        let result = v.verify(&ctx).await;
        assert!(result.score >= 90.0);
    }

    #[tokio::test]
    async fn test_verify_unwrap_detected() {
        let v = ReviewerVerifier::new();
        let ctx = VerificationContext {
            requirement: "test".into(),
            task_description: "test".into(),
            pre_snapshot_id: None,
            diff_patch: "+let x = option.unwrap();".into(),
            changed_files: vec![],
            project_root: std::path::PathBuf::from("/tmp"),
        };
        let result = v.verify(&ctx).await;
        assert!(result.score < 100.0);
        assert!(!result.issues.is_empty());
    }

    #[tokio::test]
    async fn test_verify_empty_diff() {
        let v = ReviewerVerifier::new();
        let ctx = VerificationContext {
            requirement: "test".into(),
            task_description: "test".into(),
            pre_snapshot_id: None,
            diff_patch: String::new(),
            changed_files: vec![],
            project_root: std::path::PathBuf::from("/tmp"),
        };
        let result = v.verify(&ctx).await;
        assert_eq!(result.score, 100.0);
    }
}
