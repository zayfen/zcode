//! Coder Agent
//!
//! Executes coding tasks through the shared ReAct loop. The coder reasons,
//! calls available MCP/capability tools, observes results, and repeats until it
//! can return a final implementation report.

use crate::agent::loop_exec::{AgentLoop, LoopConfig};
use crate::agent::traits::AgentTrait;
use crate::agent::types::{AgentId, AgentMessage, AgentState, AgentType, Task, TaskResult};
use zcode_core::Result;
use zcode_capabilities::ToolRegistry;
use async_trait::async_trait;
use std::sync::Arc;

/// Agent that executes code modification tasks using tools + LLM
pub struct CoderAgent {
    id: AgentId,
    state: AgentState,
    registry: Arc<ToolRegistry>,
    loop_config: LoopConfig,
}

impl CoderAgent {
    /// Create a new Coder agent
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self {
            id: AgentId::new(),
            state: AgentState::Idle,
            registry,
            loop_config: LoopConfig {
                system_prompt: "You are a senior software engineer. \
                    You must use a ReAct workflow: reason about the next step, \
                    call available MCP/capability tools when you need project facts \
                    or workspace changes, observe the tool result, then continue. \
                    Do not fabricate tool results. Always verify your changes before \
                    reporting completion."
                    .to_string(),
                ..Default::default()
            },
        }
    }

    /// Set a specific system prompt override
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.loop_config.system_prompt = prompt.into();
        self
    }

    fn transition(&mut self, next: AgentState) {
        if self.state.can_transition_to(next) {
            self.state = next;
        }
    }

    fn is_simple_task(task: &Task) -> bool {
        if task
            .context
            .get("is_simple")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
        {
            return true;
        }

        let desc = task.description.to_lowercase();
        let simple_markers = [
            "typo",
            "rename",
            "comment",
            "docs",
            "small",
            "single function",
            "简单",
            "修复拼写",
            "文档",
        ];
        simple_markers.iter().any(|marker| desc.contains(marker))
    }
}

#[async_trait]
impl AgentTrait for CoderAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Coder
    }

    fn state(&self) -> AgentState {
        self.state
    }

    async fn handle(&mut self, message: AgentMessage) -> Result<Option<AgentMessage>> {
        match message {
            AgentMessage::TaskAssigned { task, .. } => {
                self.transition(AgentState::Executing);
                let result = self.execute_task(&task).await;
                self.transition(if result.success { AgentState::Completed } else { AgentState::Failed });

                Ok(Some(AgentMessage::TaskCompleted {
                    agent: self.id.clone(),
                    result,
                }))
            }
            _ => Ok(None),
        }
    }

    async fn reset(&mut self) -> Result<()> {
        self.state = AgentState::Idle;
        Ok(())
    }
}

impl CoderAgent {
    /// Execute a task using the agent loop.
    ///
    /// If `provider` is supplied it will be used for LLM calls. Otherwise the agent
    /// will try to build a `RigProvider` from environment variables, falling back to
    /// an offline stub if no API key is found (so unit tests always work without
    /// network access).
    pub async fn execute_task(&self, task: &Task) -> TaskResult {
        self.execute_task_with(task, None).await
    }

