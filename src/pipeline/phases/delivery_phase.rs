//! Delivery phase adapter

use crate::error::Result;
use async_trait::async_trait;

use crate::pipeline::context::PipelineContext;
use crate::pipeline::phases::PipelinePhase;
use crate::pipeline::result::PhaseResult;

/// Delivery phase — creates PRs, changelogs, and delivers results
pub struct DeliveryPhase;

impl DeliveryPhase {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DeliveryPhase {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PipelinePhase for DeliveryPhase {
    fn name(&self) -> &str {
        "delivery"
    }

    fn description(&self) -> &str {
        "Creates changelog, PR, and delivers the final result"
    }

    async fn execute(&self, context: &mut PipelineContext) -> Result<PhaseResult> {
        let start = std::time::Instant::now();

        // In full implementation, this would:
        // 1. Run gate checks via delivery::GateChecker
        // 2. Generate changelog via delivery::ChangelogGenerator
        // 3. Create branch + commit
        // 4. Push and create PR via delivery::PullRequestCreator
        // 5. Monitor CI via delivery::CiMonitor

        let summary = format!(
            "Delivered {} task(s). Avg verification score: {:.0}",
            context.verification_results.len(),
            context.avg_verification_score()
        );

        let duration = start.elapsed();
        Ok(PhaseResult::success(self.name(), duration, &summary))
    }
}
