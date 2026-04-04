//! Edge definitions for graph routing
//!
//! Edges define transitions between nodes.
//! `Always` = unconditional jump, `Conditional` = router-based routing.

use crate::agent::graph::state::DefaultState;

// ─── Edge ─────────────────────────────────────────────────────────────────────

/// An edge in the state graph.
///
/// `Always`: unconditionally jump from one node to another.
/// `Conditional`: call a router function to decide the next node at runtime.
///   Return `Some(node_name)` to continue, `None` to end the graph.
pub enum Edge {
    Always { from: String, to: String },
    Conditional {
        from: String,
        router: Box<dyn Fn(&DefaultState) -> Option<String> + Send + Sync>,
        branches: Vec<String>,
    },
}

impl Edge {
    /// Create an unconditional edge: `from` → `to`
    pub fn always(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::Always {
            from: from.into(),
            to: to.into(),
        }
    }

    /// Create a conditional edge from `from` with a router function
    pub fn conditional(
        from: impl Into<String>,
        router: impl Fn(&DefaultState) -> Option<String> + Send + Sync + 'static,
        branches: Vec<&str>,
    ) -> Self {
        Self::Conditional {
            from: from.into(),
            router: Box::new(router),
            branches: branches.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Source node name
    pub fn from_node(&self) -> &str {
        match self {
            Edge::Always { from, .. } => from,
            Edge::Conditional { from, .. } => from,
        }
    }

    /// Target node name (Only meaningful for Always edges)
    pub fn to_node(&self) -> Option<&str> {
        match self {
            Edge::Always { to, .. } => Some(to),
            Edge::Conditional { .. } => None,
        }
    }
}

// ─── Built-in routers ─────────────────────────────────────────────────────────

/// Pre-built router functions for common patterns
pub mod routers {
    use crate::agent::graph::state::DefaultState;

    /// Router: check metadata key for a specific value → END if matched
    pub fn metadata_eq(
        key: &str,
        value: &str,
        retry_node: &str,
    ) -> impl Fn(&DefaultState) -> Option<String> + Send + Sync + 'static {
        let key = key.to_string();
        let value = value.to_string();
        let retry_node = retry_node.to_string();
        move |state: &DefaultState| match state.metadata.get(&key) {
            Some(v) if v.as_str() == Some(value.as_str()) => None,
            _ => Some(retry_node.clone()),
        }
    }

    /// Router: read `next_node` from metadata
    pub fn next_node_router() -> impl Fn(&DefaultState) -> Option<String> + Send + Sync {
        |state: &DefaultState| {
            state
                .metadata
                .get("next_node")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
        }
    }

    /// Router: end if iteration count >= max
    pub fn max_iterations(
        max: usize,
        retry_node: &str,
    ) -> impl Fn(&DefaultState) -> Option<String> + Send + Sync + 'static {
        let retry_node = retry_node.to_string();
        move |state: &DefaultState| {
            if state.iteration >= max {
                None
            } else {
                Some(retry_node.clone())
            }
        }
    }

    /// Router: check `review_passed` metadata — pass → END, fail → retry_node
    pub fn review_router(
        retry_node: &str,
    ) -> impl Fn(&DefaultState) -> Option<String> + Send + Sync + 'static {
        let retry_node = retry_node.to_string();
        move |state: &DefaultState| match state.metadata.get("review_passed") {
            Some(v) if v.as_bool() == Some(true) => None,
            _ => Some(retry_node.clone()),
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::Task;

    #[test]
    fn test_edge_always() {
        let edge = Edge::always("a", "b");
        assert_eq!(edge.from_node(), "a");
        assert_eq!(edge.to_node(), Some("b"));
    }

    #[test]
    fn test_edge_conditional() {
        let edge = Edge::conditional("a", |_| Some("b".into()), vec!["b", "__end__"]);
        assert_eq!(edge.from_node(), "a");
        assert!(edge.to_node().is_none());
    }

    #[test]
    fn test_router_metadata_eq_match() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("status".into(), serde_json::json!("done"));
        let router = routers::metadata_eq("status", "done", "retry");
        assert!(router(&state).is_none()); // END
    }

    #[test]
    fn test_router_metadata_eq_no_match() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("status".into(), serde_json::json!("working"));
        let router = routers::metadata_eq("status", "done", "retry");
        assert_eq!(router(&state), Some("retry".to_string()));
    }

    #[test]
    fn test_router_next_node() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("next_node".into(), serde_json::json!("coder"));
        let router = routers::next_node_router();
        assert_eq!(router(&state), Some("coder".to_string()));
    }

    #[test]
    fn test_router_next_node_none() {
        let state = DefaultState::new(Task::new("test"));
        let router = routers::next_node_router();
        assert!(router(&state).is_none());
    }

    #[test]
    fn test_router_max_iterations_under() {
        let state = DefaultState::new(Task::new("test"));
        let router = routers::max_iterations(5, "loop");
        assert_eq!(router(&state), Some("loop".to_string()));
    }

    #[test]
    fn test_router_max_iterations_exceeded() {
        let mut state = DefaultState::new(Task::new("test"));
        state.iteration = 5;
        let router = routers::max_iterations(5, "loop");
        assert!(router(&state).is_none());
    }

    #[test]
    fn test_router_review_passed() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("review_passed".into(), serde_json::json!(true));
        let router = routers::review_router("coder");
        assert!(router(&state).is_none()); // END
    }

    #[test]
    fn test_router_review_failed() {
        let state = DefaultState::new(Task::new("test"));
        let router = routers::review_router("coder");
        assert_eq!(router(&state), Some("coder".to_string()));
    }
}