     /// Execute a task using an explicit LLM provider (for testing / injection).
    pub async fn execute_task_with(
        &self,
        task: &Task,
        provider: Option<std::sync::Arc<dyn zcode_llm_provider::provider::LlmProvider>>,
    ) -> TaskResult {
        use zcode_llm_provider::provider::{LlmProvider, MockLlmProvider, RigProvider};
        use zcode_llm_provider::{LlmConfig, Message};
        use std::sync::Arc;

        let agent_loop = AgentLoop::new(self.loop_config.clone(), self.registry.clone());

        // Determine the LLM provider: injected > env-based RigProvider > offline stub
        let effective_provider: Arc<dyn LlmProvider> = if let Some(p) = provider {
            p
        } else if std::env::var("ZCODE_API_KEY").is_ok() {
            let model = if Self::is_simple_task(task) {
                std::env::var("ZCODE_FAST_MODEL")
                    .or_else(|_| std::env::var("ZCODE_MODEL"))
                    .unwrap_or_else(|_| "gpt-4o".to_string())
            } else {
                std::env::var("ZCODE_MODEL").unwrap_or_else(|_| "gpt-4o".to_string())
            };
            let llm_config = LlmConfig {
                provider: "openai-compatible".to_string(),
                model,
                fast_model: std::env::var("ZCODE_FAST_MODEL").ok(),
                ..Default::default()
            };
            Arc::new(RigProvider::new(llm_config))
        } else {
            Arc::new(MockLlmProvider::new(
                "Task acknowledged. Set ZCODE_API_KEY for real LLM responses.",
            ))
        };

        let result: zcode_core::Result<crate::agent::loop_exec::LoopResult> = agent_loop.run(
            &task.description,
            &[],
            &[],
            move |messages, tools| {
                let p = Arc::clone(&effective_provider);
                async move {
                    let llm_messages: Vec<Message> = messages.iter()
                        .filter_map(|v| {
                            let role_str = v.get("role")?.as_str()?;
                            match role_str {
                                "system" => {
                                    let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                                    Some(Message::system(content))
                                }
                                "assistant" => {
                                    if let Some(tool_calls) = v.get("tool_calls").and_then(|tc| tc.as_array()) {
                                        if !tool_calls.is_empty() {
                                            let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                                            Some(Message::assistant_with_tool_calls(content, tool_calls.clone()))
                                        } else {
                                            let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                                            Some(Message::assistant(content))
                                        }
                                    } else {
                                        let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                                        Some(Message::assistant(content))
                                    }
                                }
                                "tool" => {
                                    let tool_call_id = v.get("tool_call_id").and_then(|id| id.as_str()).unwrap_or("").to_string();
                                    let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                                    let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                                    Some(Message::tool_result(tool_call_id, name, content))
                                }
                                _ => {
                                    let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                                    Some(Message::user(content))
                                }
                            }
                        })
                        .collect();

                    match p.chat(&llm_messages, &tools) {
                        Ok(resp) => {
                            use crate::agent::loop_exec::LlmResponse as AgentLlmResponse;
                            if let Ok(agent_resp) = AgentLlmResponse::from_openai_response(&resp.raw_response) {
                                Ok(agent_resp)
                            } else {
                                Ok(AgentLlmResponse::Text(resp.content))
                            }
                        }
                        Err(e) => Err(e),
                    }
                }
            },
        ).await;

        match result {
            Ok(loop_result) => {
                let mut task_result = TaskResult::success(&task.id, loop_result.answer);
                task_result.llm_calls = loop_result.llm_calls;
                task_result.tool_calls = loop_result.tool_calls_executed;
                task_result
            }
            Err(e) => TaskResult::failure(&task.id, e.to_string()),
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use zcode_capabilities::{Tool, ToolResult};
    use zcode_core::{LlmResponse, UsageStats};

    fn make_coder() -> CoderAgent {
        CoderAgent::new(Arc::new(ToolRegistry::new()))
    }

    struct EchoTool;

    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }

        fn description(&self) -> &str {
            "Echoes the provided text"
        }

        fn execute(&self, input: serde_json::Value) -> ToolResult<serde_json::Value> {
            Ok(json!({
                "echo": input.get("text").and_then(|v| v.as_str()).unwrap_or("")
            }))
        }
    }

    struct ReActProvider {
        calls: AtomicUsize,
    }

    impl ReActProvider {
        fn new() -> Self {
            Self {
                calls: AtomicUsize::new(0),
            }
        }
    }

    impl zcode_llm_provider::provider::LlmProvider for ReActProvider {
        fn complete(&self, _prompt: &str) -> zcode_core::Result<String> {
            Ok("done".to_string())
        }

