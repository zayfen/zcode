//! Pipeline Orchestrator — ties Cognition → Plan → Execute → Verify → Deliver
//!
//! The Pipeline is the top-level orchestrator that runs phases sequentially,
//! handles retry loops (verification → execution), persists state for crash
//! recovery, and collects metrics.

pub mod config;
pub mod context;
pub mod result;
pub mod state;
pub mod hooks;
pub mod phases;

pub use config::PipelineConfig;
pub use context::PipelineContext;
pub use result::{PipelineResult, PipelineMetrics, PhaseResult, PhaseStatus, PhaseMetrics, TokenUsage};
pub use state::PipelineState;
pub use hooks::{PipelineHooks, HookConfig, HookAction};
pub use phases::{
    PipelinePhase, CognitionPhase, PlanningPhase, ExecutionPhase,
    VerificationPhase, DeliveryPhase,
};

use crate::error::{Result, ZcodeError};
use std::collections::HashMap;
use std::path::Path;

/// The pipeline orchestrator
pub struct Pipeline {
    config: PipelineConfig,
    hooks: PipelineHooks,
}

impl Pipeline {
    /// Create a new pipeline with the given config
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            config,
            hooks: PipelineHooks::new(),
        }
    }

    /// Create with default config
    pub fn default_pipeline() -> Self {
        Self::new(PipelineConfig::default())
    }

    /// Register a hook
    pub fn add_hook(&mut self, hook: HookConfig) {
        self.hooks.register(hook);
    }

    /// Run the full pipeline
    pub async fn run(&self, requirement: &str, project_root: &Path) -> Result<PipelineResult> {
        let mut ctx = PipelineContext::new(requirement, project_root);
        ctx.metrics.started_at = Some(chrono::Utc::now().timestamp());

        // Initialize state for crash recovery
        let state_id = format!("pipe-{}", uuid::Uuid::new_v4());
        let mut state = PipelineState::new(&state_id, requirement, project_root);

        // Run hooks
        let env = HashMap::new();
        self.hooks.run_hooks("before_pipeline", &env).await;

        // Build ordered phases
        let phases: Vec<Box<dyn PipelinePhase>> = self.build_phases();
        let mut phase_results: Vec<PhaseResult> = Vec::new();

        let mut phase_idx = 0;
        while phase_idx < phases.len() {
            let phase = &phases[phase_idx];
            let phase_name = phase.name();

            tracing::info!("▶ Phase: {} — {}", phase_name, phase.description());

            // Save state
            state.set_current_phase(phase_name, project_root)?;

            // Execute phase
            let result = phase.execute(&mut ctx).await?;

            // Record metrics
            let now_ts = chrono::Utc::now().timestamp();
            let duration = result.duration;
            ctx.metrics.record_phase(PhaseMetrics {
                phase_name: phase_name.to_string(),
                duration,
                tokens: result.tokens_used.clone(),
                status: result.status.clone(),
                started_at: now_ts - duration.as_secs() as i64,
                finished_at: now_ts,
            });

            tracing::info!("✓ Phase {} completed: {}", phase_name, result.summary);

            // Handle status — clone first to release borrow on `result`
            let status = result.status.clone();
            match status {
                PhaseStatus::Success => {
                    state.complete_phase(phase_name, project_root)?;
                    phase_results.push(result);
                    phase_idx += 1;
                }
                PhaseStatus::Failed => {
                    let is_optional = self.is_phase_optional(phase_name);
                    if is_optional {
                        tracing::warn!("Optional phase {} failed, continuing", phase_name);
                        phase_results.push(result);
                        phase_idx += 1;
                    } else {
                        tracing::error!("Required phase {} failed", phase_name);
                        phase_results.push(result);
                        break;
                    }
                }
                PhaseStatus::Skipped => {
                    phase_results.push(result);
                    phase_idx += 1;
                }
                PhaseStatus::Retry { target_phase, reason } => {
                    tracing::warn!("Phase {} requests retry to {}: {}", phase_name, target_phase, reason);
                    ctx.feedback = Some(reason);
                    phase_results.push(result);

                    // Find target phase
                    if let Some(target_idx) = phases.iter().position(|p| p.name() == &target_phase) {
                        phase_idx = target_idx;
                    } else {
                        tracing::error!("Retry target '{}' not found", target_phase);
                        break;
                    }
                }
            }
        }

        ctx.metrics.finished_at = Some(chrono::Utc::now().timestamp());
        ctx.metrics.total_duration = Some(
            ctx.metrics.phase_metrics.iter().map(|m| m.duration).sum()
        );
        ctx.metrics.estimated_cost_usd = estimate_cost(ctx.metrics.total_tokens);

        // Run after hooks
        self.hooks.run_hooks("after_pipeline", &env).await;

        // Clear state on success
        let success = phase_results.iter().all(|r| r.is_success() || matches!(r.status, PhaseStatus::Skipped));
        if success {
            let _ = PipelineState::clear(project_root);
        }

        Ok(PipelineResult::new(success, requirement.to_string(), phase_results, ctx.metrics))
    }

    /// Resume from a previously saved state
    pub async fn resume(&self, project_root: &Path) -> Result<PipelineResult> {
        let state = PipelineState::load(project_root)?
            .ok_or_else(|| ZcodeError::InternalError("No pipeline state found for resume".into()))?;

        tracing::info!("Resuming pipeline '{}' from phase '{}'",
            state.id,
            state.current_phase.as_deref().unwrap_or("unknown"));

        // Re-run from the requirement
        self.run(&state.requirement, project_root).await
    }

    /// Build the ordered list of phases from config
    fn build_phases(&self) -> Vec<Box<dyn PipelinePhase>> {
        let mut phases: Vec<Box<dyn PipelinePhase>> = Vec::new();

        if self.config.cognition.enabled {
            phases.push(Box::new(CognitionPhase::new()));
        }
        if self.config.planning.enabled {
            phases.push(Box::new(PlanningPhase::new()));
        }
        if self.config.execution.enabled {
            phases.push(Box::new(ExecutionPhase::new()));
        }
        if self.config.verification.enabled {
            phases.push(Box::new(VerificationPhase::new(
                self.config.verification.min_score,
                self.config.verification.max_retries,
            )));
        }
        if self.config.delivery.enabled {
            phases.push(Box::new(DeliveryPhase::new()));
        }

        phases
    }

    /// Check if a phase is optional
    fn is_phase_optional(&self, phase_name: &str) -> bool {
        match phase_name {
            "cognition" => self.config.cognition.optional,
            "planning" => self.config.planning.optional,
            "execution" => self.config.execution.optional,
            "verification" => self.config.verification.optional,
            "delivery" => self.config.delivery.optional,
            _ => false,
        }
    }
}

