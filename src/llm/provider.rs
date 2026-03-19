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
    use crate::llm::{LlmConfig, Message};

    // ============================================================
    // MockLlmProvider tests
    // ============================================================

    #[test]
    fn test_mock_provider_new() {
        let provider = MockLlmProvider::new("Hello, world!");
        let result = provider.complete("test").unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_mock_provider_complete_empty() {
        let provider = MockLlmProvider::new("");
        let result = provider.complete("test").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_mock_provider_complete_long_response() {
        let long_response = "x".repeat(10000);
        let provider = MockLlmProvider::new(long_response.clone());
        let result = provider.complete("test").unwrap();
        assert_eq!(result, long_response);
    }

    #[test]
    fn test_mock_provider_complete_ignores_prompt() {
        let provider = MockLlmProvider::new("Fixed response");
        let result1 = provider.complete("prompt 1").unwrap();
        let result2 = provider.complete("prompt 2").unwrap();
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_mock_provider_chat_basic() {
        let provider = MockLlmProvider::new("Response");
        let messages = vec![Message::user("Hello")];
        let response = provider.chat(&messages).unwrap();
        assert_eq!(response.content, "Response");
    }

    #[test]
    fn test_mock_provider_chat_model_field() {
        let provider = MockLlmProvider::new("Response");
        let messages = vec![Message::user("Hello")];
        let response = provider.chat(&messages).unwrap();
        assert_eq!(response.model, "mock-model");
    }

    #[test]
    fn test_mock_provider_chat_usage_stats() {
        let provider = MockLlmProvider::new("Response");
        let messages = vec![Message::user("Hello")];
        let response = provider.chat(&messages).unwrap();
        assert!(response.usage.is_some());
        let usage = response.usage.unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 5);
    }

    #[test]
    fn test_mock_provider_chat_empty_messages() {
        let provider = MockLlmProvider::new("Response");
        let messages: Vec<Message> = vec![];
        let response = provider.chat(&messages).unwrap();
        assert_eq!(response.content, "Response");
    }

    #[test]
    fn test_mock_provider_chat_multiple_messages() {
        let provider = MockLlmProvider::new("Response");
        let messages = vec![
            Message::system("You are helpful"),
            Message::user("Hi"),
            Message::assistant("Hello"),
            Message::user("How are you?"),
        ];
        let response = provider.chat(&messages).unwrap();
        assert_eq!(response.content, "Response");
    }

    #[tokio::test]
    async fn test_mock_provider_stream_complete() {
        let provider = MockLlmProvider::new("Stream response");
        let stream = provider.stream_complete("test").unwrap();

        use futures::StreamExt;
        let chunks: Vec<_> = stream.collect().await;

        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].is_ok());
        assert_eq!(chunks[0].as_ref().unwrap(), "Stream response");
    }

    // ============================================================
    // RigProvider tests
    // ============================================================

    #[test]
    fn test_rig_provider_new() {
        let config = LlmConfig::default();
        let provider = RigProvider::new(config);
        assert_eq!(provider.config().provider, "anthropic");
    }

    #[test]
    fn test_rig_provider_config() {
        let config = LlmConfig {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            temperature: 0.5,
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let retrieved_config = provider.config();
        assert_eq!(retrieved_config.provider, "openai");
        assert_eq!(retrieved_config.model, "gpt-4");
        assert_eq!(retrieved_config.temperature, 0.5);
    }

    #[test]
    fn test_rig_provider_complete_with_api_key() {
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let result = provider.complete("test prompt");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Stub response"));
    }

    #[test]
    fn test_rig_provider_complete_includes_prompt() {
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let result = provider.complete("my prompt").unwrap();
        assert!(result.contains("my prompt"));
    }

    #[test]
    fn test_rig_provider_complete_missing_api_key() {
        let config = LlmConfig {
            api_key: None,
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let result = provider.complete("test");
        assert!(result.is_err());
        match result.unwrap_err() {
            ZcodeError::MissingApiKey(provider_name) => {
                assert_eq!(provider_name, "anthropic");
            }
            _ => panic!("Expected MissingApiKey error"),
        }
    }

    #[test]
    fn test_rig_provider_chat_with_api_key() {
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let messages = vec![Message::user("Hello")];
        let result = provider.chat(&messages);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rig_provider_chat_response_model() {
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            model: "gpt-4".to_string(),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let messages = vec![Message::user("Hello")];
        let response = provider.chat(&messages).unwrap();
        assert_eq!(response.model, "gpt-4");
    }

    #[test]
    fn test_rig_provider_chat_finds_last_user_message() {
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let messages = vec![
            Message::user("First message"),
            Message::assistant("Response"),
            Message::user("Last message"),
        ];
        let response = provider.chat(&messages).unwrap();
        assert!(response.content.contains("Last message"));
    }

    #[test]
    fn test_rig_provider_chat_no_user_message() {
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let messages = vec![Message::assistant("Just assistant")];
        let response = provider.chat(&messages).unwrap();
        assert!(response.content.contains("no message"));
    }

    #[test]
    fn test_rig_provider_chat_usage_stats() {
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let messages = vec![Message::user("Hello")];
        let response = provider.chat(&messages).unwrap();
        assert!(response.usage.is_some());
        let usage = response.usage.unwrap();
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
    }

    #[test]
    fn test_rig_provider_chat_missing_api_key() {
        let config = LlmConfig {
            api_key: None,
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let messages = vec![Message::user("Hello")];
        let result = provider.chat(&messages);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rig_provider_stream_complete_with_api_key() {
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let stream = provider.stream_complete("test").unwrap();

        use futures::StreamExt;
        let chunks: Vec<_> = stream.collect().await;

        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(chunk.is_ok());
        }
    }

    #[tokio::test]
    async fn test_rig_provider_stream_complete_content() {
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let stream = provider.stream_complete("test prompt").unwrap();

        use futures::StreamExt;
        let chunks: Vec<_> = stream.collect().await;

        let full_content: String = chunks
            .iter()
            .filter_map(|c| c.as_ref().ok())
            .cloned()
            .collect();

        assert!(full_content.contains("test prompt"));
    }

    #[test]
    fn test_rig_provider_stream_complete_missing_api_key() {
        let config = LlmConfig {
            api_key: None,
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let result = provider.stream_complete("test");
        assert!(result.is_err());
    }

    // ============================================================
    // API key environment variable tests
    // ============================================================

    #[test]
    fn test_rig_provider_get_api_key_from_config() {
        let config = LlmConfig {
            api_key: Some("sk-from-config".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let result = provider.complete("test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_rig_provider_openai_api_key_env() {
        // Save original env var
        let original = std::env::var("OPENAI_API_KEY").ok();

        let config = LlmConfig {
            provider: "openai".to_string(),
            api_key: None,
            ..Default::default()
        };
        let provider = RigProvider::new(config);

        // Without env var set
        std::env::remove_var("OPENAI_API_KEY");
        let result = provider.complete("test");
        assert!(result.is_err());

        // Restore original
        if let Some(val) = original {
            std::env::set_var("OPENAI_API_KEY", val);
        }
    }

    // ============================================================
    // LlmProvider trait tests
    // ============================================================

    #[test]
    fn test_llm_provider_trait_mock() {
        let provider = MockLlmProvider::new("test");
        // Verify trait object creation works
        let _trait_obj: &dyn LlmProvider = &provider;
    }

    #[test]
    fn test_llm_provider_trait_rig() {
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        // Verify trait object creation works
        let _trait_obj: &dyn LlmProvider = &provider;
    }

    // ============================================================
    // StreamingResponse type tests
    // ============================================================

    #[tokio::test]
    async fn test_streaming_response_type() {
        let chunks = vec![
            Ok("Hello ".to_string()),
            Ok("world!".to_string()),
        ];
        let stream: StreamingResponse = Box::pin(futures::stream::iter(chunks));

        use futures::StreamExt;
        let collected: Vec<_> = stream.collect().await;
        assert_eq!(collected.len(), 2);
    }

    // ============================================================
    // Edge cases
    // ============================================================

    #[test]
    fn test_mock_provider_special_characters() {
        let provider = MockLlmProvider::new("Response with \"quotes\" and 'apostrophes'");
        let result = provider.complete("test").unwrap();
        assert!(result.contains("quotes"));
    }

    #[test]
    fn test_mock_provider_unicode() {
        let provider = MockLlmProvider::new("Hello 你好 🎉");
        let result = provider.complete("test").unwrap();
        assert!(result.contains("你好"));
    }

    #[test]
    fn test_mock_provider_newlines() {
        let provider = MockLlmProvider::new("Line 1\nLine 2\nLine 3");
        let result = provider.complete("test").unwrap();
        assert!(result.contains('\n'));
    }

    #[test]
    fn test_rig_provider_custom_provider_api_key_env() {
        let config = LlmConfig {
            provider: "custom_provider".to_string(),
            api_key: None,
            ..Default::default()
        };
        let provider = RigProvider::new(config);

        // Should look for API_KEY env var for unknown providers
        let original = std::env::var("API_KEY").ok();
        std::env::remove_var("API_KEY");

        let result = provider.complete("test");
        assert!(result.is_err());

        // Restore
        if let Some(val) = original {
            std::env::set_var("API_KEY", val);
        }
    }
}
