//! Zcode - A programming agent CLI tool
//!
//! This crate provides the core functionality for the zcode programming agent,
//! including tool execution, LLM integration, and configuration management.
//!
//! # Architecture
//!
//! Zcode is built as a modular monolith with the following main components:
//!
//! - **error**: Error types and result aliases
//! - **config**: Configuration management (user-level settings and project-level configs)
//! - **tools**: Tool registry and execution system
//! - **llm**: LLM provider integration with streaming support
//! - **agent**: Agent orchestration and state management
//! - **tui**: Terminal user interface with chat capabilities
//!
//! # Example
//!
//! ```rust,no_run
//! use zcode::{Settings, ToolRegistry};
//!
//! // Load user settings
//! let settings = Settings::load().unwrap_or_default();
//!
//! // Initialize tool registry
//! let registry = ToolRegistry::new();
//! ```

pub mod error;
pub mod config;
pub mod tools;
pub mod llm;
pub mod agent;
pub mod tui;

// Re-exports for convenience
pub use error::{ZcodeError, Result};
pub use config::{Settings, ProjectConfig};
pub use tools::{ToolRegistry, Tool, ToolResult};
pub use llm::{LlmProvider, LlmConfig, Message, LlmResponse};
pub use tui::{TuiApp, ChatInterface};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_load_or_default() {
        let settings = Settings::load().unwrap_or_default();
        assert_eq!(settings.llm.provider, "anthropic");
    }

    #[test]
    fn test_tool_registry_creation() {
        let registry = ToolRegistry::new();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_error_creation() {
        let error = ZcodeError::ToolNotFound {
            name: "test".to_string(),
        };
        assert!(error.to_string().contains("test"));
    }
}
