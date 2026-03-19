//! LLM integration module for zcode
//!
//! This module provides integration with various LLM providers using the rig-core library.

pub mod provider;
pub mod streaming;

pub use provider::{LlmProvider, RigProvider};
pub use streaming::{StreamHandler, StreamingResponse};

/// LLM client configuration
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Provider name (anthropic, openai, etc.)
    pub provider: String,
    /// Model name
    pub model: String,
    /// API key
    pub api_key: Option<String>,
    /// Temperature (0.0-2.0)
    pub temperature: f32,
    /// Maximum tokens
    pub max_tokens: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "anthropic".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            api_key: None,
            temperature: 0.7,
            max_tokens: 4096,
        }
    }
}

/// Message role
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    /// System message
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// A chat message
#[derive(Debug, Clone)]
pub struct Message {
    /// Role of the message sender
    pub role: MessageRole,
    /// Content of the message
    pub content: String,
}

impl Message {
    /// Create a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }

    /// Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}

/// LLM response
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// Response content
    pub content: String,
    /// Model used
    pub model: String,
    /// Usage statistics
    pub usage: Option<UsageStats>,
}

/// Usage statistics for LLM calls
#[derive(Debug, Clone)]
pub struct UsageStats {
    /// Input tokens
    pub input_tokens: u32,
    /// Output tokens
    pub output_tokens: u32,
}

/// Stub LLM client (to be fully implemented in Task 3)
pub struct LlmClient {
    config: LlmConfig,
}

impl LlmClient {
    /// Create a new LLM client
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    /// Get the provider name
    pub fn provider(&self) -> &str {
        &self.config.provider
    }

    /// Get the model name
    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// Get the configuration
    pub fn config(&self) -> &LlmConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_client_creation() {
        let config = LlmConfig::default();
        let client = LlmClient::new(config);
        assert_eq!(client.provider(), "anthropic");
        assert_eq!(client.model(), "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_message_creation() {
        let system = Message::system("You are a helpful assistant.");
        assert_eq!(system.role, MessageRole::System);
        assert_eq!(system.content, "You are a helpful assistant.");

        let user = Message::user("Hello!");
        assert_eq!(user.role, MessageRole::User);

        let assistant = Message::assistant("Hi there!");
        assert_eq!(assistant.role, MessageRole::Assistant);
    }

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.provider, "anthropic");
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 4096);
    }
}
