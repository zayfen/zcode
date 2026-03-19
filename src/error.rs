//! Error types for zcode
//!
//! This module defines the error handling strategy using `thiserror` for
//! structured error types and `anyhow` for error propagation.

use thiserror::Error;

/// The main error type for zcode operations
#[derive(Error, Debug)]
pub enum ZcodeError {
    /// Tool-related errors
    #[error("Tool '{name}' not found")]
    ToolNotFound { name: String },

    #[error("Tool '{name}' execution failed: {message}")]
    ToolExecutionFailed { name: String, message: String },

    #[error("Invalid tool input: {0}")]
    InvalidToolInput(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Failed to load config file: {path}")]
    ConfigLoadError { path: String },

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// LLM-related errors
    #[error("LLM API error: {0}")]
    LlmApiError(String),

    #[error("LLM response parsing failed: {0}")]
    LlmResponseError(String),

    #[error("API key not found for provider: {0}")]
    MissingApiKey(String),

    /// File system errors
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization errors
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("TOML parsing error: {0}")]
    TomlError(#[from] toml::de::Error),

    /// Generic errors
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Operation cancelled by user")]
    Cancelled,
}

/// Result type alias for zcode operations
pub type Result<T> = std::result::Result<T, ZcodeError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    // ============================================================
    // Tool-related errors
    // ============================================================

    #[test]
    fn test_tool_not_found_error_display() {
        let error = ZcodeError::ToolNotFound {
            name: "test_tool".to_string(),
        };
        assert_eq!(error.to_string(), "Tool 'test_tool' not found");
    }

    #[test]
    fn test_tool_not_found_error_empty_name() {
        let error = ZcodeError::ToolNotFound {
            name: "".to_string(),
        };
        assert_eq!(error.to_string(), "Tool '' not found");
    }

    #[test]
    fn test_tool_execution_failed_error_display() {
        let error = ZcodeError::ToolExecutionFailed {
            name: "test".to_string(),
            message: "failed".to_string(),
        };
        assert_eq!(error.to_string(), "Tool 'test' execution failed: failed");
    }

    #[test]
    fn test_tool_execution_failed_error_long_message() {
        let error = ZcodeError::ToolExecutionFailed {
            name: "read_file".to_string(),
            message: "permission denied: /etc/shadow".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Tool 'read_file' execution failed: permission denied: /etc/shadow"
        );
    }

    #[test]
    fn test_invalid_tool_input_error_display() {
        let error = ZcodeError::InvalidToolInput("expected JSON object".to_string());
        assert_eq!(error.to_string(), "Invalid tool input: expected JSON object");
    }

    #[test]
    fn test_invalid_tool_input_error_empty() {
        let error = ZcodeError::InvalidToolInput("".to_string());
        assert_eq!(error.to_string(), "Invalid tool input: ");
    }

    // ============================================================
    // Configuration errors
    // ============================================================

    #[test]
    fn test_config_error_display() {
        let error = ZcodeError::ConfigError("invalid config".to_string());
        assert_eq!(error.to_string(), "Configuration error: invalid config");
    }

    #[test]
    fn test_config_load_error_display() {
        let error = ZcodeError::ConfigLoadError {
            path: "/home/user/.zcode/config.toml".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Failed to load config file: /home/user/.zcode/config.toml"
        );
    }

    #[test]
    fn test_config_load_error_relative_path() {
        let error = ZcodeError::ConfigLoadError {
            path: ".zcode/config.toml".to_string(),
        };
        assert_eq!(error.to_string(), "Failed to load config file: .zcode/config.toml");
    }

    #[test]
    fn test_invalid_config_error_display() {
        let error = ZcodeError::InvalidConfig("missing required field 'name'".to_string());
        assert_eq!(
            error.to_string(),
            "Invalid configuration: missing required field 'name'"
        );
    }

    // ============================================================
    // LLM-related errors
    // ============================================================

    #[test]
    fn test_llm_api_error_display() {
        let error = ZcodeError::LlmApiError("API error".to_string());
        assert_eq!(error.to_string(), "LLM API error: API error");
    }

    #[test]
    fn test_llm_api_error_rate_limit() {
        let error = ZcodeError::LlmApiError("rate limit exceeded".to_string());
        assert_eq!(error.to_string(), "LLM API error: rate limit exceeded");
    }

    #[test]
    fn test_llm_response_error_display() {
        let error = ZcodeError::LlmResponseError("unexpected EOF".to_string());
        assert_eq!(error.to_string(), "LLM response parsing failed: unexpected EOF");
    }

    #[test]
    fn test_llm_response_error_invalid_json() {
        let error = ZcodeError::LlmResponseError("invalid JSON at position 42".to_string());
        assert_eq!(
            error.to_string(),
            "LLM response parsing failed: invalid JSON at position 42"
        );
    }

    #[test]
    fn test_missing_api_key_display() {
        let error = ZcodeError::MissingApiKey("anthropic".to_string());
        assert_eq!(error.to_string(), "API key not found for provider: anthropic");
    }

    #[test]
    fn test_missing_api_key_openai() {
        let error = ZcodeError::MissingApiKey("openai".to_string());
        assert_eq!(error.to_string(), "API key not found for provider: openai");
    }

    // ============================================================
    // File system errors
    // ============================================================

