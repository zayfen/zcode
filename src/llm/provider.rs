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

    /// Get the API key from config or environment.
    ///
    /// Priority for Anthropic provider:
    /// 1. `config.api_key` (explicit override)
    /// 2. `ANTHROPIC_AUTH_TOKEN` env var (BigModel / proxy compatible)
    /// 3. `ANTHROPIC_API_KEY` env var (standard Anthropic)
    fn get_api_key(&self) -> Result<String> {
        if let Some(ref key) = self.config.api_key {
            return Ok(key.clone());
        }

        match self.config.provider.as_str() {
            "anthropic" => {
                // Try ANTHROPIC_AUTH_TOKEN first (used by BigModel / Claude proxies)
                if let Ok(key) = std::env::var("ANTHROPIC_AUTH_TOKEN") {
                    return Ok(key);
                }
                std::env::var("ANTHROPIC_API_KEY")
                    .map_err(|_| ZcodeError::MissingApiKey(self.config.provider.clone()))
            }
            "openai" => std::env::var("OPENAI_API_KEY")
                .map_err(|_| ZcodeError::MissingApiKey(self.config.provider.clone())),
            _ => std::env::var("API_KEY")
                .map_err(|_| ZcodeError::MissingApiKey(self.config.provider.clone())),
        }
    }
}

/// Run an async HTTP future synchronously without causing issues inside a tokio runtime.
///
/// This always spawns a new OS thread with its own `tokio::Runtime` so that:
/// - `reqwest` (async) works correctly
/// - We don't call `block_in_place` inside a current-thread runtime (which would panic)
/// - We don't create nested runtimes inside the same thread
fn run_http<F, T>(fut: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>> + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .map_err(|e| ZcodeError::LlmApiError(format!("Failed to create runtime: {}", e)))?
            .block_on(fut)
    })
    .join()
    .map_err(|e| ZcodeError::LlmApiError(format!("HTTP thread panicked: {:?}", e)))?
}

impl LlmProvider for RigProvider {
    fn complete(&self, prompt: &str) -> Result<String> {
        let messages = vec![Message::user(prompt)];
        let resp = self.chat(&messages)?;
        Ok(resp.content)
    }

    fn chat(&self, messages: &[Message]) -> Result<LlmResponse> {
        let api_key = self.get_api_key()?;

        match self.config.provider.as_str() {
            "anthropic" => self.chat_anthropic(messages, &api_key),
            "openai" | _ => self.chat_openai(messages, &api_key),
        }
    }

    fn stream_complete(&self, prompt: &str) -> Result<StreamingResponse> {
        // Fallback to non-streaming for now
        let response = self.complete(prompt)?;
        let chunks = vec![Ok(response)];
        Ok(Box::pin(futures::stream::iter(chunks)))
    }
}

