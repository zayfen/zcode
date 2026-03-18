use crate::tools::{ToolCall, ToolResult, AgentError};
use std::collections::HashMap;

/// LLM 消息
#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
    
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
    
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }
}

/// LLM 响应
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub should_continue: bool,
}

/// LLM 提供者 trait
pub trait LlmProvider: Send + Sync {
    fn complete(&self, messages: Vec<Message>) -> Result<LlmResponse, AgentError>;
}

/// Mock LLM 提供者（用于测试）
pub struct MockLlmProvider {
    responses: HashMap<String, LlmResponse>,
}

impl MockLlmProvider {
    pub fn new() -> Self {
        let mut responses = HashMap::new();
        
        // 预设一些响应
        responses.insert(
            "read".to_string(),
            LlmResponse {
                content: "I'll read the file".to_string(),
                tool_calls: vec![],
                should_continue: false,
            },
        );
        
        Self { responses }
    }
    
    pub fn with_response(mut self, key: String, response: LlmResponse) -> Self {
        self.responses.insert(key, response);
        self
    }
}

impl Default for MockLlmProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmProvider for MockLlmProvider {
    fn complete(&self, messages: Vec<Message>) -> Result<LlmResponse, AgentError> {
        // 简单的响应逻辑
        if let Some(last_msg) = messages.last() {
            if last_msg.content.contains("read") {
                return Ok(LlmResponse {
                    content: "Reading file".to_string(),
                    tool_calls: vec![],
                    should_continue: false,
                });
            }
        }
        
        Ok(LlmResponse {
            content: "Task completed".to_string(),
            tool_calls: vec![],
            should_continue: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let user_msg = Message::user("test");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, "test");
        
        let assistant_msg = Message::assistant("response");
        assert_eq!(assistant_msg.role, "assistant");
        
        let system_msg = Message::system("instruction");
        assert_eq!(system_msg.role, "system");
    }

    #[test]
    fn test_llm_response() {
        let response = LlmResponse {
            content: "test".to_string(),
            tool_calls: vec![],
            should_continue: true,
        };
        assert!(response.should_continue);
    }

    #[test]
    fn test_mock_llm_provider() {
        let provider = MockLlmProvider::new();
        let messages = vec![Message::user("read file")];
        
        let response = provider.complete(messages).unwrap();
        assert!(response.content.contains("Reading"));
    }

    #[test]
    fn test_mock_llm_with_custom_response() {
        let custom_response = LlmResponse {
            content: "Custom response".to_string(),
            tool_calls: vec![],
            should_continue: false,
        };
        
        let provider = MockLlmProvider::new()
            .with_response("custom".to_string(), custom_response);
        
        assert!(provider.responses.contains_key("custom"));
    }

    #[test]
    fn test_llm_provider_default() {
        let provider = MockLlmProvider::default();
        let messages = vec![Message::user("test")];
        
        let response = provider.complete(messages).unwrap();
        assert!(!response.tool_calls.is_empty() || !response.content.is_empty());
    }
}
