//! Coder Agent
//!
//! Executes coding tasks using the tool registry (file_read, file_write,
//! file_edit, search, shell, glob) via the ReAct loop.

use crate::agent::loop_exec::{AgentLoop, LoopConfig, LlmResponse};
use crate::agent::traits::AgentTrait;
use crate::agent::types::{AgentId, AgentMessage, AgentState, AgentType, Task, TaskResult};
use crate::error::Result;
use crate::tools::ToolRegistry;
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
                    You write clean, idiomatic code and use the available tools \
                    to read files, make edits, and run commands. \
                    Always verify your changes work before reporting completion."
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
        provider: Option<std::sync::Arc<dyn crate::llm::provider::LlmProvider>>,
    ) -> TaskResult {
        use crate::llm::provider::{LlmProvider, MockLlmProvider, RigProvider};
        use crate::llm::{LlmConfig, Message, MessageRole};
        use std::sync::Arc;

        let agent_loop = AgentLoop::new(self.loop_config.clone(), self.registry.clone());

        // Determine the LLM provider: injected > env-based RigProvider > offline stub
        let effective_provider: Arc<dyn LlmProvider> = if let Some(p) = provider {
            p
        } else {
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                .ok();

            if let Some(_key) = api_key {
                let provider_name = if std::env::var("OPENAI_API_KEY").is_ok()
                    && std::env::var("ANTHROPIC_API_KEY").is_err()
                {
                    "openai"
                } else {
                    "anthropic"
                };
                let llm_config = LlmConfig {
                    provider: provider_name.to_string(),
                    model: if provider_name == "openai" {
                        "gpt-4o".to_string()
                    } else {
                        "claude-3-5-sonnet-20241022".to_string()
                    },
                    ..Default::default()
                };
                Arc::new(RigProvider::new(llm_config))
            } else {
                Arc::new(MockLlmProvider::new(
                    "Task acknowledged. Set ANTHROPIC_API_KEY or OPENAI_API_KEY for real LLM responses."
                ))
            }
        };

        let result: crate::error::Result<crate::agent::loop_exec::LoopResult> = agent_loop.run(
            &task.description,
            &[],
            move |messages| {
                let p = Arc::clone(&effective_provider);
                async move {
                    let llm_messages: Vec<Message> = messages.iter()
                        .filter_map(|v| {
                            let role = v.get("role")?.as_str()?;
                            let content = v.get("content")?.as_str().unwrap_or("").to_string();
                            let role = match role {
                                "system" => MessageRole::System,
                                "assistant" => MessageRole::Assistant,
                                _ => MessageRole::User,
                            };
                            Some(Message { role, content })
                        })
                        .collect();

                    match p.chat(&llm_messages) {
                        Ok(resp) => Ok(LlmResponse::Text(resp.content)),
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
    use crate::tools::register_default_tools;

    fn make_coder() -> CoderAgent {
        let mut r = ToolRegistry::new();
        register_default_tools(&mut r);
        CoderAgent::new(Arc::new(r))
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
        let mut coder = make_coder();
        let task = Task::new("Write a hello world function");
        // Inject MockLlmProvider so the test doesn't depend on env vars or network
        let provider: Arc<dyn crate::llm::provider::LlmProvider> =
            Arc::new(crate::llm::provider::MockLlmProvider::new("Done!"));
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
        let provider: Arc<dyn crate::llm::provider::LlmProvider> =
            Arc::new(crate::llm::provider::MockLlmProvider::new("Task done!"));
        let result = coder.execute_task_with(&task, Some(provider)).await;
        assert!(result.success, "task should succeed: {:?}", result.output);
        assert!(!result.output.is_empty());
        assert_eq!(result.llm_calls, 1);
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
