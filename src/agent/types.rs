//! Core types for the zcode Agent system
//!
//! Defines the fundamental data structures used across all agents.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ─── AgentId ───────────────────────────────────────────────────────────────────

/// Unique identifier for an agent instance
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    /// Generate a new unique ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create an ID from a string (for named agents)
    pub fn named(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ─── AgentType ─────────────────────────────────────────────────────────────────

/// The role/type of an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    /// Orchestrates other agents, decomposes tasks
    Orchestrator,
    /// Analyzes codebase and creates execution plans
    Planner,
    /// Reads, writes, edits code files using tools
    Coder,
    /// Reviews code changes and gives feedback
    Reviewer,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::Orchestrator => write!(f, "Orchestrator"),
            AgentType::Planner => write!(f, "Planner"),
            AgentType::Coder => write!(f, "Coder"),
            AgentType::Reviewer => write!(f, "Reviewer"),
        }
    }
}

// ─── AgentState ────────────────────────────────────────────────────────────────

/// State machine for an agent's lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    /// Waiting for a task
    Idle,
    /// Analyzing the task and creating a plan
    Planning,
    /// Actively executing tools or calling LLM
    Executing,
    /// Reviewing results before completing
    Reviewing,
    /// Task finished successfully
    Completed,
    /// Task failed with an error
    Failed,
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentState::Idle => write!(f, "Idle"),
            AgentState::Planning => write!(f, "Planning"),
            AgentState::Executing => write!(f, "Executing"),
            AgentState::Reviewing => write!(f, "Reviewing"),
            AgentState::Completed => write!(f, "Completed"),
            AgentState::Failed => write!(f, "Failed"),
        }
    }
}

impl AgentState {
    /// Whether this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, AgentState::Completed | AgentState::Failed)
    }

    /// Valid transitions from this state
    pub fn can_transition_to(&self, next: AgentState) -> bool {
        use AgentState::*;
        match (self, next) {
            (Idle, Planning) => true,
            (Idle, Executing) => true,
            (Planning, Executing) => true,
            (Planning, Failed) => true,
            (Executing, Reviewing) => true,
            (Executing, Completed) => true,
            (Executing, Failed) => true,
            (Reviewing, Completed) => true,
            (Reviewing, Executing) => true, // re-execute after review
            (Reviewing, Failed) => true,
            _ => false,
        }
    }
}

// ─── Task ──────────────────────────────────────────────────────────────────────

/// Task priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

/// A unit of work assigned to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: String,
    /// Human-readable description of what to do
    pub description: String,
    /// Optional working directory for this task
    pub cwd: Option<String>,
    /// Additional context key-value pairs
    pub context: HashMap<String, String>,
    /// Task priority
    pub priority: TaskPriority,
    /// Parent task ID (if this is a subtask)
    pub parent_id: Option<String>,
}

impl Task {
    /// Create a new task with a generated ID
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.into(),
            cwd: None,
            context: HashMap::new(),
            priority: TaskPriority::Normal,
            parent_id: None,
        }
    }

    /// Set working directory
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Add context key-value
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }
}

// ─── TaskResult ────────────────────────────────────────────────────────────────

/// Result of a completed task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task ID this result belongs to
    pub task_id: String,
    /// Whether the task completed successfully
    pub success: bool,
    /// Output / answer from the agent
    pub output: String,
    /// Files that were modified during the task
    pub files_modified: Vec<String>,
    /// Error message if the task failed
    pub error: Option<String>,
    /// Number of LLM calls made
    pub llm_calls: usize,
    /// Number of tool calls made
    pub tool_calls: usize,
}

impl TaskResult {
    pub fn success(task_id: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            success: true,
            output: output.into(),
            files_modified: Vec::new(),
            error: None,
            llm_calls: 0,
            tool_calls: 0,
        }
    }

    pub fn failure(task_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            success: false,
            output: String::new(),
            files_modified: Vec::new(),
            error: Some(error.into()),
            llm_calls: 0,
            tool_calls: 0,
        }
    }
}

// ─── AgentMessage ──────────────────────────────────────────────────────────────

/// Messages exchanged between agents via the MessageBus
#[derive(Debug, Clone)]
pub enum AgentMessage {
    /// Assign a task to an agent
    TaskAssigned {
        from: AgentId,
        to: AgentId,
        task: Task,
    },
    /// Agent reports progress (0.0 - 1.0)
    ProgressUpdate {
        agent: AgentId,
        progress: f32,
        message: String,
    },
    /// Agent requests execution of a tool
    ToolRequest {
        agent: AgentId,
        tool_name: String,
        input: serde_json::Value,
    },
    /// Result of tool execution sent back to agent
    ToolResult {
        agent: AgentId,
        tool_name: String,
        result: serde_json::Value,
    },
    /// Task completed (success or failure)
    TaskCompleted {
        agent: AgentId,
        result: TaskResult,
    },
    /// Agent spawned a new sub-agent
    SubAgentSpawned {
        parent: AgentId,
        child: AgentId,
        agent_type: AgentType,
        task: Task,
    },
    /// Broadcast a text chunk to the TUI
    StreamChunk {
        agent: AgentId,
        chunk: String,
    },
}

