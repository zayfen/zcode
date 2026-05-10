//! Pre-built graph presets for common agent workflows
//!
//! These are ready-to-use `StateGraph` factories following LangGraph patterns.
//! Each preset returns an **uncompiled** `StateGraph` that you can further
//! customise before calling `.compile()`.
//!
//! # Available presets
//!
//! | Function                | Topology                                      |
//! |-------------------------|-----------------------------------------------|
//! | `linear_pipeline()`     | planner → coder → END                        |
//! | `review_pipeline()`     | planner → coder → reviewer ⟳ (until passed) |
//!
//! # Example
//!
//! ```rust,no_run
//! use zcode::agent::graph::{presets, DefaultState};
//!
//! let graph = presets::review_pipeline().compile()?;
//! let mut state = DefaultState::default();
//! let output = graph.execute(&mut state).await?;
//! # Ok::<(), zcode::ZcodeError>(())
//! ```

use crate::agent::graph::edge::routers;
use crate::agent::graph::graph::StateGraph;
use crate::agent::graph::node::FnNode;
use crate::agent::graph::state::NodeOutput;
use crate::agent::types::AgentState;

/// Linear pipeline: planner → coder → END (no review loop)
///
/// Useful for simple tasks where you trust the coder's output.
pub fn linear_pipeline() -> StateGraph {
    let mut g = StateGraph::new("planner");

    g.add_node(FnNode::new("planner", |state| {
        state.agent_state = AgentState::Planning;
        Ok(NodeOutput::None)
    }));

    g.add_node(FnNode::new("coder", |state| {
        state.agent_state = AgentState::Executing;
        Ok(NodeOutput::None)
    }));

    g.add_edge("planner", "coder");
    // coder has no outgoing edge → natural END
    g
}

/// Review pipeline: planner → coder → reviewer ⟳
///
/// The reviewer loops back to the coder until `review_passed = true`
/// is set in state metadata, then the graph ends.
///
/// Your reviewer node should set:
/// ```rust,ignore
/// state.metadata.insert("review_passed".into(), serde_json::json!(true));
/// ```
pub fn review_pipeline() -> StateGraph {
    let mut g = StateGraph::new("planner");

    g.add_node(FnNode::new("planner", |state| {
        state.agent_state = AgentState::Planning;
        Ok(NodeOutput::None)
    }));

    g.add_node(FnNode::new("coder", |state| {
        state.agent_state = AgentState::Executing;
        // Reset review_passed so reviewer re-evaluates each time
        state.metadata.remove("review_passed");
        Ok(NodeOutput::None)
    }));

    g.add_node(FnNode::new("reviewer", |state| {
        state.agent_state = AgentState::Reviewing;
        Ok(NodeOutput::None)
    }));

    g.add_edge("planner", "coder");
    g.add_edge("coder", "reviewer");
    // reviewer: if review_passed → END, else → coder
    g.add_conditional_edge(
        "reviewer",
        routers::review_router("coder"),
        vec!["coder", "__end__"],
    );
    g
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_pipeline_compiles() {
        let g = linear_pipeline();
        assert!(g.compile().is_ok());
    }

    #[test]
    fn test_review_pipeline_compiles() {
        let g = review_pipeline();
        assert!(g.compile().is_ok());
    }

    #[tokio::test]
    async fn test_linear_pipeline_executes() {
        use crate::agent::graph::state::DefaultState;

        let compiled = linear_pipeline().compile().unwrap();
        let mut state = DefaultState::default();
        let out = compiled.execute(&mut state).await.unwrap();

        assert!(out.nodes_executed.contains(&"planner".to_string()));
        assert!(out.nodes_executed.contains(&"coder".to_string()));
        assert_eq!(state.agent_state, AgentState::Executing);
    }

    #[tokio::test]
    async fn test_review_pipeline_passes_on_first_try() {
        use crate::agent::graph::state::DefaultState;

        let mut g = review_pipeline();
        // Override reviewer to pass immediately
        g.add_node(FnNode::new("reviewer", |state| {
            state
                .metadata
                .insert("review_passed".into(), serde_json::json!(true));
            Ok(NodeOutput::None)
        }));

        let compiled = g.compile().unwrap();
        let mut state = DefaultState::default();
        let out = compiled.execute(&mut state).await.unwrap();

        // Should have executed: planner → coder → reviewer → END
        assert_eq!(out.nodes_executed.len(), 3);
        assert_eq!(out.nodes_executed[2], "reviewer");
    }

    #[tokio::test]
    async fn test_review_pipeline_retries_once() {
        use crate::agent::graph::state::DefaultState;

        let mut g = review_pipeline().max_iterations(10);

        // Reviewer fails first time, passes second
        let call_count = std::sync::Arc::new(std::sync::Mutex::new(0usize));
        let cc = call_count.clone();
        g.add_node(FnNode::new("reviewer", move |state| {
            let mut count = cc.lock().unwrap();
            *count += 1;
            if *count >= 2 {
                state
                    .metadata
                    .insert("review_passed".into(), serde_json::json!(true));
            }
            Ok(NodeOutput::None)
        }));

        let compiled = g.compile().unwrap();
        let mut state = DefaultState::default();
        let out = compiled.execute(&mut state).await.unwrap();

        // planner → coder → reviewer(fail) → coder → reviewer(pass) = 5 nodes
        assert_eq!(out.nodes_executed.len(), 5);
        assert_eq!(*call_count.lock().unwrap(), 2);
    }
}
