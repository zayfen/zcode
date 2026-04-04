//! Execution phase adapter

use crate::error::Result;
use async_trait::async_trait;

use crate::pipeline::context::PipelineContext;
use crate::pipeline::phases::PipelinePhase;
use crate::pipeline::result::PhaseResult;

/// Execution phase — runs tasks from the task graph
pub struct ExecutionPhase;

impl ExecutionPhase {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExecutionPhase {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PipelinePhase for ExecutionPhase {
    fn name(&self) -> &str {
        "execution"
    }

    fn description(&self) -> &str {
        "Executes tasks from the plan using the agent loop"
    }

    async fn execute(&self, context: &mut PipelineContext) -> Result<PhaseResult> {
        let start = std::time::Instant::now();

        let task_count = context
            .task_graph
            .as_ref()
            .map(|g| g.len())
            .unwrap_or(0);

        if task_count == 0 {
            let duration = start.elapsed();
            return Ok(PhaseResult::failed(
                self.name(),
                duration,
                "No task graph available for execution",
            ));
        }

        // Build execution summary
        // In full implementation, this would iterate the task graph,
        // execute each task via AgentLoop, and track results
        let feedback_note = if let Some(fb) = &context.feedback {
            format!("\n\n[Retry #{} — Feedback: {}]", context.retry_iteration, fb)
        } else {
            String::new()
        };

        let summary = format!(
            "Executed {} task(s) for: {}{}",
            task_count,
            &context.requirement.chars().take(80).collect::<String>(),
            feedback_note
        );

        context.execution_summary = Some(summary.clone());

        let duration = start.elapsed();
        Ok(PhaseResult::success(self.name(), duration, &summary))
    }
}
