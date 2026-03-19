//! LLM Provider implementations using rig-core
//!
//! This module provides the LLM provider trait and implementations using the rig-core library.

use crate::error::{Result, ZcodeError};
use crate::llm::{LlmConfig, LlmResponse, Message};

/// Trait for LLM providers
pub trait LlmProvider: Send + Sync {
    /// Generate a completion from a prompt
    fn complete(&self, prompt: &str) -> Result<String>;

    /// Generate a completion from a conversation
    fn chat(&self, messages: &[Message]) -> Result<LlmResponse>;

    /// Stream a completion (returns a stream of text chunks)
    fn stream_complete(&self, prompt: &str) -> Result<StreamingResponse>;
}

/// Streaming response type
pub type StreamingResponse = std::pin::Pin<Box<dyn futures::Stream<Item = Result<String>> + Send>>;

/// Rig-based LLM provider
pub struct RigProvider {
    config: LlmConfig,
}

impl RigProvider {
    /// Create a new Rig provider
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    /// Get the configuration
    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// Get the API key from config or environment
    fn get_api_key(&self) -> Result<String> {
        if let Some(ref key) = self.config.api_key {
            return Ok(key.clone());
        }

        // Try environment variable based on provider
        let env_var = match self.config.provider.as_str() {
            "anthropic" => "ANTHROPIC_API_KEY",
            "openai" => "OPENAI_API_KEY",
            _ => "API_KEY",
        };

        std::env::var(env_var).map_err(|_| ZcodeError::MissingApiKey(self.config.provider.clone()))
    }
}

impl LlmProvider for RigProvider {
    fn complete(&self, prompt: &str) -> Result<String> {
        // Placeholder implementation - will be fully implemented in Task 3
        // This uses a simple stub for now
        let _api_key = self.get_api_key()?;

        // For now, return a placeholder response
        // Real implementation will use rig-core to make actual API calls
        Ok(format!("[Stub response for: {}]", prompt))
    }

    fn chat(&self, messages: &[Message]) -> Result<LlmResponse> {
        // Placeholder implementation - will be fully implemented in Task 3
        let _api_key = self.get_api_key()?;

        // Build a simple response from messages for testing
        let last_user_msg = messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, crate::llm::MessageRole::User))
            .map(|m| m.content.as_str())
            .unwrap_or("no message");

        Ok(LlmResponse {
            content: format!("[Stub response to: {}]", last_user_msg),
            model: self.config.model.clone(),
            usage: Some(crate::llm::UsageStats {
                input_tokens: 100,
                output_tokens: 50,
            }),
        })
    }

    fn stream_complete(&self, prompt: &str) -> Result<StreamingResponse> {
        // Placeholder implementation - will be fully implemented in Task 3
        let _api_key = self.get_api_key()?;

        // Create a simple stream that returns chunks
        let chunks = vec![
            Ok("[Stub ".to_string()),
            Ok("streaming ".to_string()),
            Ok("response ".to_string()),
            Ok(format!("for: {}]", prompt)),
        ];

        Ok(Box::pin(futures::stream::iter(chunks)))
    }
}

/// Mock LLM provider for testing
pub struct MockLlmProvider {
    response: String,
}

impl MockLlmProvider {
    /// Create a new mock provider with a fixed response
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }
}

impl LlmProvider for MockLlmProvider {
    fn complete(&self, _prompt: &str) -> Result<String> {
        Ok(self.response.clone())
    }

    fn chat(&self, _messages: &[Message]) -> Result<LlmResponse> {
        Ok(LlmResponse {
            content: self.response.clone(),
            model: "mock-model".to_string(),
            usage: Some(crate::llm::UsageStats {
                input_tokens: 10,
                output_tokens: 5,
            }),
        })
    }

    fn stream_complete(&self, _prompt: &str) -> Result<StreamingResponse> {
        let response = self.response.clone();
        let chunks = vec![Ok(response)];
        Ok(Box::pin(futures::stream::iter(chunks)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_provider() {
        let provider = MockLlmProvider::new("Hello, world!");
        let result = provider.complete("test").unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_mock_provider_chat() {
        let provider = MockLlmProvider::new("Response");
        let messages = vec![Message::user("Hello")];
        let response = provider.chat(&messages).unwrap();
        assert_eq!(response.content, "Response");
    }

    #[test]
    fn test_rig_provider_creation() {
        let config = LlmConfig::default();
        let provider = RigProvider::new(config);
        assert_eq!(provider.config().provider, "anthropic");
    }
}
