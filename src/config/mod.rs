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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

    #[test]
    fn test_project_config_creation() {
        let config = ProjectConfig::new("test-project".to_string());
        assert_eq!(config.name, "test-project");
        assert!(config.description.is_none());
        assert!(config.languages.is_empty());
    }

    #[test]
    fn test_project_config_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        let mut config = ProjectConfig::new("test-project".to_string());
        config.description = Some("A test project".to_string());
        config.languages = vec!["rust".to_string()];

        config.save(&project_dir).unwrap();
        let loaded = ProjectConfig::load(&project_dir).unwrap();

        assert_eq!(loaded.name, "test-project");
        assert_eq!(loaded.description, Some("A test project".to_string()));
        assert_eq!(loaded.languages, vec!["rust"]);
    }

    #[test]
    fn test_project_config_default() {
        let config = ProjectConfig::default();
        assert_eq!(config.name, "my-project");
        assert!(config.description.is_none());
        assert!(config.languages.is_empty());
        assert!(config.frameworks.is_empty());
    }

    #[test]
    fn test_tool_configs_default() {
        let tools = ToolConfigs::default();
        assert!(tools.enabled.is_empty());
        assert!(tools.disabled.is_empty());
    }

    #[test]
    fn test_llm_config_override() {
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
    fn test_project_config_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        let result = ProjectConfig::load(&project_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_project_config_serialization() {
        let mut config = ProjectConfig::new("test".to_string());
        config.languages = vec!["rust".to_string(), "python".to_string()];
        config.frameworks = vec!["tokio".to_string()];

        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: ProjectConfig = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.name, config.name);
        assert_eq!(deserialized.languages, config.languages);
        assert_eq!(deserialized.frameworks, config.frameworks);
    }
}
