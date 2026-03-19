//! Agent module for zcode
//!
//! This module implements the main agent loop and orchestration.

use serde_json::Value;
use std::sync::Arc;

use crate::error::Result;
use crate::llm::{LlmProvider, Message};
use crate::tools::ToolRegistry;

/// Maximum number of tool call iterations per user message
const MAX_TOOL_ITERATIONS: usize = 5;

/// Agent state
pub struct Agent {
    /// Agent name
    name: String,
    /// LLM provider
    llm: Arc<dyn LlmProvider>,
    /// Tool registry
    tools: Arc<ToolRegistry>,
    /// System prompt
    system_prompt: String,
    /// Conversation history (excluding system prompt)
    conversation: Vec<Message>,
}

impl Agent {
    /// Create a new agent
    pub fn new(
        name: impl Into<String>,
        llm: Arc<dyn LlmProvider>,
        tools: Arc<ToolRegistry>,
        system_prompt: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            llm,
            tools,
            system_prompt: system_prompt.into(),
            conversation: Vec::new(),
        }
    }

    /// Get the agent name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the conversation history length
    pub fn conversation_len(&self) -> usize {
        self.conversation.len()
    }

    /// Process a user message and return a response
    pub async fn run(&mut self, user_input: &str) -> Result<String> {
        // a. Add user message to conversation
        self.conversation.push(Message::user(user_input));

        // Build messages for LLM: system prompt + conversation history
        let mut messages = vec![Message::system(&self.system_prompt)];
        messages.extend(self.conversation.iter().cloned());

        // b + d. Call LLM, check for tool calls, repeat (max 5 iterations)
        for _ in 0..MAX_TOOL_ITERATIONS {
            let response = self.llm.chat(&messages).await?;
            let content = response.content.clone();

            // c. Parse response for tool call blocks
            if let Some((tool_name, tool_input)) = Self::parse_tool_call(&content) {
                // Tool call found: add assistant message with the tool call
                self.conversation.push(Message::assistant(&content));
                messages.push(Message::assistant(&content));

                // Execute tool
                let tool_result = match self.tools.execute(&tool_name, tool_input).await {
                    Ok(output) => serde_json::to_string_pretty(&output)
                        .unwrap_or_else(|_| output.to_string()),
                    Err(e) => format!("Tool execution failed: {}", e),
                };

                // Add tool result to conversation
                let result_msg = format!("Tool `{}` result:\n```\n{}\n```", tool_name, tool_result);
                self.conversation.push(Message::user(&result_msg));
                messages.push(Message::user(&result_msg));

                // Go to step b (continue loop)
                continue;
            }

            // e. No tool call found: return the LLM response text
            self.conversation.push(Message::assistant(&content));
            return Ok(content);
        }

        // Max iterations reached
        Ok("Maximum tool call iterations reached. Please try a simpler request.".to_string())
    }

    /// Parse a tool call from LLM response.
    ///
    /// Looks for ```json blocks containing {"tool": "...", "input": {...}}.
    pub fn parse_tool_call(response: &str) -> Option<(String, Value)> {
        let json_start = response.find("```json")?;
        let json_content_start = json_start + 7; // len("```json")
        let json_end = response[json_content_start..].find("```")?;
        let json_str = response[json_content_start..json_content_start + json_end].trim();

        let parsed: Value = serde_json::from_str(json_str).ok()?;

        let tool = parsed.get("tool")?.as_str()?.to_string();
        let input = parsed.get("input")?.clone();

        Some((tool, input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::MockLlmProvider;

    #[tokio::test]
    async fn test_agent_creation() {
        let llm = Arc::new(MockLlmProvider::new("Hello!"));
        let tools = Arc::new(ToolRegistry::new());
        let agent = Agent::new("zcode", llm, tools, "You are a helpful assistant.");

        assert_eq!(agent.name(), "zcode");
        assert_eq!(agent.conversation_len(), 0);
    }

    #[tokio::test]
    async fn test_agent_process_message() {
        let llm = Arc::new(MockLlmProvider::new("I can help with that!"));
        let tools = Arc::new(ToolRegistry::new());
        let mut agent = Agent::new("zcode", llm, tools, "You are a helpful assistant.");

        let response = agent.run("Hello").await.unwrap();
        assert_eq!(response, "I can help with that!");
        assert_eq!(agent.conversation_len(), 2); // user + assistant
    }

    #[test]
    fn test_parse_tool_call() {
        let response = r#"Here is the file:
```json
{"tool": "file_read", "input": {"path": "test.rs"}}
```
"#;
        let (tool, input) = Agent::parse_tool_call(response).unwrap();
        assert_eq!(tool, "file_read");
        assert_eq!(input["path"], "test.rs");
    }

    #[test]
    fn test_parse_tool_call_no_json() {
        let response = "Just a regular response with no tool call.";
        assert!(Agent::parse_tool_call(response).is_none());
    }

    #[test]
    fn test_parse_tool_call_invalid_json() {
        let response = "```json\nnot valid json\n```";
        assert!(Agent::parse_tool_call(response).is_none());
    }

    #[test]
    fn test_parse_tool_call_missing_fields() {
        let response = "```json\n{\"tool\": \"file_read\"}\n```";
        assert!(Agent::parse_tool_call(response).is_none());
    }

    #[tokio::test]
    async fn test_agent_with_tool() {
        let llm = Arc::new(MockLlmProvider::new(
            r#"```json
{"tool": "test", "input": {}}
```"#,
        ));
        let mut registry = ToolRegistry::new();
        registry.register_built_in_tools();
        let tools = Arc::new(registry);
        let mut agent = Agent::new("zcode", llm, tools, "You are a helpful assistant.");

        let response = agent.run("read a file").await.unwrap();
        // The mock provider returns the same response for the follow-up too,
        // so we hit max iterations since it keeps returning tool calls
        assert!(response.contains("Maximum tool call iterations"));
    }

    #[tokio::test]
    async fn test_agent_tool_error() {
        // First call returns tool call, second returns regular text
        // But MockLlmProvider returns the same thing each time,
        // so we test that tool errors are handled gracefully
        let llm = Arc::new(MockLlmProvider::new(
            r#"```json
{"tool": "nonexistent_tool", "input": {}}
```"#,
        ));
        let tools = Arc::new(ToolRegistry::new());
        let mut agent = Agent::new("zcode", llm, tools, "You are a helpful assistant.");

        // Should not panic, tool error is reported back to LLM
        let response = agent.run("do something").await.unwrap();
        // Mock returns the same tool call again, so max iterations
        assert!(response.contains("Maximum tool call iterations"));
    }
}
