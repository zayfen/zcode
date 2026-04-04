//! Pipeline phases — each stage of the harness engineering pipeline

pub mod cognition_phase;
pub mod planning_phase;
pub mod execution_phase;
pub mod verification_phase;
pub mod delivery_phase;

pub use cognition_phase::CognitionPhase;
pub use planning_phase::PlanningPhase;
pub use execution_phase::ExecutionPhase;
pub use verification_phase::VerificationPhase;
pub use delivery_phase::DeliveryPhase;

use crate::error::Result;
use async_trait::async_trait;

use super::context::PipelineContext;
use super::result::PhaseResult;

/// A single pipeline phase
#[async_trait]
pub trait PipelinePhase: Send + Sync {
    /// Phase name (e.g. "cognition", "execution")
    fn name(&self) -> &str;

    /// Human-readable description
    fn description(&self) -> &str;

    /// Execute this phase, reading/writing the shared context
    async fn execute(&self, context: &mut PipelineContext) -> Result<PhaseResult>;
}
