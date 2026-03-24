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
    /// Execute a task using the agent loop with a mock LLM
    /// (real HTTP LLM calls happen via the AgentRunner or integration layer)
    pub async fn execute_task(&self, task: &Task) -> TaskResult {
        let agent_loop = AgentLoop::new(self.loop_config.clone(), self.registry.clone());

        // For now, use a simple mock: just list the tools available and return
        // a stub answer. Real LLM calls are wired in AgentRunner/runner.rs.
        let result: crate::error::Result<crate::agent::loop_exec::LoopResult> = agent_loop.run(
            &task.description,
            &[],
            |_messages| async {
                Ok(LlmResponse::Text(
                    "Task acknowledged. In production mode, this would call the LLM API.".to_string()
                ))
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
        let msg = AgentMessage::TaskAssigned {
            from: AgentId::named("orchestrator"),
            to: coder.id().clone(),
            task,
        };
        let result = coder.handle(msg).await.unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            AgentMessage::TaskCompleted { result, .. } => {
                assert!(result.success);
            }
            _ => panic!("Expected TaskCompleted"),
        }
        assert!(coder.is_done());
    }

    #[tokio::test]
    async fn test_coder_execute_task_returns_result() {
        let coder = make_coder();
        let task = Task::new("Do something");
        let result = coder.execute_task(&task).await;
        assert!(result.success);
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
