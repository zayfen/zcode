//! Agent module for zcode
//!
//! Multi-agent system with Orchestrator, Planner, and Coder agents.
//! Uses tokio-based message passing for coordination.

pub mod types;
pub mod traits;
pub mod bus;
pub mod orchestrator;
pub mod planner;
pub mod coder;
pub mod loop_exec;
pub mod reviewer;

pub use types::{AgentId, AgentMessage, AgentState, AgentType, Task, TaskPriority, TaskResult};
pub use traits::AgentTrait;
pub use bus::{BusDispatcher, BusHandle, MessageBus};
pub use orchestrator::OrchestratorAgent;
pub use planner::PlannerAgent;
pub use coder::CoderAgent;
pub use loop_exec::{AgentLoop, LoopConfig, LoopResult, LlmResponse, ConversationMessage};
pub use reviewer::{ReviewerAgent, ReviewResult, ReviewIssue, IssueSeverity, ReviewCategory, ReviewConfig};
