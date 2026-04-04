//! Verification policy — controls verification behavior

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Verification policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationPolicy {
    /// Minimum passing score (0.0 - 100.0), default 70.0
    #[serde(default = "default_min_score")]
    pub min_score: f64,

    /// Maximum retries (verify → feedback → re-execute → re-verify), default 3
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Enabled verifiers by name (empty = all enabled)
    #[serde(default)]
    pub enabled_verifiers: Vec<String>,

    /// Weight overrides per verifier
    #[serde(default)]
    pub weight_overrides: HashMap<String, f64>,

    /// Inject previous verification results as feedback on retry
    #[serde(default = "bool_true")]
    pub inject_feedback: bool,

    /// Run full verification at gate (vs incremental)
    #[serde(default = "bool_true")]
    pub full_gate_verification: bool,
}

impl Default for VerificationPolicy {
    fn default() -> Self {
        Self {
            min_score: default_min_score(),
            max_retries: default_max_retries(),
            enabled_verifiers: vec![],
            weight_overrides: HashMap::new(),
            inject_feedback: true,
            full_gate_verification: true,
        }
    }
}

impl VerificationPolicy {
    /// Check if a verifier is enabled
    pub fn is_verifier_enabled(&self, name: &str) -> bool {
        if self.enabled_verifiers.is_empty() {
            return true;
        }
        self.enabled_verifiers.iter().any(|v| v == name)
    }

    /// Get weight for a verifier (with override)
    pub fn verifier_weight(&self, name: &str, default_weight: f64) -> f64 {
        self.weight_overrides.get(name).copied().unwrap_or(default_weight)
    }
}

fn default_min_score() -> f64 { 70.0 }
fn default_max_retries() -> u32 { 3 }
fn bool_true() -> bool { true }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy() {
        let p = VerificationPolicy::default();
        assert_eq!(p.min_score, 70.0);
        assert_eq!(p.max_retries, 3);
        assert!(p.inject_feedback);
        assert!(p.full_gate_verification);
    }

    #[test]
    fn test_verifier_enabled_all() {
        let p = VerificationPolicy::default();
        assert!(p.is_verifier_enabled("anything"));
    }

    #[test]
    fn test_verifier_enabled_specific() {
        let p = VerificationPolicy {
            enabled_verifiers: vec!["test".into(), "lint".into()],
            ..Default::default()
        };
        assert!(p.is_verifier_enabled("test"));
        assert!(p.is_verifier_enabled("lint"));
        assert!(!p.is_verifier_enabled("coverage"));
    }

    #[test]
    fn test_verifier_weight_default() {
        let p = VerificationPolicy::default();
        assert_eq!(p.verifier_weight("test", 0.3), 0.3);
    }

    #[test]
    fn test_verifier_weight_override() {
        let mut overrides = HashMap::new();
        overrides.insert("test".into(), 0.5);
        let p = VerificationPolicy {
            weight_overrides: overrides,
            ..Default::default()
        };
        assert_eq!(p.verifier_weight("test", 0.3), 0.5);
    }

    #[test]
    fn test_policy_serialization_roundtrip() {
        let p = VerificationPolicy::default();
        let json = serde_json::to_string(&p).unwrap();
        let back: VerificationPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(back.min_score, p.min_score);
        assert_eq!(back.max_retries, p.max_retries);
    }
}
