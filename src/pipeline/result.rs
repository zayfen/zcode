//! Pipeline result types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Status of a pipeline phase execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    /// Phase completed successfully
    Success,
    /// Phase failed
    Failed,
    /// Phase was skipped
    Skipped,
    /// Phase requests retry of a previous phase
    Retry { target_phase: String, reason: String },
}

impl std::fmt::Display for PhaseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
            Self::Retry { target_phase, reason } => {
                write!(f, "retry({}): {}", target_phase, reason)
            }
        }
    }
}

/// Token usage tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn new(input: u32, output: u32) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            total_tokens: input + output,
        }
    }
}

/// Result of a single phase execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResult {
    /// Phase name
    pub phase_name: String,
    /// Execution status
    pub status: PhaseStatus,
    /// Duration
    #[serde(
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub duration: Duration,
    /// Token usage
    pub tokens_used: TokenUsage,
    /// Summary of what happened
    pub summary: String,
    /// Phase-specific metadata
    pub metadata: HashMap<String, String>,
    /// Error message if failed
    pub error: Option<String>,
}

impl PhaseResult {
    /// Create a successful result
    pub fn success(name: &str, duration: Duration, summary: &str) -> Self {
        Self {
            phase_name: name.to_string(),
            status: PhaseStatus::Success,
            duration,
            tokens_used: TokenUsage::default(),
            summary: summary.to_string(),
            metadata: HashMap::new(),
            error: None,
        }
    }

    /// Create a failed result
    pub fn failed(name: &str, duration: Duration, error: &str) -> Self {
        Self {
            phase_name: name.to_string(),
            status: PhaseStatus::Failed,
            duration,
            tokens_used: TokenUsage::default(),
            summary: format!("Failed: {}", error),
            metadata: HashMap::new(),
            error: Some(error.to_string()),
        }
    }

    /// Create a skipped result
    pub fn skipped(name: &str, reason: &str) -> Self {
        Self {
            phase_name: name.to_string(),
            status: PhaseStatus::Skipped,
            duration: Duration::ZERO,
            tokens_used: TokenUsage::default(),
            summary: format!("Skipped: {}", reason),
            metadata: HashMap::new(),
            error: None,
        }
    }

    /// Create a retry result
    pub fn retry(name: &str, target: &str, reason: &str) -> Self {
        Self {
            phase_name: name.to_string(),
            status: PhaseStatus::Retry {
                target_phase: target.to_string(),
                reason: reason.to_string(),
            },
            duration: Duration::ZERO,
            tokens_used: TokenUsage::default(),
            summary: format!("Retry {} requested: {}", target, reason),
            metadata: HashMap::new(),
            error: None,
        }
    }

    /// Whether this phase succeeded
    pub fn is_success(&self) -> bool {
        matches!(self.status, PhaseStatus::Success)
    }
}

/// Single phase metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseMetrics {
    pub phase_name: String,
    #[serde(
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub duration: Duration,
    pub tokens: TokenUsage,
    pub status: PhaseStatus,
    pub started_at: i64,
    pub finished_at: i64,
}

/// Pipeline-level metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineMetrics {
    /// Per-phase metrics
    pub phase_metrics: Vec<PhaseMetrics>,
    /// Total token usage
    pub total_tokens: u64,
    /// Total duration
    #[serde(
        default,
        serialize_with = "serialize_duration_opt",
        deserialize_with = "deserialize_duration_opt"
    )]
    pub total_duration: Option<Duration>,
    /// Estimated cost in USD
    pub estimated_cost_usd: f64,
    /// Pipeline start timestamp
    pub started_at: Option<i64>,
    /// Pipeline end timestamp
    pub finished_at: Option<i64>,
}

impl PipelineMetrics {
    /// Add a phase metric
    pub fn record_phase(&mut self, metrics: PhaseMetrics) {
        self.total_tokens += metrics.tokens.total_tokens as u64;
        self.phase_metrics.push(metrics);
    }

    /// Human-readable summary
    pub fn summary(&self) -> String {
        let total_secs = self
            .total_duration
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        format!(
            "Pipeline: {:.1}s | {} tokens | ${:.4} | {} phases",
            total_secs,
            self.total_tokens,
            self.estimated_cost_usd,
            self.phase_metrics.len()
        )
    }
}

