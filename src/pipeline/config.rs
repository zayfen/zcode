//! Pipeline configuration

use serde::{Deserialize, Serialize};

/// Per-phase config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseConfig {
    /// Whether this phase is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Whether failure of this phase is optional (pipeline continues)
    #[serde(default)]
    pub optional: bool,
}

impl Default for PhaseConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            optional: false,
        }
    }
}

/// Pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Cognition phase config
    #[serde(default)]
    pub cognition: PhaseConfig,

    /// Planning phase config
    #[serde(default)]
    pub planning: PhaseConfig,

    /// Execution phase config
    #[serde(default)]
    pub execution: PhaseConfig,

    /// Verification phase config
    #[serde(default = "default_verification")]
    pub verification: VerificationPhaseConfig,

    /// Delivery phase config
    #[serde(default)]
    pub delivery: PhaseConfig,

    /// Maximum verification retry loops
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Minimum verification score to pass
    #[serde(default = "default_min_score")]
    pub min_verification_score: f64,
}

fn default_true() -> bool {
    true
}
fn default_max_retries() -> u32 {
    3
}
fn default_min_score() -> f64 {
    70.0
}

fn default_verification() -> VerificationPhaseConfig {
    VerificationPhaseConfig::default()
}

/// Verification-specific config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationPhaseConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub optional: bool,
    #[serde(default = "default_min_score")]
    pub min_score: f64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

impl Default for VerificationPhaseConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            optional: false,
            min_score: 70.0,
            max_retries: 3,
        }
    }
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            cognition: PhaseConfig {
                enabled: true,
                optional: true,
            },
            planning: PhaseConfig {
                enabled: true,
                optional: false,
            },
            execution: PhaseConfig {
                enabled: true,
                optional: false,
            },
            verification: VerificationPhaseConfig::default(),
            delivery: PhaseConfig {
                enabled: true,
                optional: false,
            },
            max_retries: 3,
            min_verification_score: 70.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let c = PipelineConfig::default();
        assert!(c.cognition.enabled);
        assert!(c.cognition.optional); // cognition is optional
        assert!(c.planning.enabled);
        assert!(!c.planning.optional); // planning is required
        assert!(c.execution.enabled);
        assert!(c.verification.enabled);
        assert!(c.delivery.enabled);
        assert_eq!(c.max_retries, 3);
        assert_eq!(c.min_verification_score, 70.0);
    }

    #[test]
    fn test_config_serialization() {
        let c = PipelineConfig::default();
        let json = serde_json::to_string(&c).unwrap();
        let back: PipelineConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.cognition.enabled, c.cognition.enabled);
        assert_eq!(back.max_retries, c.max_retries);
    }

    #[test]
    fn test_phase_config_default() {
        let p = PhaseConfig::default();
        assert!(p.enabled);
        assert!(!p.optional);
    }
}
