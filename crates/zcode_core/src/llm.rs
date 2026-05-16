//! Shared LLM request/response DTOs.

/// LLM client configuration.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    pub fast_model: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub api_key_env: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "openai-compatible".to_string(),
            model: std::env::var("ZCODE_MODEL").unwrap_or_else(|_| "gpt-4o".to_string()),
            fast_model: std::env::var("ZCODE_FAST_MODEL").ok(),
            base_url: std::env::var("ZCODE_BASE_URL").ok(),
            api_key: None,
            api_key_env: None,
            temperature: 0.7,
            max_tokens: 4096,
        }
    }
}

/// Message role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A chat message.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Option<Vec<serde_json::Value>>,
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
    pub reasoning_content: Option<String>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            tool_name: None,
            reasoning_content: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            tool_name: None,
            reasoning_content: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            tool_name: None,
            reasoning_content: None,
        }
    }

    pub fn assistant_with_tool_calls(
        content: impl Into<String>,
        tool_calls: Vec<serde_json::Value>,
    ) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            tool_name: None,
            reasoning_content: None,
        }
    }

    pub fn assistant_with_tool_calls_and_reasoning(
        content: impl Into<String>,
        tool_calls: Vec<serde_json::Value>,
        reasoning_content: Option<String>,
    ) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            tool_name: None,
            reasoning_content,
        }
    }

    pub fn tool_result(
        tool_call_id: impl Into<String>,
        tool_name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            tool_name: Some(tool_name.into()),
            reasoning_content: None,
        }
    }
}

/// LLM response.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<UsageStats>,
    pub raw_response: serde_json::Value,
}

/// Usage statistics for LLM calls.
#[derive(Debug, Clone)]
pub struct UsageStats {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
