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

        let env_name = self
            .config
            .api_key_env
            .as_deref()
            .unwrap_or("ZCODE_API_KEY");
        std::env::var(env_name).map_err(|_| ZcodeError::MissingApiKey(env_name.to_string()))
    }

    /// Resolve the OpenAI-compatible chat completions endpoint.
    ///
    /// The configured base URL may point at the service root, a `/v1` root, or
    /// the complete `/chat/completions` endpoint.
    fn chat_endpoint(&self) -> String {
        let base = self
            .config
            .base_url
            .clone()
            .or_else(|| std::env::var("ZCODE_BASE_URL").ok())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
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
                        if let Some(reasoning_content) = &m.reasoning_content {
                            msg["reasoning_content"] =
                                serde_json::Value::String(reasoning_content.clone());
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
        let endpoint = self.chat_endpoint();

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
        let endpoint = self.chat_endpoint();

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
mod provider_tests;
