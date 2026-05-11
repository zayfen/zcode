//! Shared agent/session DTOs used across orchestration, requirements, and UI.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for an agent instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

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

/// The role/type of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    Orchestrator,
    Planner,
    Coder,
    Reviewer,
    SelfLearning,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::Orchestrator => write!(f, "Orchestrator"),
            AgentType::Planner => write!(f, "Planner"),
            AgentType::Coder => write!(f, "Coder"),
            AgentType::Reviewer => write!(f, "Reviewer"),
            AgentType::SelfLearning => write!(f, "SelfLearning"),
        }
    }
}

/// State machine for an agent's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    Idle,
    Planning,
    Executing,
    Reviewing,
    Learning,
    Completed,
    Failed,
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentState::Idle => write!(f, "Idle"),
            AgentState::Planning => write!(f, "Planning"),
            AgentState::Executing => write!(f, "Executing"),
            AgentState::Reviewing => write!(f, "Reviewing"),
            AgentState::Learning => write!(f, "Learning"),
            AgentState::Completed => write!(f, "Completed"),
            AgentState::Failed => write!(f, "Failed"),
        }
    }
}

impl AgentState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, AgentState::Completed | AgentState::Failed)
    }

    pub fn can_transition_to(&self, next: AgentState) -> bool {
        use AgentState::*;
        matches!(
            (self, next),
            (Idle, Planning)
                | (Idle, Executing)
                | (Planning, Executing)
                | (Planning, Failed)
                | (Executing, Reviewing)
                | (Executing, Completed)
                | (Executing, Failed)
                | (Reviewing, Completed)
                | (Reviewing, Executing)
                | (Reviewing, Learning)
                | (Reviewing, Failed)
                | (Learning, Completed)
                | (Learning, Failed)
        )
    }
}

/// Task priority level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum TaskPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// A unit of work assigned to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub cwd: Option<String>,
    pub context: HashMap<String, String>,
    pub priority: TaskPriority,
    pub parent_id: Option<String>,
}

impl Task {
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

    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }
}

/// Result of a completed task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub success: bool,
    pub output: String,
    pub files_modified: Vec<String>,
    pub error: Option<String>,
    pub llm_calls: usize,
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

/// OpenAI-compatible conversation message used by agent graph/session storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ConversationMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn assistant_text(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn assistant_tool_calls(tool_calls: Vec<serde_json::Value>) -> Self {
        Self {
            role: "assistant".into(),
            content: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            name: None,
        }
    }

    pub fn tool_result(
        tool_call_id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            role: "tool".into(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            name: Some(name.into()),
        }
    }
}

/// Default persisted graph/session state shared across orchestration and
/// requirements task storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultState {
    pub messages: Vec<ConversationMessage>,
    pub task: Option<Task>,
    pub result: Option<TaskResult>,
    pub agent_state: AgentState,
    pub metadata: HashMap<String, serde_json::Value>,
    pub iteration: usize,
}

impl DefaultState {
    pub fn new(task: Task) -> Self {
        Self {
            task: Some(task),
            ..Default::default()
        }
    }

    pub fn with_message(mut self, msg: ConversationMessage) -> Self {
        self.messages.push(msg);
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.messages.insert(0, ConversationMessage::system(prompt.into()));
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

impl Default for DefaultState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            task: None,
            result: None,
            agent_state: AgentState::Idle,
            metadata: HashMap::new(),
            iteration: 0,
        }
    }
}

/// Messages exchanged between agents via the MessageBus.
#[derive(Debug, Clone)]
pub enum AgentMessage {
    TaskAssigned {
        from: AgentId,
        to: AgentId,
        task: Task,
    },
    ProgressUpdate {
        agent: AgentId,
        progress: f32,
        message: String,
    },
    ToolRequest {
        agent: AgentId,
        tool_name: String,
        input: serde_json::Value,
    },
    ToolResult {
        agent: AgentId,
        tool_name: String,
        result: serde_json::Value,
    },
    TaskCompleted {
        agent: AgentId,
        result: TaskResult,
    },
    SubAgentSpawned {
        parent: AgentId,
        child: AgentId,
        agent_type: AgentType,
        task: Task,
    },
    StreamChunk {
        agent: AgentId,
        chunk: String,
    },
}

impl AgentMessage {
    pub fn agent_id(&self) -> Option<&AgentId> {
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

    pub fn sender(&self) -> Option<&AgentId> {
        self.agent_id()
    }
}
