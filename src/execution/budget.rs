//! ExecutionBudget — resource limits for task execution
//!
//! Tracks tokens, iterations, tool calls, cost, and time.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// Execution budget — limits for a single task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionBudget {
    /// Maximum LLM token consumption
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Maximum agent loop iterations
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,

    /// Maximum estimated cost in USD
    #[serde(default = "default_max_cost")]
    pub max_cost_usd: f64,

    /// Maximum execution duration
    #[serde(
        default = "default_max_duration",
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub max_duration: Duration,

    /// Maximum tool call count
    #[serde(default = "default_max_tool_calls")]
    pub max_tool_calls: u32,
}

fn default_max_tokens() -> u32 { 100_000 }
fn default_max_iterations() -> u32 { 20 }
fn default_max_cost() -> f64 { 1.0 }
fn default_max_duration() -> Duration { Duration::from_secs(300) }
fn default_max_tool_calls() -> u32 { 50 }

impl Default for ExecutionBudget {
    fn default() -> Self {
        Self {
            max_tokens: default_max_tokens(),
            max_iterations: default_max_iterations(),
            max_cost_usd: default_max_cost(),
            max_duration: default_max_duration(),
            max_tool_calls: default_max_tool_calls(),
        }
    }
}

fn serialize_duration<S: serde::Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u64(d.as_secs())
}

fn deserialize_duration<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
    Ok(Duration::from_secs(u64::deserialize(d)?))
}

/// Atomic budget tracker — thread-safe real-time tracking
pub struct BudgetTracker {
    budget: ExecutionBudget,
    tokens_used: AtomicU32,
    iterations_used: AtomicU32,
    tool_calls_used: AtomicU32,
    start_time: Instant,
}

impl BudgetTracker {
    /// Create a new tracker for the given budget
    pub fn new(budget: ExecutionBudget) -> Self {
        Self {
            budget,
            tokens_used: AtomicU32::new(0),
            iterations_used: AtomicU32::new(0),
            tool_calls_used: AtomicU32::new(0),
            start_time: Instant::now(),
        }
    }

    /// Check if the budget has been exceeded
    pub fn is_exceeded(&self) -> bool {
        self.tokens_used.load(Ordering::Relaxed) >= self.budget.max_tokens
            || self.iterations_used.load(Ordering::Relaxed) >= self.budget.max_iterations
            || self.tool_calls_used.load(Ordering::Relaxed) >= self.budget.max_tool_calls
            || self.start_time.elapsed() >= self.budget.max_duration
    }

    /// Record token usage
    pub fn record_tokens(&self, count: u32) {
        self.tokens_used.fetch_add(count, Ordering::Relaxed);
    }

    /// Record an iteration
    pub fn record_iteration(&self) {
        self.iterations_used.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a tool call
    pub fn record_tool_call(&self) {
        self.tool_calls_used.fetch_add(1, Ordering::Relaxed);
    }

    /// Get remaining tokens
    pub fn remaining_tokens(&self) -> u32 {
        self.budget.max_tokens.saturating_sub(self.tokens_used.load(Ordering::Relaxed))
    }

    /// Get remaining iterations
    pub fn remaining_iterations(&self) -> u32 {
        self.budget.max_iterations.saturating_sub(self.iterations_used.load(Ordering::Relaxed))
    }

    /// Generate a usage report
    pub fn report(&self) -> BudgetReport {
        let elapsed = self.start_time.elapsed();
        let tokens_used = self.tokens_used.load(Ordering::Relaxed);
        let iterations_used = self.iterations_used.load(Ordering::Relaxed);
        let tool_calls_used = self.tool_calls_used.load(Ordering::Relaxed);

        // Rough cost estimate: Claude Sonnet ~$3/M input, $15/M output tokens
        let estimated_cost = (tokens_used as f64) * 9.0 / 1_000_000.0;

        BudgetReport {
            tokens_used,
            tokens_budget: self.budget.max_tokens,
            iterations_used,
            iterations_budget: self.budget.max_iterations,
            tool_calls_used,
            tool_calls_budget: self.budget.max_tool_calls,
            elapsed,
            duration_budget: self.budget.max_duration,
            estimated_cost,
            cost_budget: self.budget.max_cost_usd,
        }
    }
}

/// Budget usage report
#[derive(Debug, Clone)]
pub struct BudgetReport {
    pub tokens_used: u32,
    pub tokens_budget: u32,
    pub iterations_used: u32,
    pub iterations_budget: u32,
    pub tool_calls_used: u32,
    pub tool_calls_budget: u32,
    pub elapsed: Duration,
    pub duration_budget: Duration,
    pub estimated_cost: f64,
    pub cost_budget: f64,
}

impl BudgetReport {
    /// Token usage percentage (0.0 - 1.0)
    pub fn token_usage_pct(&self) -> f64 {
        if self.tokens_budget == 0 { return 0.0; }
        self.tokens_used as f64 / self.tokens_budget as f64
    }

