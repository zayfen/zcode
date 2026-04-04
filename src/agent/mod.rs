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

pub mod graph;
pub mod types;
pub mod traits;
pub mod bus;
pub mod orchestrator;
pub mod planner;
pub mod coder;
pub mod loop_exec;
pub mod reviewer;

// ── LangGraph-style graph orchestration ──────────────────────────────────────
pub use graph::{
    // Graph builder & executor
    StateGraph, CompiledGraph,
    // Output & events
    GraphOutput, GraphEvent, EndReason,
    // Node types
    GraphNode, FnNode, AsyncFnNode,
    // Edge routing
    Edge, routers,
    // State
    DefaultState, NodeOutput, GraphState,
};

// ── Agent types ───────────────────────────────────────────────────────────────
pub use types::{AgentId, AgentMessage, AgentState, AgentType, Task, TaskPriority, TaskResult};
pub use traits::AgentTrait;
pub use bus::{BusDispatcher, BusHandle, MessageBus};

// ── Concrete agent implementations ───────────────────────────────────────────
pub use orchestrator::OrchestratorAgent;
pub use planner::PlannerAgent;
pub use coder::CoderAgent;
pub use loop_exec::{AgentLoop, LoopConfig, LoopResult, LlmResponse, ConversationMessage};
pub use reviewer::{ReviewerAgent, ReviewResult, ReviewIssue, IssueSeverity, ReviewCategory, ReviewConfig};
