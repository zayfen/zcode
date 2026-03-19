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
    use tempfile::TempDir;

    // ============================================================
    // Settings creation tests
    // ============================================================

    #[test]
    fn test_settings_new() {
        let settings = Settings::new();
        assert_eq!(settings.llm.provider, "anthropic");
        assert_eq!(settings.llm.temperature, 0.7);
        assert!(settings.editor.auto_save);
        assert!(settings.ui.color);
    }

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert_eq!(settings.llm.provider, "anthropic");
        assert_eq!(settings.llm.model, "claude-3-5-sonnet-20241022");
        assert_eq!(settings.llm.temperature, 0.7);
        assert_eq!(settings.llm.max_tokens, 4096);
        assert_eq!(settings.llm.timeout, 120);
        assert!(settings.llm.api_key.is_none());
        assert!(settings.editor.auto_save);
        assert!(settings.ui.color);
        assert!(!settings.ui.verbose);
        assert_eq!(settings.ui.progress_style, "spinner");
        assert!(!settings.tools.enable_dangerous_ops);
        assert!(settings.tools.require_confirmation);
        assert_eq!(settings.tools.timeout, 60);
    }

    // ============================================================
    // LlmSettings tests
    // ============================================================

    #[test]
    fn test_llm_settings_default_values() {
        let llm = LlmSettings::default();
        assert_eq!(llm.provider, "anthropic");
        assert_eq!(llm.model, "claude-3-5-sonnet-20241022");
        assert_eq!(llm.temperature, 0.7);
        assert_eq!(llm.max_tokens, 4096);
        assert_eq!(llm.timeout, 120);
        assert!(llm.api_key.is_none());
    }

    #[test]
    fn test_llm_settings_with_api_key() {
        let llm = LlmSettings {
            api_key: Some("sk-test-key".to_string()),
            ..Default::default()
        };
        assert_eq!(llm.api_key, Some("sk-test-key".to_string()));
    }

    #[test]
    fn test_llm_settings_custom_provider() {
        let llm = LlmSettings {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            ..Default::default()
        };
        assert_eq!(llm.provider, "openai");
        assert_eq!(llm.model, "gpt-4");
    }

    #[test]
    fn test_llm_settings_temperature_extremes() {
        let llm_min = LlmSettings {
            temperature: 0.0,
            ..Default::default()
        };
        assert_eq!(llm_min.temperature, 0.0);

        let llm_max = LlmSettings {
            temperature: 2.0,
            ..Default::default()
        };
        assert_eq!(llm_max.temperature, 2.0);
    }

    #[test]
    fn test_llm_settings_high_max_tokens() {
        let llm = LlmSettings {
            max_tokens: 128000,
            ..Default::default()
        };
        assert_eq!(llm.max_tokens, 128000);
    }

    #[test]
    fn test_llm_settings_custom_timeout() {
        let llm = LlmSettings {
            timeout: 300,
            ..Default::default()
        };
        assert_eq!(llm.timeout, 300);
    }

    #[test]
    fn test_llm_settings_serialization() {
        let llm = LlmSettings {
            provider: "openai".to_string(),
            model: "gpt-4-turbo".to_string(),
            api_key: Some("sk-key".to_string()),
            temperature: 0.5,
            max_tokens: 8192,
            timeout: 60,
        };

        let serialized = toml::to_string_pretty(&llm).unwrap();
        let deserialized: LlmSettings = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.provider, "openai");
        assert_eq!(deserialized.model, "gpt-4-turbo");
        assert_eq!(deserialized.api_key, Some("sk-key".to_string()));
        assert_eq!(deserialized.temperature, 0.5);
        assert_eq!(deserialized.max_tokens, 8192);
        assert_eq!(deserialized.timeout, 60);
    }

    // ============================================================
    // EditorSettings tests
    // ============================================================

    #[test]
    fn test_editor_settings_default() {
        let editor = EditorSettings::default();
        assert!(editor.auto_save);
        // editor.command depends on $EDITOR env var
    }

    #[test]
    fn test_editor_settings_custom_command() {
        let editor = EditorSettings {
            command: "code".to_string(),
            auto_save: false,
        };
        assert_eq!(editor.command, "code");
        assert!(!editor.auto_save);
    }

    #[test]
    fn test_editor_settings_no_auto_save() {
        let editor = EditorSettings {
            auto_save: false,
            ..Default::default()
        };
        assert!(!editor.auto_save);
    }

    #[test]
    fn test_editor_settings_serialization() {
        let editor = EditorSettings {
            command: "nano".to_string(),
            auto_save: true,
        };

        let serialized = toml::to_string_pretty(&editor).unwrap();
        let deserialized: EditorSettings = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.command, "nano");
        assert!(deserialized.auto_save);
    }

    // ============================================================
    // UiSettings tests
    // ============================================================

    #[test]
    fn test_ui_settings_default() {
        let ui = UiSettings::default();
        assert!(ui.color);
        assert!(!ui.verbose);
        assert_eq!(ui.progress_style, "spinner");
    }

    #[test]
    fn test_ui_settings_no_color() {
        let ui = UiSettings {
            color: false,
            ..Default::default()
        };
        assert!(!ui.color);
    }

    #[test]
    fn test_ui_settings_verbose() {
        let ui = UiSettings {
            verbose: true,
            ..Default::default()
        };
        assert!(ui.verbose);
    }

    #[test]
    fn test_ui_settings_progress_styles() {
        let styles = ["spinner", "bar", "dots", "none"];
        for style in styles {
            let ui = UiSettings {
                progress_style: style.to_string(),
                ..Default::default()
            };
            assert_eq!(ui.progress_style, style);
        }
    }

    #[test]
    fn test_ui_settings_serialization() {
        let ui = UiSettings {
            color: false,
            verbose: true,
            progress_style: "bar".to_string(),
        };

        let serialized = toml::to_string_pretty(&ui).unwrap();
        let deserialized: UiSettings = toml::from_str(&serialized).unwrap();

        assert!(!deserialized.color);
        assert!(deserialized.verbose);
        assert_eq!(deserialized.progress_style, "bar");
    }

    // ============================================================
    // ToolSettings tests
    // ============================================================

    #[test]
    fn test_tool_settings_default() {
        let tools = ToolSettings::default();
        assert!(!tools.enable_dangerous_ops);
        assert!(tools.require_confirmation);
        assert_eq!(tools.timeout, 60);
    }

    #[test]
    fn test_tool_settings_enable_dangerous() {
        let tools = ToolSettings {
            enable_dangerous_ops: true,
            ..Default::default()
        };
        assert!(tools.enable_dangerous_ops);
    }

    #[test]
    fn test_tool_settings_no_confirmation() {
        let tools = ToolSettings {
            require_confirmation: false,
            ..Default::default()
        };
        assert!(!tools.require_confirmation);
    }

    #[test]
    fn test_tool_settings_custom_timeout() {
        let tools = ToolSettings {
            timeout: 120,
            ..Default::default()
        };
        assert_eq!(tools.timeout, 120);
    }

    #[test]
    fn test_tool_settings_serialization() {
        let tools = ToolSettings {
            enable_dangerous_ops: true,
            require_confirmation: false,
            timeout: 180,
        };

        let serialized = toml::to_string_pretty(&tools).unwrap();
        let deserialized: ToolSettings = toml::from_str(&serialized).unwrap();

        assert!(deserialized.enable_dangerous_ops);
        assert!(!deserialized.require_confirmation);
        assert_eq!(deserialized.timeout, 180);
    }

    // ============================================================
    // Settings merge tests
    // ============================================================

    #[test]
    fn test_settings_merge_llm_temperature() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.llm.temperature = 1.0;

        base.merge(override_settings);

        assert_eq!(base.llm.temperature, 1.0);
    }

    #[test]
    fn test_settings_merge_llm_provider() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.llm.provider = "openai".to_string();

        base.merge(override_settings);

        assert_eq!(base.llm.provider, "openai");
    }

    #[test]
    fn test_settings_merge_llm_model() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.llm.model = "gpt-4".to_string();

        base.merge(override_settings);

        assert_eq!(base.llm.model, "gpt-4");
    }

    #[test]
    fn test_settings_merge_llm_api_key() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.llm.api_key = Some("sk-test".to_string());

        base.merge(override_settings);

        assert_eq!(base.llm.api_key, Some("sk-test".to_string()));
    }

    #[test]
    fn test_settings_merge_llm_max_tokens() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.llm.max_tokens = 8192;

        base.merge(override_settings);

        assert_eq!(base.llm.max_tokens, 8192);
    }

    #[test]
    fn test_settings_merge_llm_timeout() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.llm.timeout = 300;

        base.merge(override_settings);

        assert_eq!(base.llm.timeout, 300);
    }

    #[test]
    fn test_settings_merge_editor_command() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.editor.command = "code".to_string();

        base.merge(override_settings);

        assert_eq!(base.editor.command, "code");
    }

    #[test]
    fn test_settings_merge_editor_auto_save() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.editor.auto_save = false;

        base.merge(override_settings);

        assert!(!base.editor.auto_save);
    }

    #[test]
    fn test_settings_merge_ui_color() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.ui.color = false;

        base.merge(override_settings);

        assert!(!base.ui.color);
    }

    #[test]
    fn test_settings_merge_ui_verbose() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.ui.verbose = true;

        base.merge(override_settings);

        assert!(base.ui.verbose);
    }

    #[test]
    fn test_settings_merge_ui_progress_style() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.ui.progress_style = "bar".to_string();

        base.merge(override_settings);

        assert_eq!(base.ui.progress_style, "bar");
    }

    #[test]
    fn test_settings_merge_tools_dangerous_ops() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.tools.enable_dangerous_ops = true;

        base.merge(override_settings);

        assert!(base.tools.enable_dangerous_ops);
    }

    #[test]
    fn test_settings_merge_tools_require_confirmation() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.tools.require_confirmation = false;

        base.merge(override_settings);

        assert!(!base.tools.require_confirmation);
    }

    #[test]
    fn test_settings_merge_tools_timeout() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.tools.timeout = 120;

        base.merge(override_settings);

        assert_eq!(base.tools.timeout, 120);
    }

    #[test]
    fn test_settings_merge_no_override_when_default() {
        let mut base = Settings::default();
        let original_temperature = base.llm.temperature;

        let override_settings = Settings::default();
        base.merge(override_settings);

        // Should stay the same since override has default values
        assert_eq!(base.llm.temperature, original_temperature);
    }

    #[test]
    fn test_settings_merge_multiple_fields() {
        let mut base = Settings::default();
        let mut override_settings = Settings::default();
        override_settings.llm.temperature = 1.0;
        override_settings.ui.verbose = true;
        override_settings.tools.enable_dangerous_ops = true;

        base.merge(override_settings);

        assert_eq!(base.llm.temperature, 1.0);
        assert!(base.ui.verbose);
        assert!(base.tools.enable_dangerous_ops);
    }

    // ============================================================
    // Settings serialization tests
    // ============================================================

    #[test]
    fn test_settings_serialization_roundtrip() {
        let settings = Settings::default();
        let serialized = toml::to_string_pretty(&settings).unwrap();
        let deserialized: Settings = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.llm.provider, settings.llm.provider);
        assert_eq!(deserialized.llm.model, settings.llm.model);
        assert_eq!(deserialized.llm.temperature, settings.llm.temperature);
    }

    #[test]
    fn test_settings_serialization_output_format() {
        let settings = Settings::default();
        let serialized = toml::to_string_pretty(&settings).unwrap();

        assert!(serialized.contains("[llm]"));
        assert!(serialized.contains("[editor]"));
        assert!(serialized.contains("[ui]"));
        assert!(serialized.contains("[tools]"));
    }

    #[test]
    fn test_settings_deserialization_from_toml() {
        let toml_str = r#"
[llm]
provider = "openai"
model = "gpt-4"
api_key = "sk-test"
temperature = 0.5
max_tokens = 8192
timeout = 60

[editor]
command = "vim"
auto_save = false

[ui]
color = false
verbose = true
progress_style = "bar"

[tools]
enable_dangerous_ops = true
require_confirmation = false
timeout = 120
"#;

        let settings: Settings = toml::from_str(toml_str).unwrap();

        assert_eq!(settings.llm.provider, "openai");
        assert_eq!(settings.llm.model, "gpt-4");
        assert_eq!(settings.llm.api_key, Some("sk-test".to_string()));
        assert_eq!(settings.llm.temperature, 0.5);
        assert_eq!(settings.llm.max_tokens, 8192);
        assert_eq!(settings.llm.timeout, 60);
        assert_eq!(settings.editor.command, "vim");
        assert!(!settings.editor.auto_save);
        assert!(!settings.ui.color);
        assert!(settings.ui.verbose);
        assert_eq!(settings.ui.progress_style, "bar");
        assert!(settings.tools.enable_dangerous_ops);
        assert!(!settings.tools.require_confirmation);
        assert_eq!(settings.tools.timeout, 120);
    }

    #[test]
    fn test_settings_deserialization_partial() {
        let toml_str = r#"
[llm]
provider = "anthropic"
"#;

        let settings: Settings = toml::from_str(toml_str).unwrap();

        // Should use defaults for missing fields
        assert_eq!(settings.llm.provider, "anthropic");
        assert_eq!(settings.llm.model, "claude-3-5-sonnet-20241022");
        assert_eq!(settings.llm.temperature, 0.7);
    }

    // ============================================================
    // Settings save/load tests (with file system)
    // ============================================================

    #[test]
    fn test_settings_save_and_load() {
        let temp_dir = TempDir::new().unwrap();

        // Override config dir for testing
        let config_dir = temp_dir.path().join("zcode");
        std::fs::create_dir_all(&config_dir).unwrap();

        let settings_path = config_dir.join("settings.toml");

        let mut settings = Settings::default();
        settings.llm.temperature = 0.9;
        settings.ui.verbose = true;

        // Manually save
        let content = toml::to_string_pretty(&settings).unwrap();
        std::fs::write(&settings_path, content).unwrap();

        // Manually load
        let loaded_content = std::fs::read_to_string(&settings_path).unwrap();
        let loaded: Settings = toml::from_str(&loaded_content).unwrap();

        assert_eq!(loaded.llm.temperature, 0.9);
        assert!(loaded.ui.verbose);
    }

    // ============================================================
    // Debug trait tests
    // ============================================================

    #[test]
    fn test_settings_debug() {
        let settings = Settings::default();
        let debug_str = format!("{:?}", settings);
        assert!(debug_str.contains("Settings"));
        assert!(debug_str.contains("LlmSettings"));
    }

    #[test]
    fn test_llm_settings_debug() {
        let llm = LlmSettings::default();
        let debug_str = format!("{:?}", llm);
        assert!(debug_str.contains("LlmSettings"));
        assert!(debug_str.contains("anthropic"));
    }

    #[test]
    fn test_editor_settings_debug() {
        let editor = EditorSettings::default();
        let debug_str = format!("{:?}", editor);
        assert!(debug_str.contains("EditorSettings"));
    }

    #[test]
    fn test_ui_settings_debug() {
        let ui = UiSettings::default();
        let debug_str = format!("{:?}", ui);
        assert!(debug_str.contains("UiSettings"));
    }

    #[test]
    fn test_tool_settings_debug() {
        let tools = ToolSettings::default();
        let debug_str = format!("{:?}", tools);
        assert!(debug_str.contains("ToolSettings"));
    }

    // ============================================================
    // Clone trait tests
    // ============================================================

    #[test]
    fn test_settings_clone() {
        let settings = Settings::default();
        let cloned = settings.clone();
        assert_eq!(settings.llm.provider, cloned.llm.provider);
    }

    #[test]
    fn test_llm_settings_clone() {
        let llm = LlmSettings {
            provider: "test".to_string(),
            ..Default::default()
        };
        let cloned = llm.clone();
        assert_eq!(llm.provider, cloned.provider);
    }

    // ============================================================
    // JsonSchema tests (if applicable)
    // ============================================================

    #[test]
    fn test_settings_json_schema() {
        use schemars::schema_for;
        let schema = schema_for!(Settings);
        assert!(schema.schema.object.is_some());
    }

    #[test]
    fn test_llm_settings_json_schema() {
        use schemars::schema_for;
        let schema = schema_for!(LlmSettings);
        assert!(schema.schema.object.is_some());
    }

    // ============================================================
    // Default function tests
    // ============================================================

    #[test]
    fn test_default_provider_function() {
        assert_eq!(default_provider(), "anthropic");
    }

    #[test]
    fn test_default_model_function() {
        assert_eq!(default_model(), "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_default_temperature_function() {
        assert_eq!(default_temperature(), 0.7);
    }

    #[test]
    fn test_default_max_tokens_function() {
        assert_eq!(default_max_tokens(), 4096);
    }

    #[test]
    fn test_default_timeout_function() {
        assert_eq!(default_timeout(), 120);
    }

    #[test]
    fn test_default_true_function() {
        assert!(default_true());
    }

    #[test]
    fn test_default_progress_style_function() {
        assert_eq!(default_progress_style(), "spinner");
    }

    #[test]
    fn test_default_tool_timeout_function() {
        assert_eq!(default_tool_timeout(), 60);
    }
}