/// Estimate cost from tokens (rough: $9/M tokens blended)
fn estimate_cost(total_tokens: u64) -> f64 {
    (total_tokens as f64) * 9.0 / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_pipeline() {
        let p = Pipeline::default_pipeline();
        let phases = p.build_phases();
        assert_eq!(phases.len(), 5);
        assert_eq!(phases[0].name(), "cognition");
        assert_eq!(phases[1].name(), "planning");
        assert_eq!(phases[2].name(), "execution");
        assert_eq!(phases[3].name(), "verification");
        assert_eq!(phases[4].name(), "delivery");
    }

    #[test]
    fn test_pipeline_with_disabled_phases() {
        let mut config = PipelineConfig::default();
        config.cognition.enabled = false;
        config.delivery.enabled = false;
        let p = Pipeline::new(config);
        let phases = p.build_phases();
        assert_eq!(phases.len(), 3);
        assert_eq!(phases[0].name(), "planning");
    }

    #[test]
    fn test_estimate_cost() {
        assert_eq!(estimate_cost(100_000), 0.9);
        assert_eq!(estimate_cost(1_000_000), 9.0);
    }

    #[test]
    fn test_is_phase_optional() {
        let p = Pipeline::default_pipeline();
        assert!(p.is_phase_optional("cognition")); // cognition is optional by default
        assert!(!p.is_phase_optional("execution"));
        assert!(!p.is_phase_optional("unknown_phase"));
    }

    #[test]
    fn test_pipeline_config_default() {
        let c = PipelineConfig::default();
        assert!(c.cognition.enabled);
        assert!(c.planning.enabled);
    }
}
