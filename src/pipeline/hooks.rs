//! Pipeline lifecycle hooks

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Hook execution result
#[derive(Debug, Clone, PartialEq)]
pub enum HookAction {
    /// Continue pipeline execution
    Continue,
    /// Abort the pipeline
    Abort(String),
    /// Skip the next phase
    SkipNext,
}

/// A registered hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Event name (e.g. "after_planning")
    pub event: String,
    /// Shell command to run
    pub command: String,
    /// Whether failure aborts the pipeline
    #[serde(default)]
    pub required: bool,
    /// Timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    60
}

/// Pipeline hooks manager
#[derive(Debug, Clone, Default)]
pub struct PipelineHooks {
    /// Registered hook configs
    hooks: Vec<HookConfig>,
}

impl PipelineHooks {
    /// Create empty hooks
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a hook
    pub fn register(&mut self, config: HookConfig) {
        self.hooks.push(config);
    }

    /// Get hooks for a specific event
    pub fn hooks_for_event(&self, event: &str) -> Vec<&HookConfig> {
        self.hooks
            .iter()
            .filter(|h| h.event == event)
            .collect()
    }

    /// Execute all hooks for an event
    pub async fn run_hooks(&self, event: &str, env: &HashMap<String, String>) -> HookAction {
        let hooks = self.hooks_for_event(event);
        if hooks.is_empty() {
            return HookAction::Continue;
        }

        for hook in hooks {
            match self.execute_hook(hook, env).await {
                HookAction::Continue => continue,
                HookAction::Abort(reason) => return HookAction::Abort(reason),
                HookAction::SkipNext => return HookAction::SkipNext,
            }
        }

        HookAction::Continue
    }

    /// Execute a single hook
    async fn execute_hook(&self, hook: &HookConfig, env: &HashMap<String, String>) -> HookAction {
        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c").arg(&hook.command);

        for (k, v) in env {
            cmd.env(k, v);
        }

        match tokio::time::timeout(
            std::time::Duration::from_secs(hook.timeout_secs),
            cmd.output(),
        )
        .await
        {
            Ok(Ok(output)) if output.status.success() => HookAction::Continue,
            Ok(Ok(output)) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if hook.required {
                    HookAction::Abort(format!(
                        "Required hook '{}' failed: {}",
                        hook.event, stderr
                    ))
                } else {
                    tracing::warn!("Optional hook '{}' failed: {}", hook.event, stderr);
                    HookAction::Continue
                }
            }
            Ok(Err(e)) => {
                if hook.required {
                    HookAction::Abort(format!("Hook execution error: {}", e))
                } else {
                    tracing::warn!("Optional hook error: {}", e);
                    HookAction::Continue
                }
            }
            Err(_) => {
                if hook.required {
                    HookAction::Abort(format!("Hook '{}' timed out", hook.event))
                } else {
                    tracing::warn!("Optional hook '{}' timed out", hook.event);
                    HookAction::Continue
                }
            }
        }
    }

    /// Standard hook event names
    pub fn standard_events() -> Vec<&'static str> {
        vec![
            "before_pipeline",
            "after_pipeline",
            "before_cognition",
            "after_cognition",
            "before_planning",
            "after_planning",
            "before_execution",
            "after_execution",
            "before_verification",
            "after_verification",
            "before_delivery",
            "after_delivery",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hooks_new() {
        let h = PipelineHooks::new();
        assert!(h.hooks_for_event("before_pipeline").is_empty());
    }

    #[test]
    fn test_register_hook() {
        let mut h = PipelineHooks::new();
        h.register(HookConfig {
            event: "after_planning".into(),
            command: "echo done".into(),
            required: false,
            timeout_secs: 30,
        });
        assert_eq!(h.hooks_for_event("after_planning").len(), 1);
        assert!(h.hooks_for_event("before_planning").is_empty());
    }

    #[test]
    fn test_hook_config_default_timeout() {
        let hc = HookConfig {
            event: "test".into(),
            command: "echo".into(),
            required: false,
            timeout_secs: default_timeout(),
        };
        assert_eq!(hc.timeout_secs, 60);
    }

    #[test]
    fn test_standard_events() {
        let events = PipelineHooks::standard_events();
        assert!(events.contains(&"before_pipeline"));
        assert!(events.contains(&"after_delivery"));
        assert_eq!(events.len(), 12);
    }

    #[tokio::test]
    async fn test_run_hooks_no_hooks() {
        let h = PipelineHooks::new();
        let env = HashMap::new();
        let result = h.run_hooks("before_pipeline", &env).await;
        assert_eq!(result, HookAction::Continue);
    }

    #[test]
    fn test_hook_config_serialization() {
        let hc = HookConfig {
            event: "test".into(),
            command: "echo hi".into(),
            required: true,
            timeout_secs: 120,
        };
        let json = serde_json::to_string(&hc).unwrap();
        let back: HookConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.event, "test");
        assert!(back.required);
    }
}
