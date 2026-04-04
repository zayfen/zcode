//! Agent graph module — LangGraph 风格的状态图编排
//!
//! # LangGraph 核心概念对应关系
//!
//! | LangGraph (Python)          | zcode (Rust)                     |
//! |-----------------------------|----------------------------------|
//! | `StateGraph`                | `StateGraph`                     |
//! | `.add_node(name, fn)`       | `.add_node(FnNode::new(name, f))`|
//! | `.add_edge(a, b)`           | `.add_edge(a, b)`                |
//! | `.add_conditional_edges()`  | `.add_conditional_edge()`        |
//! | `graph.compile()`           | `.compile()? -> CompiledGraph`   |
//! | `graph.invoke(state)`       | `.execute(&mut state).await`     |
//! | `END` sentinel              | `None` from router / no edges    |
//! | `StateSnapshot`             | `Checkpoint` (checkpoints module)|
//!
//! # 快速开始
//!
//! ```rust,no_run
//! use zcode::agent::graph::{StateGraph, FnNode, NodeOutput, DefaultState, routers};
//!
//! let mut graph = StateGraph::new("planner");
//! graph.add_node(FnNode::new("planner",  |s| { /* ... */ Ok(NodeOutput::None) }));
//! graph.add_node(FnNode::new("coder",    |s| { /* ... */ Ok(NodeOutput::None) }));
//! graph.add_node(FnNode::new("reviewer", |s| { /* ... */ Ok(NodeOutput::None) }));
//! graph.add_edge("planner", "coder");
//! graph.add_edge("coder",   "reviewer");
//! graph.add_conditional_edge(
//!     "reviewer",
//!     routers::review_router("coder"),   // fail → coder, pass → END
//!     vec!["coder", "__end__"],
//! );
//!
//! let compiled = graph.compile()?;
//! let mut state = DefaultState::default();
//! let output = compiled.execute(&mut state).await?;
//! println!("Executed {} nodes, ended: {}", output.total_iterations, output.end_reason);
//! # Ok::<(), zcode::ZcodeError>(())
//! ```

pub mod checkpoint;
pub mod edge;
pub mod graph;
pub mod node;
pub mod presets;
pub mod state;

// ── Core graph types ──────────────────────────────────────────────────────────
pub use graph::{CompiledGraph, EndReason, GraphEvent, GraphOutput, StateGraph};

// ── Node types ────────────────────────────────────────────────────────────────
pub use node::{AsyncFnNode, FnNode, GraphNode};

// ── Edge types & routers ──────────────────────────────────────────────────────
pub use edge::{Edge, routers};

// ── State types ───────────────────────────────────────────────────────────────
pub use state::{DefaultState, GraphState, NodeOutput};

// ── Presets (pre-built pipelines) ─────────────────────────────────────────────
pub use presets::*;
