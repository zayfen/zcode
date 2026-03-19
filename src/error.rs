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

    #[test]
    fn test_tool_not_found_error() {
        let error = ZcodeError::ToolNotFound {
            name: "test_tool".to_string(),
        };
        assert_eq!(error.to_string(), "Tool 'test_tool' not found");
    }

    #[test]
    fn test_tool_execution_failed_error() {
        let error = ZcodeError::ToolExecutionFailed {
            name: "test".to_string(),
            message: "failed".to_string(),
        };
        assert_eq!(error.to_string(), "Tool 'test' execution failed: failed");
    }

    #[test]
    fn test_config_error() {
        let error = ZcodeError::ConfigError("invalid config".to_string());
        assert_eq!(error.to_string(), "Configuration error: invalid config");
    }

    #[test]
    fn test_llm_api_error() {
        let error = ZcodeError::LlmApiError("API error".to_string());
        assert_eq!(error.to_string(), "LLM API error: API error");
    }

    #[test]
    fn test_missing_api_key() {
        let error = ZcodeError::MissingApiKey("anthropic".to_string());
        assert_eq!(error.to_string(), "API key not found for provider: anthropic");
    }

    #[test]
    fn test_file_not_found() {
        let error = ZcodeError::FileNotFound {
            path: "/test/path".to_string(),
        };
        assert_eq!(error.to_string(), "File not found: /test/path");
    }

    #[test]
    fn test_cancelled() {
        let error = ZcodeError::Cancelled;
        assert_eq!(error.to_string(), "Operation cancelled by user");
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.ok(), Some(42));
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(ZcodeError::InternalError("error".to_string()));
        assert!(result.is_err());
    }
}
