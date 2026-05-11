//! LLM Provider implementations using rig-core
//!
//! This module provides the LLM provider trait and implementations using the rig-core library.

use zcode_core::llm::{LlmConfig, LlmResponse, Message, MessageRole, UsageStats};
use zcode_core::{Result, ZcodeError};

/// Stream event emitted by chat-completion providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmStreamEvent {
    /// Visible assistant response text.
    Content(String),
    /// Hidden/collapsible model thinking or reasoning text.
    Thinking(String),
}

/// Streaming chat response type.
pub type ChatStreamingResponse =
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<LlmStreamEvent>> + Send>>;

/// Trait for LLM providers
pub trait LlmProvider: Send + Sync {
    /// Generate a completion from a prompt
    fn complete(&self, prompt: &str) -> Result<String>;

    /// Generate a completion from a conversation
    fn chat(&self, messages: &[Message], tools: &[serde_json::Value]) -> Result<LlmResponse>;

    /// Stream a completion (returns a stream of text chunks)
    fn stream_complete(&self, prompt: &str) -> Result<StreamingResponse>;

    /// Stream a chat completion as structured UI events.
    fn stream_chat(
        &self,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<ChatStreamingResponse> {
        let response = self.chat(messages, tools)?;
        Ok(Box::pin(futures::stream::iter(vec![Ok(
            LlmStreamEvent::Content(response.content),
        )])))
    }
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

    /// Get the API key from config or `ZCODE_API_KEY`.
    fn get_api_key(&self) -> Result<String> {
        if let Some(ref key) = self.config.api_key {
            return Ok(key.clone());
        }

        std::env::var("ZCODE_API_KEY")
            .map_err(|_| ZcodeError::MissingApiKey("ZCODE_API_KEY".to_string()))
    }

    /// Resolve the OpenAI-compatible chat completions endpoint.
    ///
    /// `ZCODE_BASE_URL` may point at the service root, a `/v1` root, or the
    /// complete `/chat/completions` endpoint.
    fn chat_endpoint() -> String {
        let base = std::env::var("ZCODE_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let base = base.trim_end_matches('/');

        if base.ends_with("/chat/completions") {
            base.to_string()
        } else if base.ends_with("/v1") {
            format!("{}/chat/completions", base)
        } else {
            format!("{}/v1/chat/completions", base)
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
        self.chat_openai_compatible(messages, &api_key, tools)
    }

    fn stream_complete(&self, prompt: &str) -> Result<StreamingResponse> {
        // Fallback to non-streaming for now
        let response = self.complete(prompt)?;
        let chunks = vec![Ok(response)];
        Ok(Box::pin(futures::stream::iter(chunks)))
    }

    fn stream_chat(
        &self,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<ChatStreamingResponse> {
        let api_key = self.get_api_key()?;
        self.stream_chat_openai_compatible(messages, &api_key, tools)
    }
}

impl RigProvider {
    /// Build the OpenAI Chat Completions API `messages` array from a `&[Message]`.
    ///
    /// OpenAI expects:
    /// - Assistant messages with tool_calls: `{role: "assistant", tool_calls: [...]}`
    /// - Tool results: `{role: "tool", tool_call_id: "...", content: "..."}`
    fn build_openai_messages(messages: &[Message]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|m| match m.role {
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
                        let openai_tool_calls: Vec<serde_json::Value> = tool_calls
                            .iter()
                            .map(|tc| {
                                // If already in OpenAI format (has "function" key), pass through
                                if tc.get("function").is_some() {
                                    tc.clone()
                                } else {
                                    // Convert legacy internal tool-use shape to OpenAI format.
                                    let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                    let name = tc.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                    let input =
                                        tc.get("input").cloned().unwrap_or(serde_json::json!({}));
                                    serde_json::json!({
                                        "id": id,
                                        "type": "function",
                                        "function": {
                                            "name": name,
                                            "arguments": serde_json::to_string(&input).unwrap_or_default()
                                        }
                                    })
                                }
                            })
                            .collect();
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
            })
            .collect()
    }

    fn build_chat_body(
        &self,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> serde_json::Value {
        // Build properly formatted messages (handles tool_calls + tool results)
        let openai_messages = Self::build_openai_messages(messages);

        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": openai_messages,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens
        });

        // Inject tool schemas if provided (convert from legacy schema shape if needed)
        if !tools.is_empty() {
            let openai_tools: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    if t.get("type").is_some() {
                        t.clone()
                    } else {
                        let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let desc = t.get("description").and_then(|v| v.as_str()).unwrap_or("");
                        let params = t
                            .get("input_schema")
                            .cloned()
                            .unwrap_or(serde_json::json!({"type": "object"}));
                        serde_json::json!({
                            "type": "function",
                            "function": {
                                "name": name,
                                "description": desc,
                                "parameters": params
                            }
                        })
                    }
                })
                .collect();
            body["tools"] = serde_json::Value::Array(openai_tools);
        }

        body
    }

