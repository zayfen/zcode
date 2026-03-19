//! Configuration module for zcode
//!
//! This module manages project and user-level configuration settings.

mod settings;

pub use settings::Settings;

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::path::Path;
use crate::error::{ZcodeError, Result};

/// Project-level configuration stored in .zcode/config.toml
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectConfig {
    /// Project name
    pub name: String,

    /// Project description
    #[serde(default)]
    pub description: Option<String>,

    /// Programming languages used in the project
    #[serde(default)]
    pub languages: Vec<String>,

    /// Frameworks and tools
    #[serde(default)]
    pub frameworks: Vec<String>,

    /// Project-specific tool configurations
    #[serde(default)]
    pub tools: ToolConfigs,

    /// LLM provider configuration overrides
    #[serde(default)]
    pub llm: Option<LlmConfigOverride>,
}

/// Tool-specific configurations
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct ToolConfigs {
    /// Enable/disable specific tools
    #[serde(default)]
    pub enabled: Vec<String>,

    /// Disabled tools
    #[serde(default)]
    pub disabled: Vec<String>,
}

/// LLM configuration overrides for the project
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct LlmConfigOverride {
    /// Preferred LLM provider
    pub provider: Option<String>,

    /// Model to use
    pub model: Option<String>,

    /// Temperature setting (0.0-2.0)
    #[serde(default)]
    pub temperature: Option<f32>,

    /// Maximum tokens
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

impl ProjectConfig {
    /// Create a new project config
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            languages: Vec::new(),
            frameworks: Vec::new(),
            tools: ToolConfigs::default(),
            llm: None,
        }
    }

    /// Load project config from a directory
    pub fn load(project_dir: &Path) -> Result<Self> {
        let config_path = project_dir.join(".zcode").join("config.toml");

        if !config_path.exists() {
            return Err(ZcodeError::FileNotFound {
                path: config_path.display().to_string(),
            });
        }

        let content = std::fs::read_to_string(&config_path).map_err(|_e| {
            ZcodeError::ConfigLoadError {
                path: config_path.display().to_string(),
            }
        })?;

        let config: ProjectConfig = toml::from_str(&content)?;

        Ok(config)
    }

    /// Save project config to a directory
    pub fn save(&self, project_dir: &Path) -> Result<()> {
        let config_dir = project_dir.join(".zcode");
        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.toml");
        let content = toml::to_string_pretty(self)
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        std::fs::write(&config_path, content)?;

        Ok(())
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self::new("my-project".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ============================================================
    // ProjectConfig creation tests
    // ============================================================

    #[test]
    fn test_project_config_new_basic() {
        let config = ProjectConfig::new("test-project".to_string());
        assert_eq!(config.name, "test-project");
        assert!(config.description.is_none());
        assert!(config.languages.is_empty());
        assert!(config.frameworks.is_empty());
        assert!(config.llm.is_none());
    }

    #[test]
    fn test_project_config_new_empty_name() {
        let config = ProjectConfig::new("".to_string());
        assert_eq!(config.name, "");
    }

    #[test]
    fn test_project_config_new_with_special_chars() {
        let config = ProjectConfig::new("my-project_123".to_string());
        assert_eq!(config.name, "my-project_123");
    }

    #[test]
    fn test_project_config_default() {
        let config = ProjectConfig::default();
        assert_eq!(config.name, "my-project");
        assert!(config.description.is_none());
        assert!(config.languages.is_empty());
        assert!(config.frameworks.is_empty());
    }

    // ============================================================
    // ProjectConfig save/load tests
    // ============================================================

    #[test]
    fn test_project_config_save_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        let config = ProjectConfig::new("test-project".to_string());
        config.save(&project_dir).unwrap();

        assert!(project_dir.join(".zcode").exists());
        assert!(project_dir.join(".zcode/config.toml").exists());
    }

    #[test]
    fn test_project_config_save_and_load_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        let mut config = ProjectConfig::new("test-project".to_string());
        config.description = Some("A test project".to_string());
        config.languages = vec!["rust".to_string()];
        config.frameworks = vec!["tokio".to_string(), "serde".to_string()];

        config.save(&project_dir).unwrap();
        let loaded = ProjectConfig::load(&project_dir).unwrap();

        assert_eq!(loaded.name, "test-project");
        assert_eq!(loaded.description, Some("A test project".to_string()));
        assert_eq!(loaded.languages, vec!["rust"]);
        assert_eq!(loaded.frameworks, vec!["tokio", "serde"]);
    }

    #[test]
    fn test_project_config_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        let result = ProjectConfig::load(&project_dir);
        assert!(result.is_err());
        match result.unwrap_err() {
            ZcodeError::FileNotFound { path } => {
                assert!(path.contains("config.toml"));
            }
            _ => panic!("Expected FileNotFound error"),
        }
    }

    #[test]
    fn test_project_config_save_with_llm_override() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        let mut config = ProjectConfig::new("ai-project".to_string());
        config.llm = Some(LlmConfigOverride {
            provider: Some("openai".to_string()),
            model: Some("gpt-4".to_string()),
            temperature: Some(0.5),
            max_tokens: Some(2048),
        });

        config.save(&project_dir).unwrap();
        let loaded = ProjectConfig::load(&project_dir).unwrap();

        assert!(loaded.llm.is_some());
        let llm = loaded.llm.unwrap();
        assert_eq!(llm.provider, Some("openai".to_string()));
        assert_eq!(llm.model, Some("gpt-4".to_string()));
        assert_eq!(llm.temperature, Some(0.5));
        assert_eq!(llm.max_tokens, Some(2048));
    }

    #[test]
    fn test_project_config_save_with_tools() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        let mut config = ProjectConfig::new("tools-project".to_string());
        config.tools = ToolConfigs {
            enabled: vec!["read_file".to_string(), "write_file".to_string()],
            disabled: vec!["delete_file".to_string()],
        };

        config.save(&project_dir).unwrap();
        let loaded = ProjectConfig::load(&project_dir).unwrap();

        assert_eq!(loaded.tools.enabled, vec!["read_file", "write_file"]);
        assert_eq!(loaded.tools.disabled, vec!["delete_file"]);
    }

    #[test]
    fn test_project_config_load_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // Create directory and invalid config file
        std::fs::create_dir_all(project_dir.join(".zcode")).unwrap();
        std::fs::write(project_dir.join(".zcode/config.toml"), "invalid = [").unwrap();

        let result = ProjectConfig::load(&project_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_project_config_load_missing_name_field() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // Create directory and config without required name field
        std::fs::create_dir_all(project_dir.join(".zcode")).unwrap();
        std::fs::write(
            project_dir.join(".zcode/config.toml"),
            "description = \"test\"",
        )
        .unwrap();

        let result = ProjectConfig::load(&project_dir);
        assert!(result.is_err());
    }

    // ============================================================
    // ToolConfigs tests
    // ============================================================

    #[test]
    fn test_tool_configs_default_empty() {
        let tools = ToolConfigs::default();
        assert!(tools.enabled.is_empty());
        assert!(tools.disabled.is_empty());
    }

    #[test]
    fn test_tool_configs_with_enabled_tools() {
        let tools = ToolConfigs {
            enabled: vec!["tool1".to_string(), "tool2".to_string()],
            disabled: vec![],
        };
        assert_eq!(tools.enabled.len(), 2);
        assert!(tools.disabled.is_empty());
    }

    #[test]
    fn test_tool_configs_with_disabled_tools() {
        let tools = ToolConfigs {
            enabled: vec![],
            disabled: vec!["dangerous_tool".to_string()],
        };
        assert!(tools.enabled.is_empty());
        assert_eq!(tools.disabled.len(), 1);
    }

    #[test]
    fn test_tool_configs_serialization() {
        let tools = ToolConfigs {
            enabled: vec!["a".to_string(), "b".to_string()],
            disabled: vec!["c".to_string()],
        };

        let serialized = toml::to_string_pretty(&tools).unwrap();
        let deserialized: ToolConfigs = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.enabled, tools.enabled);
        assert_eq!(deserialized.disabled, tools.disabled);
    }

    // ============================================================
    // LlmConfigOverride tests
    // ============================================================

    #[test]
    fn test_llm_config_override_all_fields() {
        let llm_override = LlmConfigOverride {
            provider: Some("openai".to_string()),
            model: Some("gpt-4".to_string()),
            temperature: Some(0.5),
            max_tokens: Some(2048),
        };

        assert_eq!(llm_override.provider, Some("openai".to_string()));
        assert_eq!(llm_override.model, Some("gpt-4".to_string()));
        assert_eq!(llm_override.temperature, Some(0.5));
        assert_eq!(llm_override.max_tokens, Some(2048));
    }

    #[test]
    fn test_llm_config_override_partial_fields() {
        let llm_override = LlmConfigOverride {
            provider: Some("anthropic".to_string()),
            model: None,
            temperature: Some(1.0),
            max_tokens: None,
        };

        assert_eq!(llm_override.provider, Some("anthropic".to_string()));
        assert!(llm_override.model.is_none());
        assert_eq!(llm_override.temperature, Some(1.0));
        assert!(llm_override.max_tokens.is_none());
    }

    #[test]
    fn test_llm_config_override_all_none() {
        let llm_override = LlmConfigOverride {
            provider: None,
            model: None,
            temperature: None,
            max_tokens: None,
        };

        assert!(llm_override.provider.is_none());
        assert!(llm_override.model.is_none());
        assert!(llm_override.temperature.is_none());
        assert!(llm_override.max_tokens.is_none());
    }

    #[test]
    fn test_llm_config_override_temperature_bounds() {
        // Test minimum temperature
        let llm_override = LlmConfigOverride {
            provider: None,
            model: None,
            temperature: Some(0.0),
            max_tokens: None,
        };
        assert_eq!(llm_override.temperature, Some(0.0));

        // Test maximum temperature
        let llm_override = LlmConfigOverride {
            provider: None,
            model: None,
            temperature: Some(2.0),
            max_tokens: None,
        };
        assert_eq!(llm_override.temperature, Some(2.0));
    }

    // ============================================================
    // Serialization/Deserialization tests
    // ============================================================

    #[test]
    fn test_project_config_serialization_roundtrip() {
        let mut config = ProjectConfig::new("test".to_string());
        config.languages = vec!["rust".to_string(), "python".to_string()];
        config.frameworks = vec!["tokio".to_string()];
        config.description = Some("A test project".to_string());

        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: ProjectConfig = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.name, config.name);
        assert_eq!(deserialized.languages, config.languages);
        assert_eq!(deserialized.frameworks, config.frameworks);
        assert_eq!(deserialized.description, config.description);
    }

    #[test]
    fn test_project_config_serialization_output_format() {
        let config = ProjectConfig::new("test".to_string());
        let serialized = toml::to_string_pretty(&config).unwrap();

        // Verify TOML format
        assert!(serialized.contains("name = \"test\""));
    }

    #[test]
    fn test_project_config_deserialization_from_toml() {
        let toml_str = r#"
name = "my-project"
description = "A sample project"
languages = ["rust", "javascript"]
frameworks = ["actix", "react"]

[tools]
enabled = ["read", "write"]
disabled = ["delete"]

[llm]
provider = "anthropic"
model = "claude-3"
temperature = 0.8
max_tokens = 8192
"#;

        let config: ProjectConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.name, "my-project");
        assert_eq!(config.description, Some("A sample project".to_string()));
        assert_eq!(config.languages, vec!["rust", "javascript"]);
        assert_eq!(config.frameworks, vec!["actix", "react"]);
        assert_eq!(config.tools.enabled, vec!["read", "write"]);
        assert_eq!(config.tools.disabled, vec!["delete"]);
        assert!(config.llm.is_some());
    }

    // ============================================================
    // Debug trait tests
    // ============================================================

    #[test]
    fn test_project_config_debug() {
        let config = ProjectConfig::new("debug-test".to_string());
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("debug-test"));
    }

    #[test]
    fn test_tool_configs_debug() {
        let tools = ToolConfigs::default();
        let debug_str = format!("{:?}", tools);
        assert!(debug_str.contains("ToolConfigs"));
    }

    #[test]
    fn test_llm_config_override_debug() {
        let llm = LlmConfigOverride {
            provider: Some("test".to_string()),
            model: None,
            temperature: None,
            max_tokens: None,
        };
        let debug_str = format!("{:?}", llm);
        assert!(debug_str.contains("LlmConfigOverride"));
    }

    // ============================================================
    // Clone trait tests
    // ============================================================

    #[test]
    fn test_project_config_clone() {
        let config = ProjectConfig::new("clone-test".to_string());
        let cloned = config.clone();
        assert_eq!(config.name, cloned.name);
    }

    #[test]
    fn test_tool_configs_clone() {
        let tools = ToolConfigs {
            enabled: vec!["a".to_string()],
            disabled: vec!["b".to_string()],
        };
        let cloned = tools.clone();
        assert_eq!(tools.enabled, cloned.enabled);
        assert_eq!(tools.disabled, cloned.disabled);
    }

    // ============================================================
    // Edge cases
    // ============================================================

    #[test]
    fn test_project_config_empty_languages_and_frameworks() {
        let config = ProjectConfig::new("empty".to_string());
        assert!(config.languages.is_empty());
        assert!(config.frameworks.is_empty());
    }

    #[test]
    fn test_project_config_multiple_languages() {
        let mut config = ProjectConfig::new("polyglot".to_string());
        config.languages = vec![
            "rust".to_string(),
            "python".to_string(),
            "javascript".to_string(),
            "typescript".to_string(),
            "go".to_string(),
        ];
        assert_eq!(config.languages.len(), 5);
    }

    #[test]
    fn test_project_config_special_characters_in_description() {
        let mut config = ProjectConfig::new("special".to_string());
        config.description = Some("A project with \"quotes\" and 'apostrophes'".to_string());

        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: ProjectConfig = toml::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized.description,
            Some("A project with \"quotes\" and 'apostrophes'".to_string())
        );
    }
}
