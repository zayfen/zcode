//! User-level settings for zcode
//!
//! This module defines the Settings struct for user-level configuration
//! stored in the user's config directory.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use directories::ProjectDirs;
use std::path::PathBuf;
use crate::error::{ZcodeError, Result};

/// User-level settings for zcode
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Settings {
    /// LLM provider configuration
    #[serde(default)]
    pub llm: LlmSettings,

    /// Editor settings
    #[serde(default)]
    pub editor: EditorSettings,

    /// UI settings
    #[serde(default)]
    pub ui: UiSettings,

    /// Tool settings
    #[serde(default)]
    pub tools: ToolSettings,
}

/// LLM provider settings
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmSettings {
    /// Default LLM provider (anthropic, openai, etc.)
    #[serde(default = "default_provider")]
    pub provider: String,

    /// Default model to use
    #[serde(default = "default_model")]
    pub model: String,

    /// API key (can also be set via environment variable)
    #[serde(default)]
    pub api_key: Option<String>,

    /// Temperature for responses (0.0-2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Maximum tokens in response
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_provider() -> String {
    "anthropic".to_string()
}

fn default_model() -> String {
    "claude-3-5-sonnet-20241022".to_string()
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_timeout() -> u64 {
    120
}

impl Default for LlmSettings {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            api_key: None,
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            timeout: default_timeout(),
        }
    }
}

/// Editor settings
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditorSettings {
    /// Default editor command
    #[serde(default = "default_editor")]
    pub command: String,

    /// Auto-save files before operations
    #[serde(default = "default_true")]
    pub auto_save: bool,
}

fn default_editor() -> String {
    std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string())
}

fn default_true() -> bool {
    true
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            command: default_editor(),
            auto_save: default_true(),
        }
    }
}

/// UI settings
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UiSettings {
    /// Enable colored output
    #[serde(default = "default_true")]
    pub color: bool,

    /// Show verbose output
    #[serde(default)]
    pub verbose: bool,

    /// Progress indicator style
    #[serde(default = "default_progress_style")]
    pub progress_style: String,
}

fn default_progress_style() -> String {
    "spinner".to_string()
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            color: default_true(),
            verbose: false,
            progress_style: default_progress_style(),
        }
    }
}

/// Tool settings
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolSettings {
    /// Enable dangerous operations (file deletion, etc.)
    #[serde(default)]
    pub enable_dangerous_ops: bool,

    /// Require confirmation for operations
    #[serde(default = "default_true")]
    pub require_confirmation: bool,

    /// Timeout for tool execution in seconds
    #[serde(default = "default_tool_timeout")]
    pub timeout: u64,
}

fn default_tool_timeout() -> u64 {
    60
}

impl Default for ToolSettings {
    fn default() -> Self {
        Self {
            enable_dangerous_ops: false,
            require_confirmation: default_true(),
            timeout: default_tool_timeout(),
        }
    }
}

impl Settings {
    /// Create new settings with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the config directory path
    pub fn config_dir() -> Result<PathBuf> {
        let project_dirs = ProjectDirs::from("com", "zcode", "zcode")
            .ok_or_else(|| ZcodeError::ConfigError("Could not determine config directory".to_string()))?;

        Ok(project_dirs.config_dir().to_path_buf())
    }

    /// Get the settings file path
    pub fn settings_file() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("settings.toml"))
    }

    /// Load settings from file
    pub fn load() -> Result<Self> {
        let settings_path = Self::settings_file()?;

        if !settings_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&settings_path)
            .map_err(|_e| ZcodeError::ConfigLoadError {
                path: settings_path.display().to_string(),
            })?;

        let settings: Settings = toml::from_str(&content)?;

        Ok(settings)
    }

    /// Save settings to file
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        std::fs::create_dir_all(&config_dir)?;

        let settings_path = Self::settings_file()?;
        let content = toml::to_string_pretty(self)
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        std::fs::write(&settings_path, content)?;

        Ok(())
    }

    /// Merge with another settings object (other takes precedence)
    pub fn merge(&mut self, other: Settings) {
        // LLM settings
        if other.llm.provider != default_provider() {
            self.llm.provider = other.llm.provider;
        }
        if other.llm.model != default_model() {
            self.llm.model = other.llm.model;
        }
        if other.llm.api_key.is_some() {
            self.llm.api_key = other.llm.api_key;
        }
        if other.llm.temperature != default_temperature() {
            self.llm.temperature = other.llm.temperature;
        }
        if other.llm.max_tokens != default_max_tokens() {
            self.llm.max_tokens = other.llm.max_tokens;
        }
        if other.llm.timeout != default_timeout() {
            self.llm.timeout = other.llm.timeout;
        }

        // Editor settings
        if other.editor.command != default_editor() {
            self.editor.command = other.editor.command;
        }
        if other.editor.auto_save != default_true() {
            self.editor.auto_save = other.editor.auto_save;
        }

        // UI settings
        if other.ui.color != default_true() {
            self.ui.color = other.ui.color;
        }
        if other.ui.verbose {
            self.ui.verbose = other.ui.verbose;
        }
        if other.ui.progress_style != default_progress_style() {
            self.ui.progress_style = other.ui.progress_style;
        }

        // Tool settings
        if other.tools.enable_dangerous_ops {
            self.tools.enable_dangerous_ops = other.tools.enable_dangerous_ops;
        }
        if other.tools.require_confirmation != default_true() {
            self.tools.require_confirmation = other.tools.require_confirmation;
        }
        if other.tools.timeout != default_tool_timeout() {
            self.tools.timeout = other.tools.timeout;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert_eq!(settings.llm.provider, "anthropic");
        assert_eq!(settings.llm.temperature, 0.7);
        assert!(settings.editor.auto_save);
        assert!(settings.ui.color);
    }

    #[test]
    fn test_settings_creation() {
        let settings = Settings::new();
        assert_eq!(settings.llm.max_tokens, 4096);
        assert_eq!(settings.tools.timeout, 60);
    }

    #[test]
    fn test_settings_merge() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.llm.temperature = 1.0;
        override_settings.ui.verbose = true;

        base.merge(override_settings);

        assert_eq!(base.llm.temperature, 1.0);
        assert!(base.ui.verbose);
    }

    #[test]
    fn test_llm_settings_defaults() {
        let llm = LlmSettings::default();
        assert_eq!(llm.provider, "anthropic");
        assert_eq!(llm.model, "claude-3-5-sonnet-20241022");
        assert_eq!(llm.temperature, 0.7);
        assert_eq!(llm.max_tokens, 4096);
        assert_eq!(llm.timeout, 120);
        assert!(llm.api_key.is_none());
    }

    #[test]
    fn test_editor_settings_defaults() {
        let editor = EditorSettings::default();
        // editor.command depends on $EDITOR env var
        assert!(editor.auto_save);
    }

    #[test]
    fn test_ui_settings_defaults() {
        let ui = UiSettings::default();
        assert!(ui.color);
        assert!(!ui.verbose);
        assert_eq!(ui.progress_style, "spinner");
    }

    #[test]
    fn test_tool_settings_defaults() {
        let tools = ToolSettings::default();
        assert!(!tools.enable_dangerous_ops);
        assert!(tools.require_confirmation);
        assert_eq!(tools.timeout, 60);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = Settings::default();
        let serialized = toml::to_string_pretty(&settings);
        assert!(serialized.is_ok());

        let deserialized: Settings = toml::from_str(&serialized.unwrap()).unwrap();
        assert_eq!(deserialized.llm.provider, settings.llm.provider);
        assert_eq!(deserialized.llm.model, settings.llm.model);
    }
}