impl AgentMessage {
    /// Get the sender agent ID (if applicable)
    pub fn sender(&self) -> Option<&AgentId> {
        match self {
            AgentMessage::TaskAssigned { from, .. } => Some(from),
            AgentMessage::ProgressUpdate { agent, .. } => Some(agent),
            AgentMessage::ToolRequest { agent, .. } => Some(agent),
            AgentMessage::ToolResult { agent, .. } => Some(agent),
            AgentMessage::TaskCompleted { agent, .. } => Some(agent),
            AgentMessage::SubAgentSpawned { parent, .. } => Some(parent),
            AgentMessage::StreamChunk { agent, .. } => Some(agent),
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_new_is_unique() {
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_agent_id_named() {
        let id = AgentId::named("orchestrator");
        assert_eq!(id.0, "orchestrator");
    }

    #[test]
    fn test_agent_id_display() {
        let id = AgentId::named("test-agent");
        assert_eq!(format!("{}", id), "test-agent");
    }

    #[test]
    fn test_agent_id_default() {
        let id = AgentId::default();
        assert!(!id.0.is_empty());
    }

    #[test]
    fn test_agent_type_display() {
        assert_eq!(format!("{}", AgentType::Orchestrator), "Orchestrator");
        assert_eq!(format!("{}", AgentType::Planner), "Planner");
        assert_eq!(format!("{}", AgentType::Coder), "Coder");
        assert_eq!(format!("{}", AgentType::Reviewer), "Reviewer");
    }

    #[test]
    fn test_agent_state_display() {
        assert_eq!(format!("{}", AgentState::Idle), "Idle");
        assert_eq!(format!("{}", AgentState::Executing), "Executing");
        assert_eq!(format!("{}", AgentState::Completed), "Completed");
    }

    #[test]
    fn test_agent_state_is_terminal() {
        assert!(!AgentState::Idle.is_terminal());
        assert!(!AgentState::Planning.is_terminal());
        assert!(!AgentState::Executing.is_terminal());
        assert!(AgentState::Completed.is_terminal());
        assert!(AgentState::Failed.is_terminal());
    }

    #[test]
    fn test_agent_state_transitions_valid() {
        assert!(AgentState::Idle.can_transition_to(AgentState::Planning));
        assert!(AgentState::Planning.can_transition_to(AgentState::Executing));
        assert!(AgentState::Executing.can_transition_to(AgentState::Completed));
        assert!(AgentState::Executing.can_transition_to(AgentState::Reviewing));
        assert!(AgentState::Reviewing.can_transition_to(AgentState::Completed));
    }

    #[test]
    fn test_agent_state_transitions_invalid() {
        assert!(!AgentState::Idle.can_transition_to(AgentState::Completed));
        assert!(!AgentState::Completed.can_transition_to(AgentState::Idle));
        assert!(!AgentState::Failed.can_transition_to(AgentState::Executing));
    }

    #[test]
    fn test_task_new() {
        let task = Task::new("Implement feature X");
        assert!(!task.id.is_empty());
        assert_eq!(task.description, "Implement feature X");
        assert_eq!(task.priority, TaskPriority::Normal);
        assert!(task.cwd.is_none());
        assert!(task.parent_id.is_none());
    }

    #[test]
    fn test_task_builder() {
        let task = Task::new("Fix bug")
            .with_cwd("/tmp/project")
            .with_context("file", "src/main.rs")
            .with_priority(TaskPriority::High);

        assert_eq!(task.cwd, Some("/tmp/project".to_string()));
        assert_eq!(task.context.get("file"), Some(&"src/main.rs".to_string()));
        assert_eq!(task.priority, TaskPriority::High);
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }

    #[test]
    fn test_task_result_success() {
        let r = TaskResult::success("task-1", "Done!");
        assert!(r.success);
        assert_eq!(r.output, "Done!");
        assert!(r.error.is_none());
        assert_eq!(r.task_id, "task-1");
    }

    #[test]
    fn test_task_result_failure() {
        let r = TaskResult::failure("task-2", "File not found");
        assert!(!r.success);
        assert!(r.error.is_some());
        assert_eq!(r.error.unwrap(), "File not found");
    }

    #[test]
    fn test_agent_message_task_assigned() {
        let from = AgentId::named("orch");
        let to = AgentId::named("coder");
        let task = Task::new("write tests");
        let msg = AgentMessage::TaskAssigned {
            from: from.clone(),
            to,
            task,
        };
        assert_eq!(msg.sender().unwrap(), &from);
    }

    #[test]
    fn test_agent_message_progress() {
        let id = AgentId::named("agent1");
        let msg = AgentMessage::ProgressUpdate {
            agent: id.clone(),
            progress: 0.5,
            message: "halfway".to_string(),
        };
        assert_eq!(msg.sender().unwrap(), &id);
    }

    #[test]
    fn test_agent_message_stream_chunk() {
        let id = AgentId::named("coder");
        let msg = AgentMessage::StreamChunk {
            agent: id.clone(),
            chunk: "Hello".to_string(),
        };
        assert_eq!(msg.sender().unwrap(), &id);
    }
}
