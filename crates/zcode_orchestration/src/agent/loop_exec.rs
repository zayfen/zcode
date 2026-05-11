//! Agent execution loop (ReAct pattern)
//!
//! Implements the core Reason + Act loop that powers zcode agents:
//! 1. Send conversation + available tools to LLM
//! 2. If LLM wants a tool, execute it and loop
//! 3. If LLM gives a text response, return it

use zcode_core::{Result, ZcodeError};
use zcode_capabilities::{execute_tool_calls, ToolCallRequest};
use zcode_capabilities::ToolRegistry;
pub use zcode_core::agent::ConversationMessage;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, info, warn};

// ─── LoopConfig ────────────────────────────────────────────────────────────────

/// Configuration for the agent execution loop
#[derive(Debug, Clone)]
pub struct LoopConfig {
    /// Maximum number of LLM → tool → LLM iterations
    pub max_iterations: usize,
    /// System prompt injected at the start of every conversation
    pub system_prompt: String,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            system_prompt: "You are zcode, a helpful AI coding agent. \
                Use a ReAct loop: reason about the task, call available \
                MCP/capability tools when needed, observe the results, and \
                continue until you can answer clearly."
                .to_string(),
        }
    }
}

// ─── LoopResult ────────────────────────────────────────────────────────────────

/// Result of a completed agent loop
#[derive(Debug, Clone)]
pub struct LoopResult {
    /// Final text answer from the LLM
    pub answer: String,
    /// Full conversation history
    pub history: Vec<ConversationMessage>,
    /// Number of LLM calls made
    pub llm_calls: usize,
    /// Number of tool calls executed
    pub tool_calls_executed: usize,
    /// Whether max iterations was reached
    pub hit_max_iterations: bool,
}

// ─── AgentLoop ─────────────────────────────────────────────────────────────────

/// The agent execution loop — drives agent reasoning using LLM + tools
pub struct AgentLoop {
    config: LoopConfig,
    registry: Arc<ToolRegistry>,
}

impl AgentLoop {
    /// Create a new agent loop
    pub fn new(config: LoopConfig, registry: Arc<ToolRegistry>) -> Self {
        Self { config, registry }
    }

    /// Run the ReAct loop for a user message, using the provided LLM caller.
    ///
    /// `llm_call(messages, tools)` — tools is the OpenAI-compatible schema array.
    pub async fn run<F, Fut>(
        &self,
        user_message: &str,
        seed_messages: &[ConversationMessage],
        _tool_schemas: &[Value],
        mut llm_call: F,
    ) -> Result<LoopResult>
    where
        F: FnMut(Vec<Value>, Vec<Value>) -> Fut,
        Fut: std::future::Future<Output = Result<LlmResponse>>,
    {
        let mut history: Vec<ConversationMessage> = vec![
            ConversationMessage::system(&self.config.system_prompt),
        ];

        // Inject shared context bus history from previous nodes (filter old system prompts)
        for msg in seed_messages {
            if msg.role != "system" {
                history.push(msg.clone());
            }
        }

        history.push(ConversationMessage::user(user_message));

        // Build tool schemas from registry once
        let tool_schemas = self.registry.openai_schemas();

        info!("Starting AgentLoop (max_iterations={})", self.config.max_iterations);

        let mut llm_calls = 0usize;
        let mut tool_calls_executed = 0usize;

        for i in 0..self.config.max_iterations {
            // Build messages array for LLM
            let messages = history
                .iter()
                .map(|m| serde_json::to_value(m).unwrap())
                .collect::<Vec<_>>();

            debug!("Loop iteration {}/{} - Calling LLM", i + 1, self.config.max_iterations);

            let response = llm_call(messages, tool_schemas.clone()).await?;
            llm_calls += 1;

            match response {
                LlmResponse::Text(text) => {
                    info!("Agent reached a final text answer.");
                    history.push(ConversationMessage::assistant_text(&text));
                    return Ok(LoopResult {
                        answer: text,
                        history,
                        llm_calls,
                        tool_calls_executed,
                        hit_max_iterations: false,
                    });
                }

                LlmResponse::ToolCalls(calls_json) => {
                    // Parse tool call requests (OpenAI-compatible format)
                    let requests: Vec<ToolCallRequest> = calls_json
                        .iter()
                        .filter_map(ToolCallRequest::from_openai)
                        .collect();

                    info!("Agent requested {} tool call(s).", requests.len());
                    for req in &requests {
                        let args_str = serde_json::to_string(&req.arguments).unwrap_or_default();
                        info!("  ⮑ Tool '{}' inputs: {}", req.name, args_str);
                    }

                    // Record assistant tool call message
                    history.push(ConversationMessage::assistant_tool_calls(calls_json.clone()));

                    // Execute all tool calls
                    let responses = execute_tool_calls(&self.registry, &requests);
                    tool_calls_executed += responses.len();

                    // Add tool results to history as OpenAI-compatible tool messages
                    for resp in &responses {
                        let snippet = if resp.content.len() > 1000 {
                            let safe_end = resp.content.floor_char_boundary(1000);
                            format!("{}... (truncated, total {} chars)", &resp.content[..safe_end], resp.content.len())
                        } else {
                            resp.content.clone()
                        };

                        if resp.success {
                            info!("  ✅ Tool '{}' succeeded. Result:\n{}", resp.name, snippet);
                        } else {
                            warn!("  ❌ Tool '{}' failed. Error:\n{}", resp.name, snippet);
                        }
                        history.push(ConversationMessage::tool_result(
                            resp.tool_call_id.clone(),
                            resp.name.clone(),
                            resp.content.clone(),
                        ));
                    }
                }
            }
        }

        // Hit max iterations — return partial result
        warn!("Maximum iterations ({}) reached without a final answer.", self.config.max_iterations);
        Ok(LoopResult {
            answer: "Maximum iterations reached without a final answer.".to_string(),
            history,
            llm_calls,
            tool_calls_executed,
            hit_max_iterations: true,
        })
    }
}

