//! LLM Provider implementations using rig-core
//!
//! This module provides the LLM provider trait and implementations using the rig-core library.

use std::future::Future;
use std::pin::Pin;

use crate::error::{Result, ZcodeError};
use crate::llm::{LlmConfig, LlmResponse, Message, MessageRole};
use crate::llm::streaming::StreamingResponse;

/// Trait for LLM providers (async)
pub trait LlmProvider: Send + Sync {
    /// Generate a completion from a prompt
    fn complete(&self, prompt: &str) -> Pin<Box<dyn Future<Output = Result<String>> + Send + '_>>;

    /// Generate a completion from a conversation
    fn chat(
        &self,
        messages: &[Message],
    ) -> Pin<Box<dyn Future<Output = Result<LlmResponse>> + Send + '_>>;

    /// Stream a completion (returns a stream of text chunks)
    fn stream_chat(
        &self,
        messages: &[Message],
    ) -> Pin<Box<dyn Future<Output = Result<StreamingResponse>> + Send + '_>>;
}

// ================================================================
// RigProvider - real API calls via rig-core
// ================================================================

/// Rig-based LLM provider using rig-core
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

        let env_var = match self.config.provider.as_str() {
            "anthropic" => "ANTHROPIC_API_KEY",
            "openai" => "OPENAI_API_KEY",
            _ => "API_KEY",
        };

        std::env::var(env_var).map_err(|_| ZcodeError::MissingApiKey(self.config.provider.clone()))
    }
}

impl LlmProvider for RigProvider {
    fn complete(&self, prompt: &str) -> Pin<Box<dyn Future<Output = Result<String>> + Send + '_>> {
        let config = self.config.clone();
        let prompt = prompt.to_string();
        let api_key = match self.get_api_key() {
            Ok(k) => k,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        Box::pin(async move {
            use rig::client::CompletionClient;
            use rig::completion::Prompt;

            let client = rig::providers::anthropic::Client::new(&api_key)
                .map_err(|e| ZcodeError::LlmApiError(format!("Failed to create client: {}", e)))?;

            let agent = client
                .agent(&config.model)
                .preamble("You are a helpful programming assistant.")
                .build();

            let response = agent
                .prompt(&prompt)
                .await
                .map_err(|e| ZcodeError::LlmApiError(format!("Completion failed: {}", e)))?;

            Ok(response)
        })
    }

    fn chat(
        &self,
        messages: &[Message],
    ) -> Pin<Box<dyn Future<Output = Result<LlmResponse>> + Send + '_>> {
        let config = self.config.clone();
        let messages = messages.to_vec();
        let api_key = match self.get_api_key() {
            Ok(k) => k,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        Box::pin(async move {
            use rig::client::CompletionClient;
            use rig::completion::Prompt;

            let client = rig::providers::anthropic::Client::new(&api_key)
                .map_err(|e| ZcodeError::LlmApiError(format!("Failed to create client: {}", e)))?;

            let mut builder = client.agent(&config.model);

            // Add system message as preamble if present
            for msg in &messages {
                if msg.role == MessageRole::System {
                    builder = builder.preamble(&msg.content);
                    break;
                }
            }

            let agent = builder.build();

            // Build conversation prompt from non-system messages
            let prompt = messages
                .iter()
                .filter(|m| m.role != MessageRole::System)
                .map(|m| match m.role {
                    MessageRole::User => format!("User: {}", m.content),
                    MessageRole::Assistant => format!("Assistant: {}", m.content),
                    _ => m.content.clone(),
                })
                .collect::<Vec<_>>()
                .join("\n");

            let response = agent
                .prompt(&prompt)
                .await
                .map_err(|e| ZcodeError::LlmApiError(format!("Chat failed: {}", e)))?;

            Ok(LlmResponse {
                content: response,
                model: config.model.clone(),
                usage: None,
            })
        })
    }

    fn stream_chat(
        &self,
        messages: &[Message],
    ) -> Pin<Box<dyn Future<Output = Result<StreamingResponse>> + Send + '_>> {
        // Delegate to chat and wrap the single response as a stream
        let config = self.config.clone();
        let messages = messages.to_vec();
        Box::pin(async move {
            let provider = RigProvider { config };
            let response = provider.chat(&messages).await?;
            let content = response.content;
            let chunks = vec![Ok(content)];
            Ok(Box::pin(futures::stream::iter(chunks)) as StreamingResponse)
        })
    }
}

// ================================================================
// MockLlmProvider - for testing
// ================================================================

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
    fn complete(&self, _prompt: &str) -> Pin<Box<dyn Future<Output = Result<String>> + Send + '_>> {
        let resp = self.response.clone();
        Box::pin(async move { Ok(resp) })
    }

    fn chat(
        &self,
        _messages: &[Message],
    ) -> Pin<Box<dyn Future<Output = Result<LlmResponse>> + Send + '_>> {
        let resp = self.response.clone();
        Box::pin(async move {
            Ok(LlmResponse {
                content: resp,
                model: "mock-model".to_string(),
                usage: Some(crate::llm::UsageStats {
                    input_tokens: 10,
                    output_tokens: 5,
                }),
            })
        })
    }

    fn stream_chat(
        &self,
        _messages: &[Message],
    ) -> Pin<Box<dyn Future<Output = Result<StreamingResponse>> + Send + '_>> {
        let resp = self.response.clone();
        Box::pin(async move {
            let chunks = vec![Ok(resp)];
            Ok(Box::pin(futures::stream::iter(chunks)) as StreamingResponse)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider() {
        let provider = MockLlmProvider::new("Hello, world!");
        let result = provider.complete("test").await.unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[tokio::test]
    async fn test_mock_provider_chat() {
        let provider = MockLlmProvider::new("Response");
        let messages = vec![Message::user("Hello")];
        let response = provider.chat(&messages).await.unwrap();
        assert_eq!(response.content, "Response");
    }

    #[tokio::test]
    async fn test_mock_provider_stream() {
        use futures::StreamExt;

        let provider = MockLlmProvider::new("Streamed response");
        let messages = vec![Message::user("Hello")];
        let mut stream = provider.stream_chat(&messages).await.unwrap();
        let mut collected = String::new();
        while let Some(chunk) = stream.next().await {
            collected.push_str(&chunk.unwrap());
        }
        assert_eq!(collected, "Streamed response");
    }

    #[test]
    fn test_rig_provider_creation() {
        let config = LlmConfig::default();
        let provider = RigProvider::new(config);
        assert_eq!(provider.config().provider, "anthropic");
    }
}