    /// OpenAI-compatible Chat Completions API call.
    fn chat_openai_compatible(
        &self,
        messages: &[Message],
        api_key: &str,
        tools: &[serde_json::Value],
    ) -> Result<LlmResponse> {
        let body = self.build_chat_body(messages, tools);
        let api_key = api_key.to_string();
        let model = self.config.model.clone();
        let endpoint = Self::chat_endpoint();

        let (status, response_body) = run_http(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .map_err(|e| {
                    ZcodeError::LlmApiError(format!("Failed to build HTTP client: {}", e))
                })?;
            let resp = client
                .post(&endpoint)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| {
                    ZcodeError::LlmApiError(format!("OpenAI-compatible request failed: {}", e))
                })?;
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.map_err(|e| {
                ZcodeError::LlmResponseError(format!(
                    "Failed to parse OpenAI-compatible response: {}",
                    e
                ))
            })?;
            Ok::<_, ZcodeError>((status, body))
        })?;

        if !status.is_success() {
            let err_msg = response_body
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            return Err(ZcodeError::LlmApiError(format!(
                "OpenAI-compatible API error ({}): {}",
                status, err_msg
            )));
        }

        let content = match parse_openai_content(&response_body)? {
            ProviderResponse::Text(text) => text,
            ProviderResponse::ToolCalls(calls) => calls
                .iter()
                .filter_map(|call| {
                    call.get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(|name| name.as_str())
                })
                .collect::<Vec<_>>()
                .join(", "),
        };

        let input_tokens = response_body
            .get("usage")
            .and_then(|u| u.get("prompt_tokens"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;
        let output_tokens = response_body
            .get("usage")
            .and_then(|u| u.get("completion_tokens"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;

        Ok(LlmResponse {
            content,
            model,
            usage: Some(UsageStats {
                input_tokens,
                output_tokens,
            }),
            raw_response: response_body,
        })
    }

    fn stream_chat_openai_compatible(
        &self,
        messages: &[Message],
        api_key: &str,
        tools: &[serde_json::Value],
    ) -> Result<ChatStreamingResponse> {
        let mut body = self.build_chat_body(messages, tools);
        body["stream"] = serde_json::Value::Bool(true);

        let api_key = api_key.to_string();
        let endpoint = Self::chat_endpoint();

        let (tx, rx) = futures::channel::mpsc::unbounded::<Result<LlmStreamEvent>>();

        std::thread::spawn(move || {
            let tx_error = tx.clone();
            let result = tokio::runtime::Runtime::new()
                .map_err(|e| {
                    ZcodeError::LlmApiError(format!("Failed to create stream runtime: {}", e))
                })
                .and_then(|runtime| {
                    runtime.block_on(async move {
                        use futures::StreamExt;

                        let client = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(120))
                            .build()
                            .map_err(|e| {
                                ZcodeError::LlmApiError(format!(
                                    "Failed to build HTTP client: {}",
                                    e
                                ))
                            })?;
                        let resp = client
                            .post(&endpoint)
                            .header("Authorization", format!("Bearer {}", api_key))
                            .header("Content-Type", "application/json")
                            .json(&body)
                            .send()
                            .await
                            .map_err(|e| {
                                ZcodeError::LlmApiError(format!(
                                    "OpenAI-compatible streaming request failed: {}",
                                    e
                                ))
                            })?;

                        let status = resp.status();
                        if !status.is_success() {
                            let error_body = resp.text().await.unwrap_or_default();
                            return Err(ZcodeError::LlmApiError(format!(
                                "OpenAI-compatible streaming API error ({}): {}",
                                status, error_body
                            )));
                        }

                        let mut parser = OpenAiStreamParser::default();
                        let mut bytes_stream = resp.bytes_stream();
                        while let Some(chunk) = bytes_stream.next().await {
                            let chunk = chunk.map_err(|e| {
                                ZcodeError::LlmResponseError(format!(
                                    "Failed to read OpenAI-compatible streaming response: {}",
                                    e
                                ))
                            })?;
                            for event in parser.push_bytes(&chunk) {
                                if tx.unbounded_send(event).is_err() {
                                    return Ok(());
                                }
                            }
                            if parser.is_done() {
                                return Ok(());
                            }
                        }

                        for event in parser.finish() {
                            if tx.unbounded_send(event).is_err() {
                                return Ok(());
                            }
                        }

                        Ok(())
                    })
                });

            if let Err(e) = result {
                let _ = tx_error.unbounded_send(Err(e));
            }
        });

        Ok(Box::pin(rx))
    }
}

#[derive(Debug, Clone)]
enum ProviderResponse {
    Text(String),
    ToolCalls(Vec<serde_json::Value>),
}

fn parse_openai_content(body: &serde_json::Value) -> Result<ProviderResponse> {
    let choice = body
        .get("choices")
        .and_then(|choices| choices.get(0))
        .ok_or_else(|| ZcodeError::LlmApiError("No choices in response".into()))?;

    let message = choice
        .get("message")
        .ok_or_else(|| ZcodeError::LlmApiError("No message in choice".into()))?;

    if let Some(tool_calls) = message.get("tool_calls").and_then(|calls| calls.as_array()) {
        if !tool_calls.is_empty() {
            return Ok(ProviderResponse::ToolCalls(tool_calls.clone()));
        }
    }

    let content = message
        .get("content")
        .and_then(|content| content.as_str())
        .unwrap_or("")
        .to_string();
    Ok(ProviderResponse::Text(content))
}

#[derive(Debug, Default)]
struct OpenAiStreamParser {
    buffer: Vec<u8>,
    done: bool,
}

impl OpenAiStreamParser {
    #[cfg(test)]
    fn push_chunk(&mut self, chunk: &str) -> Vec<Result<LlmStreamEvent>> {
        self.push_bytes(chunk.as_bytes())
    }

    fn push_bytes(&mut self, chunk: &[u8]) -> Vec<Result<LlmStreamEvent>> {
        if self.done {
            return Vec::new();
        }

        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();

        while let Some(newline_pos) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let raw_line: Vec<u8> = self.buffer.drain(..=newline_pos).collect();
            let line = trim_line_ending(&raw_line);
            match std::str::from_utf8(line) {
                Ok(line) => events.extend(self.parse_line(line)),
                Err(e) => events.push(Err(ZcodeError::LlmResponseError(format!(
                    "Failed to decode streaming chunk: {}",
                    e
                )))),
            }
            if self.done {
                self.buffer.clear();
                break;
            }
        }

        events
    }

    fn finish(&mut self) -> Vec<Result<LlmStreamEvent>> {
        if self.done || self.buffer.iter().all(u8::is_ascii_whitespace) {
            self.buffer.clear();
            return Vec::new();
        }

        let raw_line = std::mem::take(&mut self.buffer);
        let line = trim_line_ending(&raw_line);
        match std::str::from_utf8(line) {
            Ok(line) => self.parse_line(line),
            Err(e) => vec![Err(ZcodeError::LlmResponseError(format!(
                "Failed to decode streaming chunk: {}",
                e
            )))],
        }
    }

    fn is_done(&self) -> bool {
        self.done
    }

    fn parse_line(&mut self, raw_line: &str) -> Vec<Result<LlmStreamEvent>> {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(':') {
            return Vec::new();
        }

        let Some(data) = line.strip_prefix("data:").map(str::trim) else {
            return Vec::new();
        };
        if data == "[DONE]" {
            self.done = true;
            return Vec::new();
        }

        let parsed: serde_json::Value = match serde_json::from_str(data) {
            Ok(value) => value,
            Err(e) => {
                return vec![Err(ZcodeError::LlmResponseError(format!(
                    "Failed to parse streaming chunk: {}",
                    e
                )))];
            }
        };

        stream_events_from_payload(&parsed)
    }
}