// ─── LlmResponse ───────────────────────────────────────────────────────────────

/// What the LLM returned for a conversation turn
#[derive(Debug, Clone)]
pub enum LlmResponse {
    /// A plain text answer
    Text(String),
    /// One or more tool calls to execute
    ToolCalls(Vec<Value>),
}

impl LlmResponse {
    /// Parse from an OpenAI-compatible response JSON
    pub fn from_openai_response(body: &Value) -> Result<Self> {
        let choice = body
            .get("choices")
            .and_then(|c| c.get(0))
            .ok_or_else(|| ZcodeError::LlmApiError("No choices in response".into()))?;

        let message = choice
            .get("message")
            .ok_or_else(|| ZcodeError::LlmApiError("No message in choice".into()))?;

        // Check for tool calls first
        if let Some(tool_calls) = message.get("tool_calls") {
            if let Some(arr) = tool_calls.as_array() {
                if !arr.is_empty() {
                    return Ok(LlmResponse::ToolCalls(arr.clone()));
                }
            }
        }

        // Fall back to text content
        let content = message
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        Ok(LlmResponse::Text(content))
    }

}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use zcode_capabilities::{Tool, ToolCallResponse, ToolRegistry, ToolResult};

    struct AddTool;
    impl Tool for AddTool {
        fn name(&self) -> &str { "add" }
        fn description(&self) -> &str { "Add two numbers" }
        fn execute(&self, input: Value) -> ToolResult<Value> {
            let a = input.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = input.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(json!({ "result": a + b }))
        }
    }

    fn make_registry() -> Arc<ToolRegistry> {
        let mut r = ToolRegistry::new();
        r.register(AddTool);
        Arc::new(r)
    }

    #[test]
    fn test_conversation_message_system() {
        let msg = ConversationMessage::system("You are a bot");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, Some("You are a bot".to_string()));
        assert!(msg.tool_calls.is_none());
    }

    #[test]
    fn test_conversation_message_user() {
        let msg = ConversationMessage::user("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_conversation_message_assistant_text() {
        let msg = ConversationMessage::assistant_text("I'm here");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, Some("I'm here".to_string()));
        assert!(msg.tool_calls.is_none());
    }

    #[test]
    fn test_conversation_message_tool_result() {
        let resp = ToolCallResponse {
            tool_call_id: "call-1".into(),
            name: "add".into(),
            content: "{\"result\": 42}".into(),
            success: true,
        };
        let msg = ConversationMessage::tool_result(
            resp.tool_call_id.clone(),
            resp.name.clone(),
            resp.content.clone(),
        );
        assert_eq!(msg.role, "tool");
        assert_eq!(msg.tool_call_id, Some("call-1".to_string()));
    }

    #[test]
    fn test_llm_response_from_text_response() {
        let body = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello!"
                }
            }]
        });
        let resp = LlmResponse::from_openai_response(&body).unwrap();
        match resp {
            LlmResponse::Text(t) => assert_eq!(t, "Hello!"),
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn test_llm_response_from_tool_call_response() {
        let body = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call-1",
                        "type": "function",
                        "function": {
                            "name": "add",
                            "arguments": "{\"a\": 1, \"b\": 2}"
                        }
                    }]
                }
            }]
        });
        let resp = LlmResponse::from_openai_response(&body).unwrap();
        match resp {
            LlmResponse::ToolCalls(calls) => assert_eq!(calls.len(), 1),
            _ => panic!("Expected ToolCalls"),
        }
    }

    #[test]
    fn test_llm_response_no_choices_error() {
        let body = json!({ "error": "bad request" });
        assert!(LlmResponse::from_openai_response(&body).is_err());
    }

    #[tokio::test]
    async fn test_agent_loop_text_response() {
        let registry = make_registry();
        let config = LoopConfig::default();
        let agent_loop = AgentLoop::new(config, registry);

        // Mock LLM that always returns a text answer
        let result = agent_loop.run(
            "What is 2+2?",
            &[],
            &[],
            |_messages: Vec<serde_json::Value>, _tools: Vec<serde_json::Value>| async { Ok(LlmResponse::Text("The answer is 4.".to_string())) },
        ).await.unwrap();

        assert_eq!(result.answer, "The answer is 4.");
        assert_eq!(result.llm_calls, 1);
        assert_eq!(result.tool_calls_executed, 0);
        assert!(!result.hit_max_iterations);
    }

    #[tokio::test]
    async fn test_agent_loop_with_tool_call() {
        let registry = make_registry();
        let config = LoopConfig {
            max_iterations: 5,
            ..Default::default()
        };
        let agent_loop = AgentLoop::new(config, registry);

        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let result = agent_loop.run(
            "Can you add 3 and 4?",
            &[],
            &[],
            |_msgs: Vec<serde_json::Value>, _tools: Vec<serde_json::Value>| {
                let n = call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                async move {
                    if n == 1 {
                        Ok(LlmResponse::ToolCalls(vec![json!({
                            "id": "call-1",
                            "type": "function",
                            "function": {
                                "name": "add",
                                "arguments": "{\"a\":3,\"b\":4}"
                            }
                        })]))
                    } else {
                        Ok(LlmResponse::Text("3 + 4 = 7".to_string()))
                    }
                }
            },
        ).await.unwrap();

        assert_eq!(result.llm_calls, 2);
        assert_eq!(result.tool_calls_executed, 1);
        assert_eq!(result.answer, "3 + 4 = 7");
    }

    #[tokio::test]
    async fn test_agent_loop_max_iterations() {
        let registry = Arc::new(ToolRegistry::new());
        let config = LoopConfig {
            max_iterations: 2,
            ..Default::default()
        };
        let agent_loop = AgentLoop::new(config, registry);

        // Mock that always requests tools (never returns text)
        let result = agent_loop.run(
            "Loop forever",
            &[],
            &[],
            |_: Vec<serde_json::Value>, _: Vec<serde_json::Value>| async {
                Ok(LlmResponse::ToolCalls(vec![json!({
                    "id": "call-x",
                    "type": "function",
                    "function": { "name": "nonexistent", "arguments": "{}" }
                })]))
            },
        ).await.unwrap();

        assert!(result.hit_max_iterations);
        assert_eq!(result.llm_calls, 2);
    }

    #[test]
    fn test_loop_config_default() {
        let cfg = LoopConfig::default();
        assert_eq!(cfg.max_iterations, 20);
        assert!(!cfg.system_prompt.is_empty());
    }
}