/// Final pipeline result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    /// Whether all phases succeeded
    pub success: bool,
    /// Original requirement
    pub requirement: String,
    /// Task counts
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub tasks_skipped: usize,
    /// Average verification score
    pub avg_verification_score: f64,
    /// Phase results
    pub phase_results: Vec<PhaseResult>,
    /// Metrics
    pub metrics: PipelineMetrics,
}

impl PipelineResult {
    /// Create a result from metrics and phase results
    pub fn new(
        success: bool,
        requirement: String,
        phase_results: Vec<PhaseResult>,
        metrics: PipelineMetrics,
    ) -> Self {
        Self {
            success,
            requirement,
            tasks_completed: 0,
            tasks_failed: 0,
            tasks_skipped: 0,
            avg_verification_score: 0.0,
            phase_results,
            metrics,
        }
    }

    /// Human-readable summary
    pub fn summary(&self) -> String {
        let status = if self.success { "SUCCESS" } else { "FAILED" };
        format!(
            "[{}] {} | {} completed, {} failed | score: {:.0} | {}",
            status,
            &self.requirement.chars().take(60).collect::<String>(),
            self.tasks_completed,
            self.tasks_failed,
            self.avg_verification_score,
            self.metrics.summary()
        )
    }
}

fn serialize_duration<S: serde::Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u64(d.as_millis() as u64)
}

fn deserialize_duration<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
    Ok(Duration::from_millis(u64::deserialize(d)?))
}

fn serialize_duration_opt<S: serde::Serializer>(
    d: &Option<Duration>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match d {
        Some(dur) => s.serialize_some(&(dur.as_millis() as u64)),
        None => s.serialize_none(),
    }
}

fn deserialize_duration_opt<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<Duration>, D::Error> {
    let opt: Option<u64> = Option::deserialize(d)?;
    Ok(opt.map(Duration::from_millis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_status_display() {
        assert_eq!(format!("{}", PhaseStatus::Success), "success");
        assert_eq!(format!("{}", PhaseStatus::Failed), "failed");
        assert_eq!(format!("{}", PhaseStatus::Skipped), "skipped");
    }

    #[test]
    fn test_phase_result_success() {
        let r = PhaseResult::success("test", Duration::from_secs(1), "done");
        assert!(r.is_success());
        assert_eq!(r.phase_name, "test");
        assert!(r.error.is_none());
    }

    #[test]
    fn test_phase_result_failed() {
        let r = PhaseResult::failed("test", Duration::from_secs(1), "error msg");
        assert!(!r.is_success());
        assert_eq!(r.error, Some("error msg".to_string()));
    }

    #[test]
    fn test_phase_result_skipped() {
        let r = PhaseResult::skipped("test", "not needed");
        assert_eq!(r.duration, Duration::ZERO);
    }

    #[test]
    fn test_token_usage() {
        let t = TokenUsage::new(100, 50);
        assert_eq!(t.total_tokens, 150);
    }

    #[test]
    fn test_pipeline_metrics() {
        let mut m = PipelineMetrics::default();
        m.record_phase(PhaseMetrics {
            phase_name: "test".into(),
            duration: Duration::from_secs(1),
            tokens: TokenUsage::new(100, 50),
            status: PhaseStatus::Success,
            started_at: 0,
            finished_at: 1,
        });
        assert_eq!(m.total_tokens, 150);
        assert_eq!(m.phase_metrics.len(), 1);
    }

    #[test]
    fn test_pipeline_result_summary() {
        let r = PipelineResult::new(
            true,
            "Add auth".into(),
            vec![],
            PipelineMetrics::default(),
        );
        assert!(r.summary().contains("SUCCESS"));
    }

    #[test]
    fn test_phase_result_serialization() {
        let r = PhaseResult::success("test", Duration::from_secs(2), "ok");
        let json = serde_json::to_string(&r).unwrap();
        let back: PhaseResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.phase_name, "test");
        assert_eq!(back.duration, Duration::from_secs(2));
    }
}