    #[test]
    fn test_file_not_found_display() {
        let error = ZcodeError::FileNotFound {
            path: "/test/path".to_string(),
        };
        assert_eq!(error.to_string(), "File not found: /test/path");
    }

    #[test]
    fn test_file_not_found_relative_path() {
        let error = ZcodeError::FileNotFound {
            path: "src/main.rs".to_string(),
        };
        assert_eq!(error.to_string(), "File not found: src/main.rs");
    }

    #[test]
    fn test_io_error_from_std_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let error: ZcodeError = io_err.into();
        assert!(error.to_string().contains("file not found"));
    }

    #[test]
    fn test_io_error_permission_denied() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let error: ZcodeError = io_err.into();
        assert!(error.to_string().contains("permission denied"));
    }

    // ============================================================
    // Serialization errors
    // ============================================================

    #[test]
    fn test_json_error_from_serde_json() {
        let json_result: core::result::Result<serde_json::Value, _> = serde_json::from_str("invalid json");
        let json_err = json_result.unwrap_err();
        let error: ZcodeError = json_err.into();
        assert!(matches!(error, ZcodeError::JsonError(_)));
        assert!(error.to_string().contains("JSON"));
    }

    #[test]
    fn test_toml_error_from_toml() {
        let toml_result: core::result::Result<toml::Value, _> = toml::from_str("invalid = [");
        let toml_err = toml_result.unwrap_err();
        let error: ZcodeError = toml_err.into();
        assert!(matches!(error, ZcodeError::TomlError(_)));
        assert!(error.to_string().contains("TOML"));
    }

    // ============================================================
    // Generic errors
    // ============================================================

    #[test]
    fn test_internal_error_display() {
        let error = ZcodeError::InternalError("unexpected state".to_string());
        assert_eq!(error.to_string(), "Internal error: unexpected state");
    }

    #[test]
    fn test_internal_error_empty() {
        let error = ZcodeError::InternalError("".to_string());
        assert_eq!(error.to_string(), "Internal error: ");
    }

    #[test]
    fn test_cancelled_display() {
        let error = ZcodeError::Cancelled;
        assert_eq!(error.to_string(), "Operation cancelled by user");
    }

    // ============================================================
    // Result type tests
    // ============================================================

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(ZcodeError::InternalError("error".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_result_type_unwrap_err() {
        let result: Result<i32> = Err(ZcodeError::Cancelled);
        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Operation cancelled by user");
    }

    #[test]
    fn test_result_type_map() {
        let result: Result<i32> = Ok(10);
        let mapped = result.map(|x| x * 2);
        assert_eq!(mapped.unwrap(), 20);
    }

    #[test]
    fn test_result_type_and_then() {
        let result: Result<i32> = Ok(10);
        let chained = result.and_then(|x| Ok(x + 5));
        assert_eq!(chained.unwrap(), 15);
    }

    #[test]
    fn test_result_type_unwrap_or() {
        let result: Result<i32> = Err(ZcodeError::Cancelled);
        assert_eq!(result.unwrap_or(0), 0);
    }

    // ============================================================
    // Debug trait tests
    // ============================================================

    #[test]
    fn test_error_debug_format() {
        let error = ZcodeError::ToolNotFound {
            name: "test".to_string(),
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ToolNotFound"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_error_debug_format_all_variants() {
        // Test that all variants can be debug formatted
        let _ = format!("{:?}", ZcodeError::ToolNotFound { name: "t".into() });
        let _ = format!("{:?}", ZcodeError::ToolExecutionFailed { name: "t".into(), message: "m".into() });
        let _ = format!("{:?}", ZcodeError::InvalidToolInput("m".into()));
        let _ = format!("{:?}", ZcodeError::ConfigError("m".into()));
        let _ = format!("{:?}", ZcodeError::ConfigLoadError { path: "p".into() });
        let _ = format!("{:?}", ZcodeError::InvalidConfig("m".into()));
        let _ = format!("{:?}", ZcodeError::LlmApiError("m".into()));
        let _ = format!("{:?}", ZcodeError::LlmResponseError("m".into()));
        let _ = format!("{:?}", ZcodeError::MissingApiKey("p".into()));
        let _ = format!("{:?}", ZcodeError::FileNotFound { path: "p".into() });
        let _ = format!("{:?}", ZcodeError::IoError(io::Error::new(io::ErrorKind::Other, "e")));
        let _ = format!("{:?}", ZcodeError::JsonError(serde_json::from_str::<serde_json::Value>("x").unwrap_err()));
        let _ = format!("{:?}", ZcodeError::TomlError(toml::from_str::<toml::Value>("x=").unwrap_err()));
        let _ = format!("{:?}", ZcodeError::InternalError("m".into()));
        let _ = format!("{:?}", ZcodeError::Cancelled);
    }

    // ============================================================
    // Error trait tests
    // ============================================================

    #[test]
    fn test_error_trait_implementation() {
        fn assert_error<T: std::error::Error>(_: &T) {}
        let error = ZcodeError::Cancelled;
        assert_error(&error);
    }

    #[test]
    fn test_error_source_chain() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let error: ZcodeError = io_err.into();
        // IoError has a source (the underlying io::Error)
        assert!(std::error::Error::source(&error).is_some());
    }
}