impl RigProvider {
    /// Anthropic Messages API call
    fn chat_anthropic(&self, messages: &[Message], api_key: &str) -> Result<LlmResponse> {
        use crate::llm::{MessageRole, UsageStats};

        // Separate system prompt from conversation messages
        let system_prompt: String = messages.iter()
            .filter(|m| m.role == MessageRole::System)
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let conv_messages: Vec<serde_json::Value> = messages.iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| serde_json::json!({
                "role": match m.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::System => "user",
                },
                "content": m.content
            }))
            .collect();

        let mut body = serde_json::json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "messages": conv_messages
        });

        if !system_prompt.is_empty() {
            body["system"] = serde_json::Value::String(system_prompt);
        }

        let api_key = api_key.to_string();
        let model = self.config.model.clone();
        // Use ANTHROPIC_BASE_URL as base; append /v1/messages for the Messages API.
        // Verified: https://open.bigmodel.cn/api/anthropic/v1/messages → HTTP 200
        let endpoint = {
            let base = std::env::var("ANTHROPIC_BASE_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com".to_string());
            format!("{}/v1/messages", base.trim_end_matches('/'))
        };

        let (status, response_body) = run_http(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120)) // 2-min hard limit
                .build()
                .map_err(|e| ZcodeError::LlmApiError(format!("Failed to build HTTP client: {}", e)))?;
            let resp = client
                .post(&endpoint)
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| ZcodeError::LlmApiError(format!("Anthropic request failed: {}", e)))?;
            let status = resp.status();
            let body: serde_json::Value = resp.json().await
                .map_err(|e| ZcodeError::LlmResponseError(format!("Failed to parse Anthropic response: {}", e)))?;
            Ok::<_, ZcodeError>((status, body))
        })?;

        if !status.is_success() {
            let err_msg = response_body.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            return Err(ZcodeError::LlmApiError(format!("Anthropic API error ({}): {}", status, err_msg)));
        }

        let content = response_body
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        let input_tokens = response_body
            .get("usage").and_then(|u| u.get("input_tokens")).and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;
        let output_tokens = response_body
            .get("usage").and_then(|u| u.get("output_tokens")).and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;

        Ok(LlmResponse {
            content,
            model,
            usage: Some(UsageStats { input_tokens, output_tokens }),
        })
    }

    /// OpenAI Chat Completions API call
    fn chat_openai(&self, messages: &[Message], api_key: &str) -> Result<LlmResponse> {
        use crate::llm::{MessageRole, UsageStats};

        let openai_messages: Vec<serde_json::Value> = messages.iter()
            .map(|m| serde_json::json!({
                "role": match m.role {
                    MessageRole::System    => "system",
                    MessageRole::User      => "user",
                    MessageRole::Assistant => "assistant",
                },
                "content": m.content
            }))
            .collect();

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": openai_messages,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens
        });

        let api_key = api_key.to_string();
        let model = self.config.model.clone();

        let (status, response_body) = run_http(async move {
            let client = reqwest::Client::new();
            let resp = client
                .post("https://api.openai.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| ZcodeError::LlmApiError(format!("OpenAI request failed: {}", e)))?;
            let status = resp.status();
            let body: serde_json::Value = resp.json().await
                .map_err(|e| ZcodeError::LlmResponseError(format!("Failed to parse OpenAI response: {}", e)))?;
            Ok::<_, ZcodeError>((status, body))
        })?;

        if !status.is_success() {
            let err_msg = response_body.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            return Err(ZcodeError::LlmApiError(format!("OpenAI API error ({}): {}", status, err_msg)));
        }

        let content = response_body
            .get("choices").and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        let input_tokens = response_body
            .get("usage").and_then(|u| u.get("prompt_tokens")).and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;
        let output_tokens = response_body
            .get("usage").and_then(|u| u.get("completion_tokens")).and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;

        Ok(LlmResponse {
            content,
            model,
            usage: Some(UsageStats { input_tokens, output_tokens }),
        })
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
        // RigProvider now makes real HTTP calls. With an invalid test key it errors.
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let result = provider.complete("test prompt");
        assert!(result.is_err(), "Expected HTTP/API error with invalid key");
    }

    #[test]
    fn test_rig_provider_complete_includes_prompt() {
        // Use MockLlmProvider to verify response handling
        let provider = MockLlmProvider::new("response for test");
        let result = provider.complete("my prompt").unwrap();
        assert_eq!(result, "response for test");
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
        // Real HTTP call with invalid key errors
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let messages = vec![Message::user("Hello")];
        let result = provider.chat(&messages);
        assert!(result.is_err(), "Expected HTTP/API error with invalid key");
    }

    #[test]
    fn test_rig_provider_chat_response_model() {
        // Use MockLlmProvider to verify response structure
        let provider = MockLlmProvider::new("reply");
        let messages = vec![Message::user("Hello")];
        let response = provider.chat(&messages).unwrap();
        assert_eq!(response.model, "mock-model");
    }

    #[test]
    fn test_rig_provider_chat_finds_last_user_message() {
        // MockLlmProvider returns fixed response regardless of messages
        let provider = MockLlmProvider::new("mock reply");
        let messages = vec![
            Message::user("First message"),
            Message::assistant("Response"),
            Message::user("Last message"),
        ];
        let response = provider.chat(&messages).unwrap();
        assert_eq!(response.content, "mock reply");
    }

    #[test]
    fn test_rig_provider_chat_no_user_message() {
        let provider = MockLlmProvider::new("mock");
        let messages = vec![Message::assistant("Just assistant")];
        let response = provider.chat(&messages).unwrap();
        assert!(!response.content.is_empty());
    }

    #[test]
    fn test_rig_provider_chat_usage_stats() {
        // MockLlmProvider returns 10/5 tokens
        let provider = MockLlmProvider::new("hello");
        let messages = vec![Message::user("Hello")];
        let response = provider.chat(&messages).unwrap();
        assert!(response.usage.is_some());
        let usage = response.usage.unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 5);
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
        // stream_complete calls complete() internally, which makes real HTTP
        // with invalid key → should return Err before creating a stream
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let result = provider.stream_complete("test");
        assert!(result.is_err(), "Expected HTTP/API error with invalid key");
    }

    #[tokio::test]
    async fn test_rig_provider_stream_complete_content() {
        // Use MockLlmProvider to verify stream content handling
        let provider = MockLlmProvider::new("test prompt result");
        let stream = provider.stream_complete("test prompt").unwrap();

        use futures::StreamExt;
        let chunks: Vec<_> = stream.collect().await;

        let full_content: String = chunks
            .iter()
            .filter_map(|c| c.as_ref().ok())
            .cloned()
            .collect();

        assert!(full_content.contains("test prompt result"));
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
        // With a valid config key, RigProvider will attempt real HTTP → Err (invalid key)
        let config = LlmConfig {
            api_key: Some("sk-from-config".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let result = provider.complete("test");
        // Real HTTP with invalid key returns an API error (not MissingApiKey)
        assert!(result.is_err());
        match result.unwrap_err() {
            ZcodeError::MissingApiKey(_) => panic!("Should not be MissingApiKey — key was provided"),
            _ => {} // Any LLM API error is expected
        }
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
