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

    // ============================================================
    // LlmConfig tests
    // ============================================================

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.provider, "anthropic");
        assert_eq!(config.model, "claude-3-5-sonnet-20241022");
        assert!(config.api_key.is_none());
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_llm_config_custom_provider() {
        let config = LlmConfig {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            api_key: None,
            temperature: 0.5,
            max_tokens: 8192,
        };
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.temperature, 0.5);
        assert_eq!(config.max_tokens, 8192);
    }

    #[test]
    fn test_llm_config_with_api_key() {
        let config = LlmConfig {
            api_key: Some("sk-test-key".to_string()),
            ..Default::default()
        };
        assert_eq!(config.api_key, Some("sk-test-key".to_string()));
    }

    #[test]
    fn test_llm_config_temperature_extremes() {
        let config_min = LlmConfig {
            temperature: 0.0,
            ..Default::default()
        };
        assert_eq!(config_min.temperature, 0.0);

        let config_max = LlmConfig {
            temperature: 2.0,
            ..Default::default()
        };
        assert_eq!(config_max.temperature, 2.0);
    }

    #[test]
    fn test_llm_config_high_max_tokens() {
        let config = LlmConfig {
            max_tokens: 128000,
            ..Default::default()
        };
        assert_eq!(config.max_tokens, 128000);
    }

    #[test]
    fn test_llm_config_clone() {
        let config = LlmConfig::default();
        let cloned = config.clone();
        assert_eq!(config.provider, cloned.provider);
        assert_eq!(config.model, cloned.model);
    }

    #[test]
    fn test_llm_config_debug() {
        let config = LlmConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("anthropic"));
        assert!(debug_str.contains("claude"));
    }

    // ============================================================
    // MessageRole tests
    // ============================================================

    #[test]
    fn test_message_role_system() {
        let role = MessageRole::System;
        assert_eq!(role, MessageRole::System);
        assert_ne!(role, MessageRole::User);
        assert_ne!(role, MessageRole::Assistant);
    }

    #[test]
    fn test_message_role_user() {
        let role = MessageRole::User;
        assert_eq!(role, MessageRole::User);
        assert_ne!(role, MessageRole::System);
        assert_ne!(role, MessageRole::Assistant);
    }

    #[test]
    fn test_message_role_assistant() {
        let role = MessageRole::Assistant;
        assert_eq!(role, MessageRole::Assistant);
        assert_ne!(role, MessageRole::System);
        assert_ne!(role, MessageRole::User);
    }

    #[test]
    fn test_message_role_equality() {
        assert_eq!(MessageRole::System, MessageRole::System);
        assert_eq!(MessageRole::User, MessageRole::User);
        assert_eq!(MessageRole::Assistant, MessageRole::Assistant);
    }

    #[test]
    fn test_message_role_clone() {
        let role = MessageRole::User;
        let cloned = role.clone();
        assert_eq!(role, cloned);
    }

    #[test]
    fn test_message_role_debug() {
        let debug_str = format!("{:?}", MessageRole::System);
        assert!(debug_str.contains("System"));
    }

    // ============================================================
    // Message tests
    // ============================================================

    #[test]
    fn test_message_system() {
        let msg = Message::system("You are a helpful assistant.");
        assert_eq!(msg.role, MessageRole::System);
        assert_eq!(msg.content, "You are a helpful assistant.");
    }

    #[test]
    fn test_message_user() {
        let msg = Message::user("Hello!");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello!");
    }

    #[test]
    fn test_message_assistant() {
        let msg = Message::assistant("Hi there!");
        assert_eq!(msg.role, MessageRole::Assistant);
        assert_eq!(msg.content, "Hi there!");
    }

    #[test]
    fn test_message_empty_content() {
        let msg = Message::user("");
        assert_eq!(msg.content, "");
    }

    #[test]
    fn test_message_long_content() {
        let long_content = "x".repeat(10000);
        let msg = Message::user(long_content.clone());
        assert_eq!(msg.content, long_content);
    }

    #[test]
    fn test_message_multiline_content() {
        let multiline = "Line 1\nLine 2\nLine 3";
        let msg = Message::user(multiline);
        assert_eq!(msg.content, multiline);
    }

    #[test]
    fn test_message_unicode_content() {
        let unicode = "Hello 你好 🎉";
        let msg = Message::user(unicode);
        assert_eq!(msg.content, unicode);
    }

    #[test]
    fn test_message_from_string() {
        let msg = Message::user(String::from("test"));
        assert_eq!(msg.content, "test");
    }

    #[test]
    fn test_message_clone() {
        let msg = Message::user("test");
        let cloned = msg.clone();
        assert_eq!(msg.role, cloned.role);
        assert_eq!(msg.content, cloned.content);
    }

    #[test]
    fn test_message_debug() {
        let msg = Message::user("test");
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("User"));
        assert!(debug_str.contains("test"));
    }

    // ============================================================
    // LlmResponse tests
    // ============================================================

    #[test]
    fn test_llm_response_basic() {
        let response = LlmResponse {
            content: "Hello, world!".to_string(),
            model: "claude-3".to_string(),
            usage: None,
        };
        assert_eq!(response.content, "Hello, world!");
        assert_eq!(response.model, "claude-3");
        assert!(response.usage.is_none());
    }

    #[test]
    fn test_llm_response_with_usage() {
        let response = LlmResponse {
            content: "Response".to_string(),
            model: "gpt-4".to_string(),
            usage: Some(UsageStats {
                input_tokens: 100,
                output_tokens: 50,
            }),
        };
        assert!(response.usage.is_some());
        let usage = response.usage.unwrap();
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
    }

    #[test]
    fn test_llm_response_empty_content() {
        let response = LlmResponse {
            content: "".to_string(),
            model: "test".to_string(),
            usage: None,
        };
        assert_eq!(response.content, "");
    }

    #[test]
    fn test_llm_response_clone() {
        let response = LlmResponse {
            content: "test".to_string(),
            model: "claude".to_string(),
            usage: None,
        };
        let cloned = response.clone();
        assert_eq!(response.content, cloned.content);
        assert_eq!(response.model, cloned.model);
    }

    #[test]
    fn test_llm_response_debug() {
        let response = LlmResponse {
            content: "test".to_string(),
            model: "claude".to_string(),
            usage: None,
        };
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("LlmResponse"));
    }

    // ============================================================
    // UsageStats tests
    // ============================================================

    #[test]
    fn test_usage_stats_basic() {
        let stats = UsageStats {
            input_tokens: 100,
            output_tokens: 50,
        };
        assert_eq!(stats.input_tokens, 100);
        assert_eq!(stats.output_tokens, 50);
    }

    #[test]
    fn test_usage_stats_zero_tokens() {
        let stats = UsageStats {
            input_tokens: 0,
            output_tokens: 0,
        };
        assert_eq!(stats.input_tokens, 0);
        assert_eq!(stats.output_tokens, 0);
    }

    #[test]
    fn test_usage_stats_large_values() {
        let stats = UsageStats {
            input_tokens: 1000000,
            output_tokens: 500000,
        };
        assert_eq!(stats.input_tokens, 1000000);
        assert_eq!(stats.output_tokens, 500000);
    }

    #[test]
    fn test_usage_stats_clone() {
        let stats = UsageStats {
            input_tokens: 100,
            output_tokens: 50,
        };
        let cloned = stats.clone();
        assert_eq!(stats.input_tokens, cloned.input_tokens);
        assert_eq!(stats.output_tokens, cloned.output_tokens);
    }

    #[test]
    fn test_usage_stats_debug() {
        let stats = UsageStats {
            input_tokens: 100,
            output_tokens: 50,
        };
        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("UsageStats"));
    }

    // ============================================================
    // LlmClient tests
    // ============================================================

    #[test]
    fn test_llm_client_new() {
        let config = LlmConfig::default();
        let client = LlmClient::new(config);
        assert_eq!(client.provider(), "anthropic");
        assert_eq!(client.model(), "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_llm_client_provider() {
        let config = LlmConfig {
            provider: "openai".to_string(),
            ..Default::default()
        };
        let client = LlmClient::new(config);
        assert_eq!(client.provider(), "openai");
    }

    #[test]
    fn test_llm_client_model() {
        let config = LlmConfig {
            model: "gpt-4-turbo".to_string(),
            ..Default::default()
        };
        let client = LlmClient::new(config);
        assert_eq!(client.model(), "gpt-4-turbo");
    }

    #[test]
    fn test_llm_client_config() {
        let config = LlmConfig {
            temperature: 0.9,
            ..Default::default()
        };
        let client = LlmClient::new(config);
        let retrieved_config = client.config();
        assert_eq!(retrieved_config.temperature, 0.9);
    }

    #[test]
    fn test_llm_client_with_api_key() {
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let client = LlmClient::new(config);
        assert_eq!(client.config().api_key, Some("sk-test".to_string()));
    }
}
