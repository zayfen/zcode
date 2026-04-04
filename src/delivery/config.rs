//! Delivery configuration types — DeliveryConfig, GitPlatform, CiConfig, GateCheck

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Top-level delivery pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryConfig {
    /// Whether to automatically create a PR after pushing
    pub auto_pr: bool,

    /// Whether to automatically generate a changelog
    pub auto_changelog: bool,

    /// Whether to automatically bump the version
    pub auto_version_bump: bool,

    /// Target git platform
    pub platform: GitPlatform,

    /// CI integration configuration
    pub ci: Option<CiConfig>,

    /// Gate checks to run before delivery
    pub gate_checks: Vec<GateCheck>,

    /// Branch naming template (supports {{date}}, {{task_summary}})
    pub branch_template: String,

    /// Custom PR body template
    pub pr_template: Option<String>,

    /// Base branch for PRs (defaults to "main")
    #[serde(default = "default_base_branch")]
    pub base_branch: String,
}

fn default_base_branch() -> String {
    "main".to_string()
}

impl Default for DeliveryConfig {
    fn default() -> Self {
        Self {
            auto_pr: true,
            auto_changelog: true,
            auto_version_bump: false,
            platform: GitPlatform::GitHub,
            ci: Some(CiConfig {
                platform: CiPlatform::GitHubActions,
                block_on_ci: true,
                timeout: Duration::from_secs(600),
            }),
            gate_checks: vec![
                GateCheck {
                    name: "all_tasks_verified".into(),
                    check_type: GateCheckType::MinVerificationScore { min_score: 70.0 },
                    required: true,
                },
                GateCheck {
                    name: "clean_tree".into(),
                    check_type: GateCheckType::CleanWorkingTree,
                    required: true,
                },
            ],
            branch_template: "zcode/{{date}}-{{task_summary}}".into(),
            pr_template: None,
            base_branch: "main".into(),
        }
    }
}

/// Supported git hosting platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GitPlatform {
    GitHub,
    GitLab,
    Gitea,
    Bitbucket,
    Custom { cli_command: String },
}

/// CI integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiConfig {
    /// CI platform
    pub platform: CiPlatform,
    /// Whether to block delivery until CI passes
    pub block_on_ci: bool,
    /// Maximum time to wait for CI
    #[serde(
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub timeout: Duration,
}

fn serialize_duration<S: serde::Serializer>(dur: &Duration, s: S) -> std::result::Result<S::Ok, S::Error> {
    s.serialize_u64(dur.as_secs())
}

fn deserialize_duration<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Duration, D::Error> {
    let secs = u64::deserialize(d)?;
    Ok(Duration::from_secs(secs))
}

/// Supported CI platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CiPlatform {
    GitHubActions,
    GitLabCI,
    CircleCI,
    Custom { check_command: String },
}

/// A single gate check to run before delivery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateCheck {
    /// Human-readable name
    pub name: String,
    /// The check to perform
    pub check_type: GateCheckType,
    /// Whether a failure blocks delivery
    pub required: bool,
}

/// Types of gate checks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GateCheckType {
    /// Run a shell command and check its exit code
    Command {
        command: String,
        expected_exit_code: i32,
    },
    /// Verify a file exists at the given path
    FileExists { path: String },
    /// Ensure all task verification scores meet a minimum threshold
    MinVerificationScore { min_score: f64 },
    /// Ensure the working tree has no uncommitted changes
    CleanWorkingTree,
    /// Ensure the current branch can fast-forward merge into the target
    CanFastForward { target_branch: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DeliveryConfig::default();
        assert!(config.auto_pr);
        assert!(config.auto_changelog);
        assert!(!config.auto_version_bump);
        assert!(matches!(config.platform, GitPlatform::GitHub));
        assert_eq!(config.base_branch, "main");
        assert_eq!(config.gate_checks.len(), 2);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = DeliveryConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: DeliveryConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.auto_pr, config.auto_pr);
        assert_eq!(deserialized.auto_changelog, config.auto_changelog);
        assert_eq!(deserialized.base_branch, config.base_branch);
        assert_eq!(deserialized.gate_checks.len(), config.gate_checks.len());
    }

    #[test]
    fn test_gate_check_types() {
        let checks = vec![
            GateCheckType::Command {
                command: "cargo test".into(),
                expected_exit_code: 0,
            },
            GateCheckType::FileExists {
                path: "Cargo.toml".into(),
            },
            GateCheckType::MinVerificationScore { min_score: 80.0 },
            GateCheckType::CleanWorkingTree,
            GateCheckType::CanFastForward {
                target_branch: "main".into(),
            },
        ];
        for check in &checks {
            let json = serde_json::to_string(&check).unwrap();
            let _: GateCheckType = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_git_platform_variants() {
        let platforms = vec![
            GitPlatform::GitHub,
            GitPlatform::GitLab,
            GitPlatform::Gitea,
            GitPlatform::Bitbucket,
            GitPlatform::Custom {
                cli_command: "my-git".into(),
            },
        ];
        for platform in &platforms {
            let json = serde_json::to_string(platform).unwrap();
            let _: GitPlatform = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_ci_config_serialization() {
        let ci = CiConfig {
            platform: CiPlatform::GitHubActions,
            block_on_ci: true,
            timeout: Duration::from_secs(600),
        };
        let json = serde_json::to_string(&ci).unwrap();
        let deserialized: CiConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.timeout.as_secs(), 600);
        assert!(deserialized.block_on_ci);
    }

    #[test]
    fn test_ci_platform_custom() {
        let platform = CiPlatform::Custom {
            check_command: "./ci-check.sh".into(),
        };
        let json = serde_json::to_string(&platform).unwrap();
        let deserialized: CiPlatform = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, CiPlatform::Custom { .. }));
    }

    #[test]
    fn test_gate_check_required_flag() {
        let check = GateCheck {
            name: "test_check".into(),
            check_type: GateCheckType::CleanWorkingTree,
            required: true,
        };
        assert!(check.required);
    }

    #[test]
    fn test_config_toml_roundtrip() {
        let config = DeliveryConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: DeliveryConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.auto_pr, config.auto_pr);
        assert_eq!(deserialized.branch_template, config.branch_template);
    }
}
