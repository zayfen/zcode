//! Agent module — multi-agent orchestration for zcode
//!
//! Provides a LangGraph-style state-graph orchestration layer on top of
//! the existing agent types (Orchestrator, Planner, Coder, Reviewer).
//!
//! # Architecture
//!
//! ```text
//! StateGraph (builder)
//!   └─ add_node(FnNode / AsyncFnNode)
//!   └─ add_edge / add_conditional_edge
//!   └─ compile() → CompiledGraph
//!                     └─ execute(&mut DefaultState)
//!                            ├─ NodeStart  event
//!                            ├─ run node   (read/write state)
//!                            ├─ NodeComplete event
//!                            └─ EdgeTraversed → next node | END
//! ```

pub mod bus;
pub mod coder;
pub mod graph;
pub mod loop_exec;
pub mod orchestrator;
pub mod planner;
pub mod reviewer;
pub mod self_learning;
pub mod traits;
pub mod types;

// ── LangGraph-style graph orchestration ──────────────────────────────────────
pub use graph::pipeline::{AgentModelLabels, AgentRuntime, TaskAgentRuntimes};
pub use graph::{
    routers,
    AsyncFnNode,
    CompiledGraph,
    // State
    DefaultState,
    // Edge routing
    Edge,
    EndReason,
    FnNode,
    GraphEvent,
    // Node types
    GraphNode,
    // Output & events
    GraphOutput,
    GraphState,
    NodeOutput,
    // Graph builder & executor
    StateGraph,
};

// ── Agent types ───────────────────────────────────────────────────────────────
pub use bus::{BusDispatcher, BusHandle, MessageBus};
pub use traits::AgentTrait;
pub use types::{AgentId, AgentMessage, AgentState, AgentType, Task, TaskPriority, TaskResult};

// ── Concrete agent implementations ───────────────────────────────────────────
pub use coder::CoderAgent;
pub use loop_exec::{
    AgentLoop, ConversationMessage, LlmResponse, LoopConfig, LoopEvent, LoopResult,
};
pub use orchestrator::OrchestratorAgent;
pub use planner::PlannerAgent;
pub use reviewer::{
    IssueSeverity, ReviewCategory, ReviewConfig, ReviewIssue, ReviewResult, ReviewerAgent,
};
pub use self_learning::{LearningEntry, SelfLearningAgent};
