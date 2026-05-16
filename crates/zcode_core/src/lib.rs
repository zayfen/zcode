//! Shared core types for zcode.
//!
//! This crate owns cross-layer DTOs, configuration, and error types so higher
//! layers can depend on a stable foundation without creating dependency cycles.

pub mod config;
pub mod error;

pub mod agent;
pub mod ask;
pub mod llm;

pub use ask::{AskRequest, AskUserSender};

pub use config::{
    GrammarConfig, HookConfig, LlmConfigOverride, LspServerConfig, McpServerConfig,
    ProjectConfig, ScriptConfig, Settings, SnapshotConfig, ToolConfigs,
};
pub use error::{Result, ZcodeError};
pub use llm::{LlmConfig, LlmResponse, Message, MessageRole, UsageStats};