    /// Whether any budget dimension is over 80%
    pub fn is_near_limit(&self) -> bool {
        self.token_usage_pct() > 0.8
            || self.elapsed.as_secs_f64() / self.duration_budget.as_secs_f64() > 0.8
    }

    /// Format as a human-readable string
    pub fn to_summary(&self) -> String {
        format!(
            "Tokens: {}/{} ({:.0}%) | Iterations: {}/{} | Tool calls: {}/{} | Time: {:.0}s/{:.0}s | Cost: ${:.4}/${:.2}",
            self.tokens_used, self.tokens_budget, self.token_usage_pct() * 100.0,
            self.iterations_used, self.iterations_budget,
            self.tool_calls_used, self.tool_calls_budget,
            self.elapsed.as_secs_f64(), self.duration_budget.as_secs_f64(),
            self.estimated_cost, self.cost_budget,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_budget() {
        let b = ExecutionBudget::default();
        assert_eq!(b.max_tokens, 100_000);
        assert_eq!(b.max_iterations, 20);
        assert_eq!(b.max_tool_calls, 50);
        assert_eq!(b.max_duration, Duration::from_secs(300));
    }

    #[test]
    fn test_budget_not_exceeded() {
        let tracker = BudgetTracker::new(ExecutionBudget::default());
        assert!(!tracker.is_exceeded());
    }

    #[test]
    fn test_budget_exceeded_tokens() {
        let budget = ExecutionBudget {
            max_tokens: 100,
            ..Default::default()
        };
        let tracker = BudgetTracker::new(budget);
        tracker.record_tokens(100);
        assert!(tracker.is_exceeded());
    }

    #[test]
    fn test_budget_exceeded_iterations() {
        let budget = ExecutionBudget {
            max_iterations: 2,
            ..Default::default()
        };
        let tracker = BudgetTracker::new(budget);
        tracker.record_iteration();
        tracker.record_iteration();
        assert!(tracker.is_exceeded());
    }

    #[test]
    fn test_remaining() {
        let budget = ExecutionBudget {
            max_tokens: 1000,
            ..Default::default()
        };
        let tracker = BudgetTracker::new(budget);
        tracker.record_tokens(300);
        assert_eq!(tracker.remaining_tokens(), 700);
    }

    #[test]
    fn test_report() {
        let tracker = BudgetTracker::new(ExecutionBudget::default());
        tracker.record_tokens(500);
        tracker.record_iteration();
        tracker.record_tool_call();
        let report = tracker.report();
        assert_eq!(report.tokens_used, 500);
        assert_eq!(report.iterations_used, 1);
        assert_eq!(report.tool_calls_used, 1);
        assert!(!report.is_near_limit());
    }

    #[test]
    fn test_report_summary() {
        let tracker = BudgetTracker::new(ExecutionBudget::default());
        tracker.record_tokens(500);
        let report = tracker.report();
        let summary = report.to_summary();
        assert!(summary.contains("Tokens: 500/100000"));
    }

    #[test]
    fn test_budget_serialization() {
        let b = ExecutionBudget::default();
        let json = serde_json::to_string(&b).unwrap();
        let back: ExecutionBudget = serde_json::from_str(&json).unwrap();
        assert_eq!(back.max_tokens, b.max_tokens);
        assert_eq!(back.max_duration, b.max_duration);
    }
}
