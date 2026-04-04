//! Pipeline context — shared state passed between phases

use crate::delivery::DeliveryResult;
use crate::execution::{TaskGraph, TaskId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::result::PipelineMetrics;

/// Shared pipeline context — data flows between phases through this
#[derive(Debug, Clone)]
pub struct PipelineContext {
    // ─── Input ───
    /// Original user requirement
    pub requirement: String,
    /// Project root path
    pub project_root: PathBuf,

    // ─── Cognition phase output ───
    /// Knowledge context from cognition engine
    pub knowledge_context: Option<String>,

    // ─── Planning phase output ───
    /// Task graph from planning
    pub task_graph: Option<TaskGraph>,

    // ─── Execution phase output ───
    /// Execution summary text
    pub execution_summary: Option<String>,

    // ─── Verification phase output ───
    /// Verification scores per task
    pub verification_results: HashMap<TaskId, f64>,
    /// Current retry iteration
    pub retry_iteration: u32,
    /// Feedback for next execution round
    pub feedback: Option<String>,

    // ─── Delivery phase output ───
    /// Delivery result
    pub delivery_result: Option<DeliveryResult>,

    // ─── Metrics ───
    /// Pipeline metrics
    pub metrics: PipelineMetrics,
}

impl PipelineContext {
    /// Create a new context for a pipeline run
    pub fn new(requirement: &str, project_root: &Path) -> Self {
        Self {
            requirement: requirement.to_string(),
            project_root: project_root.to_path_buf(),
            knowledge_context: None,
            task_graph: None,
            execution_summary: None,
            verification_results: HashMap::new(),
            retry_iteration: 0,
            feedback: None,
            delivery_result: None,
            metrics: PipelineMetrics::default(),
        }
    }

    /// Get the current phase name (based on what's been completed)
    pub fn current_phase(&self) -> &str {
        if self.delivery_result.is_some() {
            "delivery"
        } else if !self.verification_results.is_empty() {
            "verification"
        } else if self.execution_summary.is_some() {
            "execution"
        } else if self.task_graph.is_some() {
            "planning"
        } else if self.knowledge_context.is_some() {
            "cognition"
        } else {
            "start"
        }
    }

    /// Record a verification score for a task
    pub fn record_verification(&mut self, task_id: TaskId, score: f64) {
        self.verification_results.insert(task_id, score);
    }

    /// Average verification score across all tasks
    pub fn avg_verification_score(&self) -> f64 {
        if self.verification_results.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.verification_results.values().sum();
        sum / self.verification_results.len() as f64
    }

    /// Count of tasks above threshold
    pub fn tasks_above_threshold(&self, threshold: f64) -> usize {
        self.verification_results
            .values()
            .filter(|&&s| s >= threshold)
            .count()
    }
}

/// Serializable pipeline context snapshot for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineContextSnapshot {
    pub requirement: String,
    pub project_root: String,
    pub knowledge_context: Option<String>,
    pub execution_summary: Option<String>,
    pub retry_iteration: u32,
    pub feedback: Option<String>,
    pub completed_phases: Vec<String>,
    pub current_phase: Option<String>,
}

impl PipelineContextSnapshot {
    /// Create a snapshot from context
    pub fn from_context(ctx: &PipelineContext) -> Self {
        let mut completed = Vec::new();
        if ctx.knowledge_context.is_some() {
            completed.push("cognition".to_string());
        }
        if ctx.task_graph.is_some() {
            completed.push("planning".to_string());
        }
        if ctx.execution_summary.is_some() {
            completed.push("execution".to_string());
        }
        if !ctx.verification_results.is_empty() {
            completed.push("verification".to_string());
        }
        if ctx.delivery_result.is_some() {
            completed.push("delivery".to_string());
        }

        Self {
            requirement: ctx.requirement.clone(),
            project_root: ctx.project_root.to_string_lossy().to_string(),
            knowledge_context: ctx.knowledge_context.clone(),
            execution_summary: ctx.execution_summary.clone(),
            retry_iteration: ctx.retry_iteration,
            feedback: ctx.feedback.clone(),
            completed_phases: completed,
            current_phase: Some(ctx.current_phase().to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_context_new() {
        let ctx = PipelineContext::new("Add auth", Path::new("/project"));
        assert_eq!(ctx.requirement, "Add auth");
        assert_eq!(ctx.current_phase(), "start");
    }

    #[test]
    fn test_context_phase_tracking() {
        let mut ctx = PipelineContext::new("test", Path::new("/tmp"));
        assert_eq!(ctx.current_phase(), "start");

        ctx.knowledge_context = Some("knowledge".into());
        assert_eq!(ctx.current_phase(), "cognition");

        ctx.execution_summary = Some("done".into());
        assert_eq!(ctx.current_phase(), "execution");
    }

    #[test]
    fn test_verification_scoring() {
        let mut ctx = PipelineContext::new("test", Path::new("/tmp"));
        ctx.record_verification(TaskId("A".into()), 80.0);
        ctx.record_verification(TaskId("B".into()), 60.0);

        assert_eq!(ctx.avg_verification_score(), 70.0);
        assert_eq!(ctx.tasks_above_threshold(70.0), 1);
    }

    #[test]
    fn test_snapshot() {
        let mut ctx = PipelineContext::new("Add auth", Path::new("/project"));
        ctx.knowledge_context = Some("knowledge".into());
        let snap = PipelineContextSnapshot::from_context(&ctx);
        assert_eq!(snap.completed_phases, vec!["cognition"]);
    }

    #[test]
    fn test_empty_avg_score() {
        let ctx = PipelineContext::new("test", Path::new("/tmp"));
        assert_eq!(ctx.avg_verification_score(), 0.0);
    }
}
