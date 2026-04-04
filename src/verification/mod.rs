//! Verification module — pluggable verification pipeline with scoring and feedback
//!
//! # Overview
//!
//! This module implements the verification pipeline as described in the Harness Engineering
//! framework. It provides:
//!
//! - **Verifier trait**: A common interface for all verification logic
//! - **Built-in verifiers**: TestVerifier, LintVerifier, ReviewerVerifier
//! - **Scoring engine**: Weighted score aggregation with configurable thresholds
//! - **Feedback loop**: Constructs actionable feedback for re-execution
//!
//! # Example
//!
//! ```rust,no_run
//! use zcode::verification::{VerificationPipeline, VerificationContext, VerificationPolicy};
//!
//! # async fn example() {
//! let pipeline = VerificationPipeline::new();
//! let ctx = VerificationContext {
//!     requirement: "Add auth".into(),
//!     task_description: "Implement JWT".into(),
//!     pre_snapshot_id: None,
//!     diff_patch: "+fn verify_token() {}".into(),
//!     changed_files: vec![],
//!     project_root: std::path::PathBuf::from("/project"),
//! };
//!
//! let (score, feedback) = pipeline.verify(&ctx).await;
//! if score.passed {
//!     println!("Verification passed: {:.1}/100", score.total);
//! } else if let Some(fb) = feedback {
//!     println!("Need to fix: {}", fb.as_prompt_context());
//! }
//! # }
//! ```

pub mod types;
pub mod policy;
pub mod feedback;
pub mod scoring;
pub mod pipeline;
pub mod verifiers;

// Re-exports
pub use types::{
    VerificationContext, VerificationIssue, VerificationResult, VerificationScore,
    VerifierScoreEntry, IssueSeverity, FileLocation,
};
pub use policy::VerificationPolicy;
pub use feedback::VerificationFeedback;
pub use pipeline::{VerificationPipeline, PipelineVerificationResult};
pub use verifiers::Verifier;
pub use verifiers::TestVerifier;
pub use verifiers::LintVerifier;
pub use verifiers::ReviewerVerifier;
