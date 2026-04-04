//! Orchestrator Agent
//!
//! The top-level coordinator: receives user tasks, decomposes them,
//! and delegates to Coder/Planner agents.

use crate::agent::bus::BusHandle;
use crate::agent::traits::AgentTrait;
use crate::agent::types::{
    AgentId, AgentMessage, AgentState, AgentType, Task, TaskResult,
};
use crate::error::Result;
use async_trait::async_trait;

/// The Orchestrator: receives user requests, breaks them into subtasks,
/// and delegates to specialized agents.
pub struct OrchestratorAgent {
    id: AgentId,
    state: AgentState,
    bus: Option<BusHandle>,
    /// Tasks that have been dispatched but not yet completed
    pending_tasks: Vec<String>,
}

impl OrchestratorAgent {
    /// Create a standalone orchestrator (no bus connected)
    pub fn new() -> Self {
        Self {
            id: AgentId::named("orchestrator"),
            state: AgentState::Idle,
            bus: None,
            pending_tasks: Vec::new(),
        }
    }

    /// Attach a bus handle for delegating to sub-agents
    pub fn with_bus(mut self, bus: BusHandle) -> Self {
        self.bus = Some(bus);
        self
    }

    /// Decompose a high-level task description into subtasks.
    /// In a real implementation this would call the LLM; here we do simple heuristics.
    pub fn decompose_task(&self, task: &Task) -> Vec<Task> {
        let desc = task.description.to_lowercase();

        let mut subtasks = Vec::new();

        // Heuristic: if the task mentions files or reading, add a search/read step
        if desc.contains("read") || desc.contains("find") || desc.contains("search") || desc.contains("analyze") {
            let mut t = Task::new(format!("Analyze codebase for: {}", task.description));
            t.parent_id = Some(task.id.clone());
            if let Some(cwd) = &task.cwd {
                t.cwd = Some(cwd.clone());
            }
            subtasks.push(t);
        }

        // If the task involves writing or editing, add a coding step
        if desc.contains("write") || desc.contains("edit") || desc.contains("implement")
            || desc.contains("fix") || desc.contains("add") || desc.contains("create")
        {
            let mut t = Task::new(format!("Implement: {}", task.description));
            t.parent_id = Some(task.id.clone());
            if let Some(cwd) = &task.cwd {
                t.cwd = Some(cwd.clone());
            }
            subtasks.push(t);
        }

        // If no specific pattern, create a single generic coder task
        if subtasks.is_empty() {
            let mut t = Task::new(task.description.clone());
            t.parent_id = Some(task.id.clone());
            if let Some(cwd) = &task.cwd {
                t.cwd = Some(cwd.clone());
            }
            subtasks.push(t);
        }

        subtasks
    }

    fn transition(&mut self, next: AgentState) {
        if self.state.can_transition_to(next) {
            self.state = next;
        }
    }
}