        fn chat(
            &self,
            _messages: &[zcode_llm_provider::Message],
            _tools: &[serde_json::Value],
        ) -> zcode_core::Result<LlmResponse> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            let raw_response = if call == 0 {
                json!({
                    "choices": [{
                        "message": {
                            "role": "assistant",
                            "content": null,
                            "tool_calls": [{
                                "id": "call-echo",
                                "type": "function",
                                "function": {
                                    "name": "echo",
                                    "arguments": "{\"text\":\"inspect first\"}"
                                }
                            }]
                        }
                    }]
                })
            } else {
                json!({
                    "choices": [{
                        "message": {
                            "role": "assistant",
                            "content": "Implemented after observing tool output."
                        }
                    }]
                })
            };

            Ok(LlmResponse {
                content: "Implemented after observing tool output.".to_string(),
                model: "react-test".to_string(),
                usage: Some(UsageStats {
                    input_tokens: 1,
                    output_tokens: 1,
                }),
                raw_response,
            })
        }

        fn stream_complete(
            &self,
            _prompt: &str,
        ) -> zcode_core::Result<zcode_llm_provider::provider::StreamingResponse> {
            Err(zcode_core::ZcodeError::LlmApiError(
                "streaming unsupported in ReActProvider test double".to_string(),
            ))
        }
    }

    #[test]
    fn test_coder_new() {
        let coder = make_coder();
        assert_eq!(coder.state(), AgentState::Idle);
        assert_eq!(coder.agent_type(), AgentType::Coder);
    }

    #[test]
    fn test_coder_not_busy_when_idle() {
        let coder = make_coder();
        assert!(!coder.is_busy());
    }

    #[tokio::test]
    async fn test_coder_handle_task_assigned() {
        let coder = make_coder();
        let task = Task::new("Write a hello world function");
        // Inject MockLlmProvider so the test doesn't depend on env vars or network
        let provider: Arc<dyn zcode_llm_provider::provider::LlmProvider> =
            Arc::new(zcode_llm_provider::provider::MockLlmProvider::new("Done!"));
        let task_result = coder.execute_task_with(&task, Some(provider)).await;
        assert!(task_result.success, "task should succeed with mock LLM: {:?}", task_result.output);
        let result = Some(AgentMessage::TaskCompleted {
            agent: coder.id().clone(),
            result: task_result,
        });
        match result.unwrap() {
            AgentMessage::TaskCompleted { result, .. } => {
                assert!(result.success);
            }
            _ => panic!("Expected TaskCompleted"),
        }
    }

    #[tokio::test]
    async fn test_coder_execute_task_returns_result() {
        let coder = make_coder();
        let task = Task::new("Do something");
        // Always use MockLlmProvider in unit tests to avoid network calls
        let provider: Arc<dyn zcode_llm_provider::provider::LlmProvider> =
            Arc::new(zcode_llm_provider::provider::MockLlmProvider::new("Task done!"));
        let result = coder.execute_task_with(&task, Some(provider)).await;
        assert!(result.success, "task should succeed: {:?}", result.output);
        assert!(!result.output.is_empty());
        assert_eq!(result.llm_calls, 1);
    }

    #[tokio::test]
    async fn test_coder_uses_react_tool_loop() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);
        let coder = CoderAgent::new(Arc::new(registry));
        let task = Task::new("Inspect with a tool before finishing");
        let provider: Arc<dyn zcode_llm_provider::provider::LlmProvider> =
            Arc::new(ReActProvider::new());

        let result = coder.execute_task_with(&task, Some(provider)).await;

        assert!(result.success, "task should finish after ReAct loop");
        assert_eq!(result.llm_calls, 2);
        assert_eq!(result.tool_calls, 1);
        assert!(result.output.contains("Implemented after observing"));
    }

    #[tokio::test]
    async fn test_coder_reset() {
        let mut coder = make_coder();
        coder.state = AgentState::Executing;
        coder.reset().await.unwrap();
        assert_eq!(coder.state(), AgentState::Idle);
    }

    #[test]
    fn test_coder_with_system_prompt() {
        let coder = make_coder().with_system_prompt("Custom prompt");
        assert_eq!(coder.loop_config.system_prompt, "Custom prompt");
    }
}
