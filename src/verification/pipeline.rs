//! VerificationPipeline — orchestrates verifiers and manages retry loop

use crate::verification::feedback::VerificationFeedback;
use crate::verification::policy::VerificationPolicy;
use crate::verification::scoring::aggregate_scores;
use crate::verification::types::{
    VerificationContext, VerificationResult, VerificationScore,
};
use crate::verification::verifiers::{LintVerifier, ReviewerVerifier, TestVerifier, Verifier};
/// The verification pipeline — runs verifiers, aggregates scores, manages feedback
pub struct VerificationPipeline {
    verifiers: Vec<Box<dyn Verifier>>,
    policy: VerificationPolicy,
}

/// Result of a full verification-with-retry cycle
#[derive(Debug)]
pub enum PipelineVerificationResult {
    Passed(VerificationScore),
    Failed(VerificationScore),
}

impl VerificationPipeline {
    /// Create a new pipeline with default verifiers and policy
    pub fn new() -> Self {
        Self::with_policy(VerificationPolicy::default())
    }

    /// Create a pipeline with a custom policy and default verifiers
    pub fn with_policy(policy: VerificationPolicy) -> Self {
        let mut pipeline = Self {
            verifiers: Vec::new(),
            policy,
        };
        pipeline.add_default_verifiers();
        pipeline
    }

    /// Create a pipeline with custom verifiers and policy
    pub fn with_verifiers(verifiers: Vec<Box<dyn Verifier>>, policy: VerificationPolicy) -> Self {
        Self {
            verifiers,
            policy,
        }
    }

    /// Add the default set of verifiers
    fn add_default_verifiers(&mut self) {
        self.add_verifier(Box::new(TestVerifier::new()));
        self.add_verifier(Box::new(LintVerifier::new()));
        self.add_verifier(Box::new(ReviewerVerifier::new()));
    }

    /// Add a verifier
    pub fn add_verifier(&mut self, verifier: Box<dyn Verifier>) {
        self.verifiers.push(verifier);
    }

    /// Get reference to policy
    pub fn policy(&self) -> &VerificationPolicy {
        &self.policy
    }

    /// Run all enabled verifiers and aggregate scores
    pub async fn run_verifiers(&self, context: &VerificationContext) -> VerificationScore {
        let mut results = Vec::new();

        for verifier in &self.verifiers {
            if !self.policy.is_verifier_enabled(verifier.name()) {
                continue;
            }

            let result = verifier.verify(context).await;
            results.push(result);
        }

        aggregate_scores(&results, &self.policy)
    }

    /// Run verification with feedback construction (no re-execution)
    /// Returns the score and optionally a feedback object if score < min_score
    pub async fn verify(&self, context: &VerificationContext) -> (VerificationScore, Option<VerificationFeedback>) {
        let score = self.run_verifiers(context).await;

        if score.passed {
            (score, None)
        } else {
            let feedback = VerificationFeedback::new(
                score.total,
                self.policy.min_score,
                score.top_issues.clone(),
                0,
                self.policy.max_retries,
            );
            (score, Some(feedback))
        }
    }

    /// Extract test output from the score breakdown
    pub fn extract_test_output(&self, score: &VerificationScore) -> Option<String> {
        score
            .breakdown
            .iter()
            .find(|e| e.name == "test")
            .and_then(|_| {
                // In a full implementation, we'd store the log in the breakdown
                // For now, return None as the log is in VerificationResult
                None
            })
    }

    /// Get the number of enabled verifiers
    pub fn verifier_count(&self) -> usize {
        self.verifiers
            .iter()
            .filter(|v| self.policy.is_verifier_enabled(v.name()))
            .count()
    }
}

impl Default for VerificationPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verification::types::VerificationIssue;
    use async_trait::async_trait;
    use std::path::PathBuf;

    struct AlwaysPassVerifier;

    #[async_trait]
    impl Verifier for AlwaysPassVerifier {
        fn name(&self) -> &str { "always_pass" }
        fn description(&self) -> &str { "always passes" }
        fn weight(&self) -> f64 { 1.0 }
        async fn verify(&self, _ctx: &VerificationContext) -> VerificationResult {
            VerificationResult::passed("always_pass")
        }
    }

    struct AlwaysFailVerifier;

    #[async_trait]
    impl Verifier for AlwaysFailVerifier {
        fn name(&self) -> &str { "always_fail" }
        fn description(&self) -> &str { "always fails" }
        fn weight(&self) -> f64 { 1.0 }
        async fn verify(&self, _ctx: &VerificationContext) -> VerificationResult {
            VerificationResult::with_issues(
                "always_fail",
                30.0,
                vec![VerificationIssue::new(
                    crate::verification::types::IssueSeverity::Critical,
                    "test",
                    "always fails",
                )],
            )
        }
    }

    fn test_context() -> VerificationContext {
        VerificationContext {
            requirement: "test req".into(),
            task_description: "test task".into(),
            pre_snapshot_id: None,
            diff_patch: String::new(),
            changed_files: vec![],
            project_root: PathBuf::from("/tmp"),
        }
    }

    #[tokio::test]
    async fn test_pipeline_passes() {
        let pipeline = VerificationPipeline::with_verifiers(
            vec![Box::new(AlwaysPassVerifier)],
            VerificationPolicy::default(),
        );
        let (score, feedback) = pipeline.verify(&test_context()).await;
        assert!(score.passed);
        assert!(feedback.is_none());
    }

    #[tokio::test]
    async fn test_pipeline_fails() {
        let pipeline = VerificationPipeline::with_verifiers(
            vec![Box::new(AlwaysFailVerifier)],
            VerificationPolicy::default(),
        );
        let (score, feedback) = pipeline.verify(&test_context()).await;
        assert!(!score.passed);
        assert!(feedback.is_some());
        let fb = feedback.unwrap();
        assert_eq!(fb.score, 30.0);
    }

    #[tokio::test]
    async fn test_pipeline_with_enabled_filter() {
        let policy = VerificationPolicy {
            enabled_verifiers: vec!["always_pass".into()],
            ..Default::default()
        };
        let pipeline = VerificationPipeline::with_verifiers(
            vec![Box::new(AlwaysPassVerifier), Box::new(AlwaysFailVerifier)],
            policy,
        );
        // Only "always_pass" runs
        let score = pipeline.run_verifiers(&test_context()).await;
        assert!(score.passed);
    }

    #[tokio::test]
    async fn test_verifier_count() {
        let pipeline = VerificationPipeline::new();
        let count = pipeline.verifier_count();
        assert_eq!(count, 3); // test, lint, reviewer
    }

    #[tokio::test]
    async fn test_custom_min_score() {
        let policy = VerificationPolicy {
            min_score: 95.0,
            ..Default::default()
        };
        let pipeline = VerificationPipeline::with_verifiers(
            vec![Box::new(AlwaysPassVerifier)],
            policy,
        );
        let (score, _) = pipeline.verify(&test_context()).await;
        assert!(score.passed); // 100 >= 95
    }

    #[test]
    fn test_default_pipeline() {
        let p = VerificationPipeline::default();
        assert_eq!(p.verifiers.len(), 3);
        assert_eq!(p.policy().min_score, 70.0);
    }
}
