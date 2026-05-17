//! User-level settings for zcode
//!
//! This module defines the Settings struct for user-level configuration
//! stored in the user's config directory.

use crate::error::{Result, ZcodeError};
use directories::UserDirs;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

    /// Global MCP Servers
    #[serde(default)]
    pub mcp_servers: Vec<super::McpServerConfig>,

    /// Extra directories to load skills from
    #[serde(default)]
    pub skill_dirs: Vec<String>,
}

/// LLM provider settings
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmSettings {
    /// Default LLM provider. Kept for config compatibility; requests use OpenAI-compatible HTTP.
    #[serde(default = "default_provider")]
    pub provider: String,

    /// Default model to use
    #[serde(default = "default_model")]
    pub model: String,

    /// Fast model for simple tasks. Falls back to `model` when unset.
    #[serde(default = "default_fast_model")]
    pub fast_model: Option<String>,

    /// OpenAI-compatible API base URL or full chat completions endpoint.
    #[serde(default = "default_base_url")]
    pub base_url: Option<String>,

    /// API key (can also be set via environment variable)
    #[serde(default)]
    pub api_key: Option<String>,

    /// Environment variable name used to load the API key.
    #[serde(default)]
    pub api_key_env: Option<String>,

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
    "openai-compatible".to_string()
}

fn default_model() -> String {
    std::env::var("ZCODE_MODEL").unwrap_or_else(|_| "gpt-4o".to_string())
}

fn default_fast_model() -> Option<String> {
    std::env::var("ZCODE_FAST_MODEL").ok()
}

fn default_base_url() -> Option<String> {
    std::env::var("ZCODE_BASE_URL").ok()
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
            fast_model: default_fast_model(),
            base_url: default_base_url(),
            api_key: None,
            api_key_env: None,
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
        let base_dir = UserDirs::new()
            .map(|d| d.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(base_dir.join(".zcode"))
    }

    /// Get the settings file path
    pub fn settings_file() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("zcode.json"))
    }

    /// Load settings from file
    pub fn load() -> Result<Self> {
        let settings_path = Self::settings_file()?;

        if !settings_path.exists() {
            return Ok(Self::default());
        }

        let content =
            std::fs::read_to_string(&settings_path).map_err(|_e| ZcodeError::ConfigLoadError {
                path: settings_path.display().to_string(),
            })?;

        let settings: Settings = serde_json::from_str(&content).unwrap_or_else(|_| Self::default());

        Ok(settings)
    }

    /// Save settings to file
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        std::fs::create_dir_all(&config_dir)?;

        let settings_path = Self::settings_file()?;
        let content = serde_json::to_string_pretty(self)
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
        if other.llm.fast_model != default_fast_model() {
            self.llm.fast_model = other.llm.fast_model;
        }
        if other.llm.base_url != default_base_url() {
            self.llm.base_url = other.llm.base_url;
        }
        if other.llm.api_key.is_some() {
            self.llm.api_key = other.llm.api_key;
        }
        if other.llm.api_key_env.is_some() {
            self.llm.api_key_env = other.llm.api_key_env;
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

        // Global MCP settings
        if !other.mcp_servers.is_empty() {
            self.mcp_servers.extend(other.mcp_servers);
        }

        // Global skill directories
        if !other.skill_dirs.is_empty() {
            self.skill_dirs.extend(other.skill_dirs);
        }
    }
}

#[cfg(test)]
mod settings_tests;
