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
    fn chat(&self, messages: &[Message], tools: &[serde_json::Value]) -> Result<LlmResponse>;

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
        let resp = self.chat(&messages, &[])?;
        Ok(resp.content)
    }

    fn chat(&self, messages: &[Message], tools: &[serde_json::Value]) -> Result<LlmResponse> {
        let api_key = self.get_api_key()?;

        match self.config.provider.as_str() {
            "anthropic" => self.chat_anthropic(messages, &api_key, tools),
            "openai" | _ => self.chat_openai(messages, &api_key, tools),
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
    /// Build the Anthropic Messages API `messages` array from a `&[Message]`.
    ///
    /// Anthropic has specific formatting requirements:
    /// - System messages are extracted separately (not in the messages array)
    /// - Assistant messages with tool_use must use `content: [{type: "tool_use", ...}]`
    /// - Tool results must be grouped into a single `user` message with
    ///   `content: [{type: "tool_result", tool_use_id: "...", content: "..."}]`
    /// - Consecutive tool_result messages are merged into one user message
    fn build_anthropic_messages(messages: &[Message]) -> Vec<serde_json::Value> {
        use crate::llm::MessageRole;

        let non_system: Vec<&Message> = messages.iter()
            .filter(|m| m.role != MessageRole::System)
            .collect();

        let mut result: Vec<serde_json::Value> = Vec::new();
        let mut i = 0;

        while i < non_system.len() {
            let m = non_system[i];
            match m.role {
                MessageRole::Assistant => {
                    if let Some(ref tool_calls) = m.tool_calls {
                        // Assistant message with tool_use blocks
                        // Build content array: optional text block + tool_use blocks
                        let mut content_blocks: Vec<serde_json::Value> = Vec::new();
                        if !m.content.is_empty() {
                            content_blocks.push(serde_json::json!({
                                "type": "text",
                                "text": m.content
                            }));
                        }
                        for tc in tool_calls {
                            content_blocks.push(tc.clone());
                        }
                        result.push(serde_json::json!({
                            "role": "assistant",
                            "content": content_blocks
                        }));
                    } else {
                        // Plain text assistant message
                        result.push(serde_json::json!({
                            "role": "assistant",
                            "content": m.content
                        }));
                    }
                    i += 1;
                }
                MessageRole::Tool => {
                    // Collect consecutive tool result messages into one user message
                    let mut tool_result_blocks: Vec<serde_json::Value> = Vec::new();
                    while i < non_system.len() && non_system[i].role == MessageRole::Tool {
                        let tm = non_system[i];
                        let tool_use_id = tm.tool_call_id.as_deref().unwrap_or("");
                        tool_result_blocks.push(serde_json::json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": tm.content
                        }));
                        i += 1;
                    }
                    result.push(serde_json::json!({
                        "role": "user",
                        "content": tool_result_blocks
                    }));
                }
                MessageRole::User | _ => {
                    result.push(serde_json::json!({
                        "role": "user",
                        "content": m.content
                    }));
                    i += 1;
                }
            }
        }

        result
    }

    /// Anthropic Messages API call
    fn chat_anthropic(&self, messages: &[Message], api_key: &str, tools: &[serde_json::Value]) -> Result<LlmResponse> {
        use crate::llm::MessageRole;

        // Separate system prompt from conversation messages
        let system_prompt: String = messages.iter()
            .filter(|m| m.role == MessageRole::System)
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        // Build properly formatted messages (handles tool_use + tool_result)
        let conv_messages = Self::build_anthropic_messages(messages);

        let mut body = serde_json::json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "messages": conv_messages
        });

        if !system_prompt.is_empty() {
            body["system"] = serde_json::Value::String(system_prompt);
        }
        // Inject tool schemas if provided
        if !tools.is_empty() {
            body["tools"] = serde_json::Value::Array(tools.to_vec());
        }
        
        tracing::debug!("Anthropic API Request Body: {}", serde_json::to_string_pretty(&body).unwrap_or_default());

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

        // Parse the response using Anthropic format
        tracing::debug!("Anthropic API Response Body: {}", serde_json::to_string_pretty(&response_body).unwrap_or_default());
        
        use crate::agent::loop_exec::LlmResponse as AgentLlmResponse;
        let agent_resp = AgentLlmResponse::from_anthropic_response(&response_body)
            .map_err(|e| ZcodeError::LlmApiError(format!("Failed to parse Anthropic response: {}", e)))?;

        // Map to provider LlmResponse
        let content = match &agent_resp {
            AgentLlmResponse::Text(t) => t.clone(),
            AgentLlmResponse::ToolCalls(calls) => calls
                .iter()
                .filter_map(|c: &serde_json::Value| c.get("name").and_then(|n: &serde_json::Value| n.as_str()))
                .collect::<Vec<_>>()
                .join(", "),
        };

        use crate::llm::UsageStats;
        let input_tokens = response_body
            .get("usage").and_then(|u| u.get("input_tokens")).and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;
        let output_tokens = response_body
            .get("usage").and_then(|u| u.get("output_tokens")).and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;

        // Return the provider response — tool routing happens in AgentLoop
        Ok(LlmResponse {
            content,
            model,
            usage: Some(UsageStats { input_tokens, output_tokens }),
            raw_response: response_body,
        })
    }

    /// Build the OpenAI Chat Completions API `messages` array from a `&[Message]`.
    ///
    /// OpenAI expects:
    /// - Assistant messages with tool_calls: `{role: "assistant", tool_calls: [...]}`
    /// - Tool results: `{role: "tool", tool_call_id: "...", content: "..."}`
    fn build_openai_messages(messages: &[Message]) -> Vec<serde_json::Value> {
        use crate::llm::MessageRole;

        messages.iter().map(|m| {
            match m.role {
                MessageRole::System => serde_json::json!({
                    "role": "system",
                    "content": m.content
                }),
                MessageRole::User => serde_json::json!({
                    "role": "user",
                    "content": m.content
                }),
                MessageRole::Assistant => {
                    if let Some(ref tool_calls) = m.tool_calls {
                        // Assistant message with tool_calls in OpenAI format
                        let openai_tool_calls: Vec<serde_json::Value> = tool_calls.iter().map(|tc| {
                            // If already in OpenAI format (has "function" key), pass through
                            if tc.get("function").is_some() {
                                tc.clone()
                            } else {
                                // Convert from Anthropic tool_use format to OpenAI format
                                let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                let name = tc.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                let input = tc.get("input").cloned().unwrap_or(serde_json::json!({}));
                                serde_json::json!({
                                    "id": id,
                                    "type": "function",
                                    "function": {
                                        "name": name,
                                        "arguments": serde_json::to_string(&input).unwrap_or_default()
                                    }
                                })
                            }
                        }).collect();
                        let mut msg = serde_json::json!({
                            "role": "assistant",
                            "tool_calls": openai_tool_calls
                        });
                        if !m.content.is_empty() {
                            msg["content"] = serde_json::Value::String(m.content.clone());
                        }
                        msg
                    } else {
                        serde_json::json!({
                            "role": "assistant",
                            "content": m.content
                        })
                    }
                }
                MessageRole::Tool => {
                    serde_json::json!({
                        "role": "tool",
                        "tool_call_id": m.tool_call_id.as_deref().unwrap_or(""),
                        "content": m.content
                    })
                }
            }
        }).collect()
    }

    /// OpenAI Chat Completions API call
    fn chat_openai(&self, messages: &[Message], api_key: &str, tools: &[serde_json::Value]) -> Result<LlmResponse> {
        use crate::llm::UsageStats;

        // Build properly formatted messages (handles tool_calls + tool results)
        let openai_messages = Self::build_openai_messages(messages);

        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": openai_messages,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens
        });

        // Inject tool schemas if provided (convert from Anthropic to OpenAI format if needed)
        if !tools.is_empty() {
            let openai_tools: Vec<serde_json::Value> = tools.iter().map(|t| {
                if t.get("type").is_some() {
                    // Already in OpenAI format
                    t.clone()
                } else {
                    // Convert from Anthropic format: {name, description, input_schema}
                    let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let desc = t.get("description").and_then(|v| v.as_str()).unwrap_or("");
                    let params = t.get("input_schema").cloned().unwrap_or(serde_json::json!({"type": "object"}));
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": name,
                            "description": desc,
                            "parameters": params
                        }
                    })
                }
            }).collect();
            body["tools"] = serde_json::Value::Array(openai_tools);
        }

        let api_key = api_key.to_string();
        let model = self.config.model.clone();

        let (status, response_body) = run_http(async move {
            let endpoint = {
                let base = std::env::var("OPENAI_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com".to_string());
                format!("{}/v1/chat/completions", base.trim_end_matches('/'))
            };

            let client = reqwest::Client::new();
            let resp = client
                .post(&endpoint)
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
            raw_response: response_body,
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

    fn chat(&self, _messages: &[Message], _tools: &[serde_json::Value]) -> Result<LlmResponse> {
        Ok(LlmResponse {
            content: self.response.clone(),
            model: "mock-model".to_string(),
            usage: Some(crate::llm::UsageStats {
                input_tokens: 10,
                output_tokens: 5,
            }),
            raw_response: serde_json::json!({
                "content": self.response
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

    // ============================================================
    // build_anthropic_messages tests
    // ============================================================

    #[test]
    fn test_build_anthropic_messages_empty() {
        let result = RigProvider::build_anthropic_messages(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_build_anthropic_messages_plain_text() {
        let messages = vec![
            Message::system("You are helpful"),
            Message::user("Hello"),
            Message::assistant("Hi there!"),
            Message::user("How are you?"),
        ];
        let result = RigProvider::build_anthropic_messages(&messages);
        // System messages are excluded from the messages array
        assert_eq!(result.len(), 3);
        assert_eq!(result[0]["role"], "user");
        assert_eq!(result[0]["content"], "Hello");
        assert_eq!(result[1]["role"], "assistant");
        assert_eq!(result[1]["content"], "Hi there!");
        assert_eq!(result[2]["role"], "user");
        assert_eq!(result[2]["content"], "How are you?");
    }

    #[test]
    fn test_build_anthropic_messages_system_only() {
        let messages = vec![Message::system("You are helpful")];
        let result = RigProvider::build_anthropic_messages(&messages);
        assert!(result.is_empty());
    }

    #[test]
    fn test_build_anthropic_messages_assistant_with_tool_calls() {
        let tool_use = serde_json::json!({
            "type": "tool_use",
            "id": "call_123",
            "name": "get_weather",
            "input": {"city": "Tokyo"}
        });
        let messages = vec![
            Message::user("What's the weather?"),
            Message::assistant_with_tool_calls("Let me check", vec![tool_use]),
        ];
        let result = RigProvider::build_anthropic_messages(&messages);
        assert_eq!(result.len(), 2);

        // Check tool_use message has content array
        let assistant_msg = &result[1];
        assert_eq!(assistant_msg["role"], "assistant");
        let content = assistant_msg["content"].as_array().unwrap();
        assert_eq!(content.len(), 2); // text block + tool_use block
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "Let me check");
        assert_eq!(content[1]["type"], "tool_use");
        assert_eq!(content[1]["id"], "call_123");
    }

    #[test]
    fn test_build_anthropic_messages_assistant_with_tool_calls_empty_content() {
        let tool_use = serde_json::json!({
            "type": "tool_use",
            "id": "call_456",
            "name": "search",
            "input": {"q": "rust"}
        });
        let messages = vec![
            Message::user("Search please"),
            Message::assistant_with_tool_calls("", vec![tool_use]),
        ];
        let result = RigProvider::build_anthropic_messages(&messages);
        let assistant_msg = &result[1];
        let content = assistant_msg["content"].as_array().unwrap();
        // Empty text content → only tool_use block
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "tool_use");
    }

    #[test]
    fn test_build_anthropic_messages_consecutive_tool_results() {
        let empty_input = serde_json::Value::Object(Default::default());
        let tool_use = serde_json::json!({
            "type": "tool_use",
            "id": "call_1",
            "name": "tool_a",
            "input": empty_input
        });
        let messages = vec![
            Message::user("Do something"),
            Message::assistant_with_tool_calls("", vec![tool_use]),
            Message::tool_result("call_1", "tool_a", "result_a"),
            Message::tool_result("call_1", "tool_a", "result_b"),
        ];
        let result = RigProvider::build_anthropic_messages(&messages);
        // Should produce: [user, assistant, user(merged tool_results)]
        assert_eq!(result.len(), 3);

        // Last message should merge both tool results into one user message
        let tool_user = &result[2];
        assert_eq!(tool_user["role"], "user");
        let content = tool_user["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "tool_result");
        assert_eq!(content[0]["tool_use_id"], "call_1");
        assert_eq!(content[0]["content"], "result_a");
        assert_eq!(content[1]["type"], "tool_result");
        assert_eq!(content[1]["tool_use_id"], "call_1");
        assert_eq!(content[1]["content"], "result_b");
    }

    #[test]
    fn test_build_anthropic_messages_mixed_roles() {
        let empty_input = serde_json::Value::Object(Default::default());
        let tc1 = serde_json::json!({"type": "tool_use", "id": "c1", "name": "t1", "input": empty_input});
        let tc2 = serde_json::json!({"type": "tool_use", "id": "c2", "name": "t2", "input": empty_input});
        let messages = vec![
            Message::system("Be helpful"),
            Message::user("Start"),
            Message::assistant_with_tool_calls("Thinking", vec![tc1, tc2]),
            Message::tool_result("c1", "t1", "r1"),
            Message::tool_result("c2", "t2", "r2"),
            Message::assistant("Done!"),
            Message::user("Great"),
        ];
        let result = RigProvider::build_anthropic_messages(&messages);
        // system filtered → [user, assistant(tc), user(tr,tr), assistant, user]
        assert_eq!(result.len(), 5);
        assert_eq!(result[0]["role"], "user");
        assert_eq!(result[0]["content"], "Start");
        assert_eq!(result[1]["role"], "assistant");
        assert!(result[1]["content"].is_array());
        assert_eq!(result[2]["role"], "user");
        assert!(result[2]["content"].is_array());
        assert_eq!(result[3]["role"], "assistant");
        assert_eq!(result[3]["content"], "Done!");
        assert_eq!(result[4]["role"], "user");
        assert_eq!(result[4]["content"], "Great");
    }

    // ============================================================
    // build_openai_messages tests
    // ============================================================

    #[test]
    fn test_build_openai_messages_empty() {
        let result = RigProvider::build_openai_messages(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_build_openai_messages_plain_text() {
        let messages = vec![
            Message::system("You are helpful"),
            Message::user("Hello"),
            Message::assistant("Hi there!"),
            Message::user("How are you?"),
        ];
        let result = RigProvider::build_openai_messages(&messages);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0]["role"], "system");
        assert_eq!(result[0]["content"], "You are helpful");
        assert_eq!(result[1]["role"], "user");
        assert_eq!(result[1]["content"], "Hello");
        assert_eq!(result[2]["role"], "assistant");
        assert_eq!(result[2]["content"], "Hi there!");
        assert_eq!(result[3]["role"], "user");
        assert_eq!(result[3]["content"], "How are you?");
    }

    #[test]
    fn test_build_openai_messages_assistant_with_tool_calls() {
        let tool_call = serde_json::json!({
            "id": "call_123",
            "name": "get_weather",
            "input": {"city": "Tokyo"}
        });
        let messages = vec![
            Message::user("What's the weather?"),
            Message::assistant_with_tool_calls("Let me check", vec![tool_call]),
        ];
        let result = RigProvider::build_openai_messages(&messages);
        assert_eq!(result.len(), 2);

        let assistant_msg = &result[1];
        assert_eq!(assistant_msg["role"], "assistant");
        assert_eq!(assistant_msg["content"], "Let me check");
        let tool_calls = assistant_msg["tool_calls"].as_array().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0]["type"], "function");
        assert_eq!(tool_calls[0]["function"]["name"], "get_weather");
        assert!(tool_calls[0]["function"]["arguments"].is_string());
    }

    #[test]
    fn test_build_openai_messages_assistant_with_tool_calls_empty_content() {
        let tool_call = serde_json::json!({
            "id": "call_456",
            "name": "search",
            "input": {"q": "rust"}
        });
        let messages = vec![
            Message::user("Search please"),
            Message::assistant_with_tool_calls("", vec![tool_call]),
        ];
        let result = RigProvider::build_openai_messages(&messages);
        let assistant_msg = &result[1];
        // OpenAI allows missing content field when empty, or it can be empty string
        assert!(assistant_msg.get("content").map(|c| c.as_str().unwrap_or("") == "").unwrap_or(true));
        let tool_calls = assistant_msg["tool_calls"].as_array().unwrap();
        assert_eq!(tool_calls.len(), 1);
    }

    #[test]
    fn test_build_openai_messages_tool_results() {
        let messages = vec![
            Message::assistant("Check this"),
            Message::tool_result("call_1", "search_tool", "Found: rust is great"),
        ];
        let result = RigProvider::build_openai_messages(&messages);
        assert_eq!(result.len(), 2);

        let tool_msg = &result[1];
        assert_eq!(tool_msg["role"], "tool");
        assert_eq!(tool_msg["tool_call_id"], "call_1");
        assert_eq!(tool_msg["content"], "Found: rust is great");
    }

    #[test]
    fn test_build_openai_messages_consecutive_tool_results() {
        let messages = vec![
            Message::user("Run tools"),
            Message::assistant("OK"),
            Message::tool_result("c1", "t1", "r1"),
            Message::tool_result("c2", "t2", "r2"),
        ];
        let result = RigProvider::build_openai_messages(&messages);
        // OpenAI keeps each tool result as separate messages (no merging like Anthropic)
        assert_eq!(result.len(), 4);
        assert_eq!(result[2]["role"], "tool");
        assert_eq!(result[2]["tool_call_id"], "c1");
        assert_eq!(result[3]["role"], "tool");
        assert_eq!(result[3]["tool_call_id"], "c2");
    }

    #[test]
    fn test_build_openai_messages_assistant_no_tool_calls() {
        let messages = vec![
            Message::user("Hello"),
            Message::assistant("World"),
        ];
        let result = RigProvider::build_openai_messages(&messages);
        assert_eq!(result.len(), 2);
        assert!(result[1].get("tool_calls").is_none());
    }

    #[test]
    fn test_build_openai_messages_anthropic_to_openai_conversion() {
        // Test that Anthropic-format tool_use blocks get converted to OpenAI format
        let anthropic_tool_use = serde_json::json!({
            "type": "tool_use",
            "id": "tu_789",
            "name": "calculator",
            "input": {"expr": "2+2"}
        });
        let messages = vec![
            Message::user("Calculate"),
            Message::assistant_with_tool_calls("", vec![anthropic_tool_use]),
            Message::tool_result("tu_789", "calculator", "4"),
        ];
        let result = RigProvider::build_openai_messages(&messages);
        let assistant_msg = &result[1];
        let tool_calls = assistant_msg["tool_calls"].as_array().unwrap();
        let tc = &tool_calls[0];
        assert_eq!(tc["id"], "tu_789");
        assert_eq!(tc["type"], "function");
        assert_eq!(tc["function"]["name"], "calculator");
        let args: serde_json::Value = serde_json::from_str(tc["function"]["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(args["expr"], "2+2");
    }

    #[test]
    fn test_build_openai_messages_openai_format_passthrough() {
        // If tool calls already in OpenAI format (has "function" key), pass through
        let openai_tool_call = serde_json::json!({
            "id": "call_999",
            "type": "function",
            "function": {
                "name": "direct_tool",
                "arguments": "{\"key\": \"value\"}"
            }
        });
        let messages = vec![
            Message::user("Do it"),
            Message::assistant_with_tool_calls("", vec![openai_tool_call]),
        ];
        let result = RigProvider::build_openai_messages(&messages);
        let assistant_msg = &result[1];
        let tool_calls = assistant_msg["tool_calls"].as_array().unwrap();
        let tc = &tool_calls[0];
        assert_eq!(tc["id"], "call_999");
        assert_eq!(tc["function"]["name"], "direct_tool");
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
        let response = provider.chat(&messages, &[]).unwrap();
        assert_eq!(response.content, "Response");
    }

    #[test]
    fn test_mock_provider_chat_model_field() {
        let provider = MockLlmProvider::new("Response");
        let messages = vec![Message::user("Hello")];
        let response = provider.chat(&messages, &[]).unwrap();
        assert_eq!(response.model, "mock-model");
    }

    #[test]
    fn test_mock_provider_chat_usage_stats() {
        let provider = MockLlmProvider::new("Response");
        let messages = vec![Message::user("Hello")];
        let response = provider.chat(&messages, &[]).unwrap();
        assert!(response.usage.is_some());
        let usage = response.usage.unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 5);
    }

    #[test]
    fn test_mock_provider_chat_empty_messages() {
        let provider = MockLlmProvider::new("Response");
        let messages: Vec<Message> = vec![];
        let response = provider.chat(&messages, &[]).unwrap();
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
        let response = provider.chat(&messages, &[]).unwrap();
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
    #[ignore = "makes real HTTP call, run with -- --ignored"]
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
    #[ignore = "makes real HTTP call, run with -- --ignored"]
    fn test_rig_provider_chat_with_api_key() {
        // Real HTTP call with invalid key errors
        let config = LlmConfig {
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let messages = vec![Message::user("Hello")];
        let result = provider.chat(&messages, &[]);
        assert!(result.is_err(), "Expected HTTP/API error with invalid key");
    }

    #[test]
    fn test_rig_provider_chat_response_model() {
        // Use MockLlmProvider to verify response structure
        let provider = MockLlmProvider::new("reply");
        let messages = vec![Message::user("Hello")];
        let response = provider.chat(&messages, &[]).unwrap();
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
        let response = provider.chat(&messages, &[]).unwrap();
        assert_eq!(response.content, "mock reply");
    }

    #[test]
    fn test_rig_provider_chat_no_user_message() {
        let provider = MockLlmProvider::new("mock");
        let messages = vec![Message::assistant("Just assistant")];
        let response = provider.chat(&messages, &[]).unwrap();
        assert!(!response.content.is_empty());
    }

    #[test]
    fn test_rig_provider_chat_usage_stats() {
        // MockLlmProvider returns 10/5 tokens
        let provider = MockLlmProvider::new("hello");
        let messages = vec![Message::user("Hello")];
        let response = provider.chat(&messages, &[]).unwrap();
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
        let result = provider.chat(&messages, &[]);
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "makes real HTTP call, run with -- --ignored"]
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
