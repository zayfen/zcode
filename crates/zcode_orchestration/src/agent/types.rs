//! Agent DTO re-exports.
//!
//! Shared agent/task/session data lives in `zcode_core` so orchestration,
//! requirements, session, and UI crates all use the same types.

pub use zcode_core::agent::{
    AgentId, AgentMessage, AgentState, AgentType, Task, TaskPriority, TaskResult,
};

