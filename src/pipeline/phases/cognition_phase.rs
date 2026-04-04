//! Cognition phase adapter

use crate::error::Result;
use async_trait::async_trait;

use crate::pipeline::context::PipelineContext;
use crate::pipeline::phases::PipelinePhase;
use crate::pipeline::result::PhaseResult;

/// Cognition phase — gathers knowledge and context about the project
pub struct CognitionPhase;

impl CognitionPhase {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CognitionPhase {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PipelinePhase for CognitionPhase {
    fn name(&self) -> &str {
        "cognition"
    }

    fn description(&self) -> &str {
        "Gathers project knowledge and builds context for planning"
    }

    async fn execute(&self, context: &mut PipelineContext) -> Result<PhaseResult> {
        let start = std::time::Instant::now();

        // For now, generate a basic knowledge context from the workspace
        // In full implementation, this would use the CognitionEngine
        let knowledge = format!(
            "Project: {}\nRequirement: {}\n\nAnalyze the project structure and gather relevant context for the task.",
            context.project_root.display(),
            context.requirement
        );

        context.knowledge_context = Some(knowledge);

        let duration = start.elapsed();
        Ok(PhaseResult::success(
            self.name(),
            duration,
            "Gathered project knowledge context",
        ))
    }
}