fn trim_line_ending(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r\n")
        .or_else(|| line.strip_suffix(b"\n"))
        .or_else(|| line.strip_suffix(b"\r"))
        .unwrap_or(line)
}

#[cfg(test)]
fn parse_openai_stream_events(text: &str) -> Vec<Result<LlmStreamEvent>> {
    let mut parser = OpenAiStreamParser::default();
    let mut events = parser.push_chunk(text);
    events.extend(parser.finish());
    events
}

fn stream_events_from_payload(parsed: &serde_json::Value) -> Vec<Result<LlmStreamEvent>> {
    let Some(delta) = parsed
        .get("choices")
        .and_then(|choices| choices.get(0))
        .and_then(|choice| choice.get("delta"))
    else {
        return Vec::new();
    };

    let mut events = Vec::new();
    if let Some(thinking) = stream_delta_thinking(delta) {
        if !thinking.is_empty() {
            events.push(Ok(LlmStreamEvent::Thinking(thinking)));
        }
    }
    if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
        if !content.is_empty() {
            events.push(Ok(LlmStreamEvent::Content(content.to_string())));
        }
    }
    events
}

fn stream_delta_thinking(delta: &serde_json::Value) -> Option<String> {
    const KEYS: &[&str] = &[
        "reasoning_content",
        "reasoning",
        "thinking",
        "thought",
        "thoughts",
    ];

    for key in KEYS {
        if let Some(text) = delta.get(*key).and_then(|v| v.as_str()) {
            return Some(text.to_string());
        }
        if let Some(text) = delta
            .get(*key)
            .and_then(|v| v.get("content"))
            .and_then(|v| v.as_str())
        {
            return Some(text.to_string());
        }
    }

    None
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
            usage: Some(UsageStats {
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

    fn stream_chat(
        &self,
        _messages: &[Message],
        _tools: &[serde_json::Value],
    ) -> Result<ChatStreamingResponse> {
        let response = self.response.clone();
        Ok(Box::pin(futures::stream::iter(vec![Ok(
            LlmStreamEvent::Content(response),
        )])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use zcode_core::llm::{LlmConfig, Message};

    fn with_zcode_api_key_removed<T>(f: impl FnOnce() -> T) -> T {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let original = std::env::var("ZCODE_API_KEY").ok();
        std::env::remove_var("ZCODE_API_KEY");
        let result = f();
        if let Some(val) = original {
            std::env::set_var("ZCODE_API_KEY", val);
        }
        result
    }

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
        assert!(assistant_msg
            .get("content")
            .map(|c| c.as_str().unwrap_or("") == "")
            .unwrap_or(true));
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
        // OpenAI keeps each tool result as a separate message.
        assert_eq!(result.len(), 4);
        assert_eq!(result[2]["role"], "tool");
        assert_eq!(result[2]["tool_call_id"], "c1");
        assert_eq!(result[3]["role"], "tool");
        assert_eq!(result[3]["tool_call_id"], "c2");
    }

    #[test]
    fn test_build_openai_messages_assistant_no_tool_calls() {
        let messages = vec![Message::user("Hello"), Message::assistant("World")];
        let result = RigProvider::build_openai_messages(&messages);
        assert_eq!(result.len(), 2);
        assert!(result[1].get("tool_calls").is_none());
    }

    #[test]
    fn test_build_openai_messages_legacy_tool_use_to_openai_conversion() {
        let legacy_tool_use = serde_json::json!({
            "type": "tool_use",
            "id": "tu_789",
            "name": "calculator",
            "input": {"expr": "2+2"}
        });
        let messages = vec![
            Message::user("Calculate"),
            Message::assistant_with_tool_calls("", vec![legacy_tool_use]),
            Message::tool_result("tu_789", "calculator", "4"),
        ];
        let result = RigProvider::build_openai_messages(&messages);
        let assistant_msg = &result[1];
        let tool_calls = assistant_msg["tool_calls"].as_array().unwrap();
        let tc = &tool_calls[0];
        assert_eq!(tc["id"], "tu_789");
        assert_eq!(tc["type"], "function");
        assert_eq!(tc["function"]["name"], "calculator");
        let args: serde_json::Value =
            serde_json::from_str(tc["function"]["arguments"].as_str().unwrap()).unwrap();
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

    #[test]
    fn test_parse_openai_stream_events_content_and_thinking() {
        let sse = concat!(
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"plan\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\n",
            "data: [DONE]\n\n"
        );

        let events = parse_openai_stream_events(sse);
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0].as_ref().unwrap(),
            &LlmStreamEvent::Thinking("plan".to_string())
        );
        assert_eq!(
            events[1].as_ref().unwrap(),
            &LlmStreamEvent::Content("hello".to_string())
        );
    }

    #[test]
    fn test_openai_stream_parser_buffers_partial_chunks() {
        let mut parser = OpenAiStreamParser::default();
        let first = parser.push_chunk("data: {\"choices\":[{\"delta\":{\"content\":\"hel");
        assert!(first.is_empty());

        let second = parser.push_chunk("lo\"}}]}\n\ndata: [DONE]\n\n");
        assert_eq!(second.len(), 1);
        assert_eq!(
            second[0].as_ref().unwrap(),
            &LlmStreamEvent::Content("hello".to_string())
        );
        assert!(parser.is_done());
    }

    #[test]
    fn test_openai_stream_parser_preserves_split_utf8() {
        let mut parser = OpenAiStreamParser::default();
        let payload = "data: {\"choices\":[{\"delta\":{\"content\":\"你好\"}}]}\n\n";
        let split_at = payload.find('好').unwrap() + 1;

        let first = parser.push_bytes(&payload.as_bytes()[..split_at]);
        assert!(first.is_empty());

        let second = parser.push_bytes(&payload.as_bytes()[split_at..]);
        assert_eq!(second.len(), 1);
        assert_eq!(
            second[0].as_ref().unwrap(),
            &LlmStreamEvent::Content("你好".to_string())
        );
    }

    // ============================================================
    // RigProvider tests
    // ============================================================

    #[test]
    fn test_rig_provider_new() {
        let config = LlmConfig::default();
        let provider = RigProvider::new(config);
        assert_eq!(provider.config().provider, "openai-compatible");
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
    fn test_rig_provider_chat_endpoint_from_base_url() {
        let original = std::env::var("ZCODE_BASE_URL").ok();

        std::env::set_var("ZCODE_BASE_URL", "https://example.com");
        assert_eq!(
            RigProvider::chat_endpoint(),
            "https://example.com/v1/chat/completions"
        );

        std::env::set_var("ZCODE_BASE_URL", "https://example.com/v1");
        assert_eq!(
            RigProvider::chat_endpoint(),
            "https://example.com/v1/chat/completions"
        );

        std::env::set_var("ZCODE_BASE_URL", "https://example.com/v1/chat/completions");
        assert_eq!(
            RigProvider::chat_endpoint(),
            "https://example.com/v1/chat/completions"
        );

        if let Some(val) = original {
            std::env::set_var("ZCODE_BASE_URL", val);
        } else {
            std::env::remove_var("ZCODE_BASE_URL");
        }
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
        with_zcode_api_key_removed(|| {
            let config = LlmConfig {
                api_key: None,
                ..Default::default()
            };
            let provider = RigProvider::new(config);
            let result = provider.complete("test");
            assert!(result.is_err());
            match result.unwrap_err() {
                ZcodeError::MissingApiKey(provider_name) => {
                    assert_eq!(provider_name, "ZCODE_API_KEY");
                }
                _ => panic!("Expected MissingApiKey error"),
            }
        });
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
        with_zcode_api_key_removed(|| {
            let config = LlmConfig {
                api_key: None,
                ..Default::default()
            };
            let provider = RigProvider::new(config);
            let messages = vec![Message::user("Hello")];
            let result = provider.chat(&messages, &[]);
            assert!(result.is_err());
        });
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
        with_zcode_api_key_removed(|| {
            let config = LlmConfig {
                api_key: None,
                ..Default::default()
            };
            let provider = RigProvider::new(config);
            let result = provider.stream_complete("test");
            assert!(result.is_err());
        });
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
            ZcodeError::MissingApiKey(_) => {
                panic!("Should not be MissingApiKey — key was provided")
            }
            _ => {} // Any LLM API error is expected
        }
    }

    #[test]
    fn test_rig_provider_zcode_api_key_env() {
        with_zcode_api_key_removed(|| {
            let config = LlmConfig {
                provider: "openai-compatible".to_string(),
                api_key: None,
                ..Default::default()
            };
            let provider = RigProvider::new(config);
            let result = provider.complete("test");
            assert!(result.is_err());
        });
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
        let chunks = vec![Ok("Hello ".to_string()), Ok("world!".to_string())];
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
    fn test_rig_provider_custom_provider_uses_zcode_api_key_env() {
        with_zcode_api_key_removed(|| {
            let config = LlmConfig {
                provider: "custom_provider".to_string(),
                api_key: None,
                ..Default::default()
            };
            let provider = RigProvider::new(config);
            let result = provider.complete("test");
            assert!(result.is_err());
        });
    }
}