impl Default for OrchestratorAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentTrait for OrchestratorAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Orchestrator
    }

    fn state(&self) -> AgentState {
        self.state
    }

    async fn handle(&mut self, message: AgentMessage) -> Result<Option<AgentMessage>> {
        match message {
            AgentMessage::TaskAssigned { task, from: _, .. } => {
                self.transition(AgentState::Planning);
                let subtasks = self.decompose_task(&task);
                self.pending_tasks.extend(subtasks.iter().map(|t| t.id.clone()));
                self.transition(AgentState::Executing);

                // Return a progress update
                Ok(Some(AgentMessage::ProgressUpdate {
                    agent: self.id.clone(),
                    progress: 0.1,
                    message: format!(
                        "Decomposed into {} subtask(s): {}",
                        subtasks.len(),
                        subtasks.iter().map(|t| t.description.as_str()).collect::<Vec<_>>().join("; ")
                    ),
                }))
            }

            AgentMessage::TaskCompleted { result, .. } => {
                self.pending_tasks.retain(|id| id != &result.task_id);

                if self.pending_tasks.is_empty() {
                    self.transition(AgentState::Completed);
                    Ok(Some(AgentMessage::TaskCompleted {
                        agent: self.id.clone(),
                        result: TaskResult::success(&self.id.0, result.output),
                    }))
                } else {
                    Ok(Some(AgentMessage::ProgressUpdate {
                        agent: self.id.clone(),
                        progress: 0.5,
                        message: format!("{} tasks remaining", self.pending_tasks.len()),
                    }))
                }
            }

            _ => Ok(None),
        }
    }

    async fn reset(&mut self) -> Result<()> {
        self.state = AgentState::Idle;
        self.pending_tasks.clear();
        Ok(())
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_new() {
        let orch = OrchestratorAgent::new();
        assert_eq!(orch.state(), AgentState::Idle);
        assert_eq!(orch.agent_type(), AgentType::Orchestrator);
    }

    #[test]
    fn test_decompose_read_task() {
        let orch = OrchestratorAgent::new();
        let task = Task::new("Read and analyze src/lib.rs");
        let subtasks = orch.decompose_task(&task);
        assert!(!subtasks.is_empty());
        assert!(subtasks[0].parent_id == Some(task.id));
        assert!(subtasks[0].description.contains("Analyze"));
    }

    #[test]
    fn test_decompose_write_task() {
        let orch = OrchestratorAgent::new();
        let task = Task::new("Implement a new function for sorting");
        let subtasks = orch.decompose_task(&task);
        assert!(subtasks.iter().any(|t| t.description.contains("Implement")));
    }

    #[test]
    fn test_decompose_generic_task() {
        let orch = OrchestratorAgent::new();
        let task = Task::new("Do something");
        let subtasks = orch.decompose_task(&task);
        assert_eq!(subtasks.len(), 1);
    }

    #[test]
    fn test_decompose_complex_task_both_steps() {
        let orch = OrchestratorAgent::new();
        let task = Task::new("Read the existing code and then implement the new feature");
        let subtasks = orch.decompose_task(&task);
        assert!(subtasks.len() >= 2);
    }

    #[test]
    fn test_decompose_inherits_cwd() {
        let orch = OrchestratorAgent::new();
        let task = Task::new("Fix bug").with_cwd("/project");
        let subtasks = orch.decompose_task(&task);
        for sub in &subtasks {
            assert_eq!(sub.cwd, Some("/project".to_string()));
        }
    }

    #[tokio::test]
    async fn test_handle_task_assigned() {
        let mut orch = OrchestratorAgent::new();
        let task = Task::new("analyze code");
        let msg = AgentMessage::TaskAssigned {
            from: AgentId::named("user"),
            to: AgentId::named("orchestrator"),
            task,
        };
        let result = orch.handle(msg).await.unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            AgentMessage::ProgressUpdate { .. } => {}
            _ => panic!("Expected ProgressUpdate"),
        }
        assert_eq!(orch.state(), AgentState::Executing);
    }

    #[tokio::test]
    async fn test_handle_task_completed_clears_pending() {
        let mut orch = OrchestratorAgent::new();
        orch.state = AgentState::Executing;
        let task_id = "task-123".to_string();
        orch.pending_tasks = vec![task_id.clone()];

        let result_msg = AgentMessage::TaskCompleted {
            agent: AgentId::named("coder"),
            result: TaskResult::success(&task_id, "Done"),
        };
        let resp = orch.handle(result_msg).await.unwrap().unwrap();
        match resp {
            AgentMessage::TaskCompleted { .. } => {}
            _ => panic!("Expected TaskCompleted"),
        }
        assert_eq!(orch.state(), AgentState::Completed);
    }

    #[tokio::test]
    async fn test_reset() {
        let mut orch = OrchestratorAgent::new();
        orch.state = AgentState::Executing;
        orch.pending_tasks = vec!["t1".to_string()];
        orch.reset().await.unwrap();
        assert_eq!(orch.state(), AgentState::Idle);
        assert!(orch.pending_tasks.is_empty());
    }
}
