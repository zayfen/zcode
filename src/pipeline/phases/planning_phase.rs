//! Planning phase adapter

use crate::error::Result;
use async_trait::async_trait;

use crate::execution::graph::{TaskGraph, TaskId};
use crate::agent::Task;
use crate::pipeline::context::PipelineContext;
use crate::pipeline::phases::PipelinePhase;
use crate::pipeline::result::PhaseResult;

/// Planning phase — breaks requirement into a task graph
pub struct PlanningPhase;

impl PlanningPhase {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlanningPhase {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PipelinePhase for PlanningPhase {
    fn name(&self) -> &str {
        "planning"
    }

    fn description(&self) -> &str {
        "Breaks the requirement into an executable task graph"
    }

    async fn execute(&self, context: &mut PipelineContext) -> Result<PhaseResult> {
        let start = std::time::Instant::now();

        // Create a simple single-task plan from the requirement
        // In full implementation, this would use PlannerAgent with LLM
        let task = Task::new(&context.requirement);
        let tasks = vec![task];
        let deps: Vec<(TaskId, TaskId)> = vec![];

        match TaskGraph::build(tasks, deps) {
            Ok(graph) => {
                let task_count = graph.len();
                context.task_graph = Some(graph);

                let duration = start.elapsed();
                Ok(PhaseResult::success(
                    self.name(),
                    duration,
                    &format!("Created plan with {} task(s)", task_count),
                ))
            }
            Err(e) => {
                let duration = start.elapsed();
                Ok(PhaseResult::failed(
                    self.name(),
                    duration,
                    &format!("Planning failed: {}", e),
                ))
            }
        }
    }
}
