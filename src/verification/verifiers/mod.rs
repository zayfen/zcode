//! Verifier trait and verifier implementations

mod test;
mod lint;
mod reviewer;

pub use test::TestVerifier;
pub use lint::LintVerifier;
pub use reviewer::ReviewerVerifier;

use async_trait::async_trait;
use crate::verification::types::{VerificationContext, VerificationResult};

/// Verifier trait — all verification logic implements this interface
#[async_trait]
pub trait Verifier: Send + Sync {
    /// Verifier name
    fn name(&self) -> &str;

    /// Verifier description
    fn description(&self) -> &str;

    /// Weight in total score (all verifier weights are normalized)
    fn weight(&self) -> f64;

    /// Execute verification
    async fn verify(&self, context: &VerificationContext) -> VerificationResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockVerifier;

    #[async_trait]
    impl Verifier for MockVerifier {
        fn name(&self) -> &str { "mock" }
        fn description(&self) -> &str { "mock verifier" }
        fn weight(&self) -> f64 { 1.0 }

        async fn verify(&self, _ctx: &VerificationContext) -> VerificationResult {
            VerificationResult::passed("mock")
        }
    }

    #[tokio::test]
    async fn test_mock_verifier() {
        let v = MockVerifier;
        assert_eq!(v.name(), "mock");
        assert_eq!(v.weight(), 1.0);

        let ctx = VerificationContext {
            requirement: "test".into(),
            task_description: "test task".into(),
            pre_snapshot_id: None,
            diff_patch: String::new(),
            changed_files: vec![],
            project_root: std::path::PathBuf::from("/tmp"),
        };
        let result = v.verify(&ctx).await;
        assert_eq!(result.score, 100.0);
    }
}
