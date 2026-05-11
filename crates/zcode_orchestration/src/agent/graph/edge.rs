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

    /// Router: check `test_passed` metadata — pass → pass_node, fail → retry_node
    /// If `coder_retries` >= `max_retries`, force → pass_node (skip retry)
    pub fn test_router(
        retry_node: &str,
        pass_node: &str,
        max_retries: u64,
    ) -> impl Fn(&DefaultState) -> Option<String> + Send + Sync + 'static {
        let retry_node = retry_node.to_string();
        let pass_node = pass_node.to_string();
        move |state: &DefaultState| {
            let retries = state.metadata.get("coder_retries")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            
            match state.metadata.get("test_passed") {
                Some(v) if v.as_bool() == Some(true) => {
                    tracing::info!("[test_router] Test PASSED → {}", pass_node);
                    Some(pass_node.clone())
                }
                _ if retries >= max_retries => {
                    tracing::warn!("[test_router] Coder retries exhausted ({}/{}) → forcing {}", retries, max_retries, pass_node);
                    Some(pass_node.clone())
                }
                _ => {
                    tracing::info!("[test_router] Test FAILED (retry {}/{}) → {}", retries, max_retries, retry_node);
                    Some(retry_node.clone())
                }
            }
        }
    }

    /// Router: check `test_passed` metadata — pass AND `is_simple` → fast_end_node
    /// pass AND NOT `is_simple` → pass_node
    /// fail → retry_node
    /// If `coder_retries` >= `max_retries`, force → pass_node (or fast_end_node if simple)
    pub fn test_fastpath_router(
        retry_node: &str,
        pass_node: &str,
        fast_end_node: &str,
        max_retries: u64,
    ) -> impl Fn(&DefaultState) -> Option<String> + Send + Sync + 'static {
        let retry_node = retry_node.to_string();
        let pass_node = pass_node.to_string();
        let fast_end_node = fast_end_node.to_string();
        move |state: &DefaultState| {
            let retries = state.metadata.get("coder_retries")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            
            let is_simple = state.metadata.get("is_simple").and_then(|v| v.as_bool()).unwrap_or(false);
            let target_node = if is_simple { fast_end_node.clone() } else { pass_node.clone() };

            match state.metadata.get("test_passed") {
                Some(v) if v.as_bool() == Some(true) => {
                    if is_simple {
                        tracing::info!("[test_fastpath_router] Test PASSED & FAST_PATH → {}", target_node);
                        None // __end__ is represented by None
                    } else {
                        tracing::info!("[test_fastpath_router] Test PASSED → {}", target_node);
                        Some(target_node)
                    }
                }
                _ if retries >= max_retries => {
                    if is_simple {
                        tracing::warn!("[test_fastpath_router] Coder retries exhausted & FAST_PATH → forcing END");
                        None
                    } else {
                        tracing::warn!("[test_fastpath_router] Coder retries exhausted ({}/{}) → forcing {}", retries, max_retries, target_node);
                        Some(target_node)
                    }
                }
                _ => {
                    tracing::info!("[test_fastpath_router] Test FAILED (retry {}/{}) → {}", retries, max_retries, retry_node);
                    Some(retry_node.clone())
                }
            }
        }
    }

    /// Router for per-task test-fix loop.
    ///
    /// After each coder run, the reviewer/test gate checks the produced code.
    /// - `test_passed = true`  → `None` (task done, exit to END)
    /// - `test_passed = false` AND `coder_retries < max_retries` → `Some(retry_node)` (back to coder with failure info)
    /// - `test_passed = false` AND `coder_retries >= max_retries` → `None` (retries exhausted, force END)
    ///
    /// The `coder_retries` counter must be incremented by the coder node itself each time it runs.
    pub fn task_test_router(
        retry_node: &str,
        max_retries: u64,
    ) -> impl Fn(&DefaultState) -> Option<String> + Send + Sync + 'static {
        let retry_node = retry_node.to_string();
        move |state: &DefaultState| {
            let retries = state.metadata.get("coder_retries")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            match state.metadata.get("test_passed") {
                Some(v) if v.as_bool() == Some(true) => {
                    tracing::info!("[task_test_router] Test PASSED → END (task complete)");
                    None // END — task successfully verified
                }
                _ if retries >= max_retries => {
                    tracing::warn!(
                        "[task_test_router] Test FAILED but retries exhausted ({}/{}) → forcing END",
                        retries, max_retries
                    );
                    None // END — give up after max retries
                }
                _ => {
                    tracing::info!(
                        "[task_test_router] Test FAILED (retry {}/{}) → {}",
                        retries, max_retries, retry_node
                    );
                    Some(retry_node.clone())
                }
            }
        }
    }

    /// Router: check `review_passed` metadata — pass → END, fail → retry_node
    /// If `coder_retries` >= `max_retries`, force → END (skip retry)
    pub fn review_router_with_limit(
        retry_node: &str,
        max_retries: u64,
    ) -> impl Fn(&DefaultState) -> Option<String> + Send + Sync + 'static {
        let retry_node = retry_node.to_string();
        move |state: &DefaultState| {
            let retries = state.metadata.get("coder_retries")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            
            match state.metadata.get("review_passed") {
                Some(v) if v.as_bool() == Some(true) => {
                    tracing::info!("[review_router] Review PASSED → END");
                    None
                }
                _ if retries >= max_retries => {
                    tracing::warn!("[review_router] Coder retries exhausted ({}/{}) → forcing END", retries, max_retries);
                    None
                }
                _ => {
                    tracing::info!("[review_router] Review FAILED (retry {}/{}) → {}", retries, max_retries, retry_node);
                    Some(retry_node.clone())
                }
            }
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

    // ============================================================
    // test_router tests
    // ============================================================

    #[test]
    fn test_test_router_passed() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("test_passed".into(), serde_json::json!(true));
        let router = routers::test_router("coder", "reviewer", 3);
        assert_eq!(router(&state), Some("reviewer".to_string()));
    }

    #[test]
    fn test_test_router_failed_first_attempt() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("test_passed".into(), serde_json::json!(false));
        state.metadata.insert("coder_retries".into(), serde_json::json!(1));
        let router = routers::test_router("coder", "reviewer", 3);
        assert_eq!(router(&state), Some("coder".to_string())); // retry
    }

    #[test]
    fn test_test_router_failed_retries_exhausted() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("test_passed".into(), serde_json::json!(false));
        state.metadata.insert("coder_retries".into(), serde_json::json!(3));
        let router = routers::test_router("coder", "reviewer", 3);
        assert_eq!(router(&state), Some("reviewer".to_string())); // forced forward
    }

    #[test]
    fn test_test_router_no_metadata_first_run() {
        let state = DefaultState::new(Task::new("test"));
        // No test_passed, no coder_retries → fail + retries=0 < 3 → retry
        let router = routers::test_router("coder", "reviewer", 3);
        assert_eq!(router(&state), Some("coder".to_string()));
    }

    #[test]
    fn test_test_router_passed_even_with_high_retries() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("test_passed".into(), serde_json::json!(true));
        state.metadata.insert("coder_retries".into(), serde_json::json!(99));
        let router = routers::test_router("coder", "reviewer", 3);
        // Pass always wins regardless of retry count
        assert_eq!(router(&state), Some("reviewer".to_string()));
    }

    #[test]
    fn test_test_router_at_exact_limit() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("test_passed".into(), serde_json::json!(false));
        state.metadata.insert("coder_retries".into(), serde_json::json!(3)); // exactly at limit
        let router = routers::test_router("coder", "reviewer", 3);
        assert_eq!(router(&state), Some("reviewer".to_string())); // forced forward
    }

    #[test]
    fn test_test_router_one_below_limit() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("test_passed".into(), serde_json::json!(false));
        state.metadata.insert("coder_retries".into(), serde_json::json!(2)); // below limit
        let router = routers::test_router("coder", "reviewer", 3);
        assert_eq!(router(&state), Some("coder".to_string())); // still retries
    }

    // ============================================================
    // review_router_with_limit tests
    // ============================================================

    #[test]
    fn test_review_router_with_limit_passed() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("review_passed".into(), serde_json::json!(true));
        let router = routers::review_router_with_limit("coder", 3);
        assert!(router(&state).is_none()); // END
    }

    #[test]
    fn test_review_router_with_limit_failed_first() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("review_passed".into(), serde_json::json!(false));
        state.metadata.insert("coder_retries".into(), serde_json::json!(1));
        let router = routers::review_router_with_limit("coder", 3);
        assert_eq!(router(&state), Some("coder".to_string())); // retry
    }

    #[test]
    fn test_review_router_with_limit_retries_exhausted() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("review_passed".into(), serde_json::json!(false));
        state.metadata.insert("coder_retries".into(), serde_json::json!(3));
        let router = routers::review_router_with_limit("coder", 3);
        assert!(router(&state).is_none()); // forced END
    }

    #[test]
    fn test_review_router_with_limit_no_metadata() {
        let state = DefaultState::new(Task::new("test"));
        // No review_passed → fail, no coder_retries → 0 < 3 → retry
        let router = routers::review_router_with_limit("coder", 3);
        assert_eq!(router(&state), Some("coder".to_string()));
    }

    #[test]
    fn test_review_router_with_limit_passed_ignores_retries() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("review_passed".into(), serde_json::json!(true));
        state.metadata.insert("coder_retries".into(), serde_json::json!(99));
        let router = routers::review_router_with_limit("coder", 3);
        assert!(router(&state).is_none()); // Pass always wins
    }

    #[test]
    fn test_review_router_with_limit_max_retries_zero() {
        // max_retries=0 means never retry
        let state = DefaultState::new(Task::new("test"));
        let router = routers::review_router_with_limit("coder", 0);
        assert!(router(&state).is_none()); // retries(0) >= 0 → forced END
    }

    #[test]
    fn test_test_router_max_retries_zero() {
        // max_retries=0 means never retry
        let state = DefaultState::new(Task::new("test"));
        let router = routers::test_router("coder", "reviewer", 0);
        assert_eq!(router(&state), Some("reviewer".to_string())); // retries(0) >= 0 → forced forward
    }

    // ============================================================
    // task_test_router tests
    // ============================================================

    #[test]
    fn test_task_test_router_pass() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("test_passed".into(), serde_json::json!(true));
        let router = routers::task_test_router("coder", 3);
        assert!(router(&state).is_none()); // PASS → END
    }

    #[test]
    fn test_task_test_router_fail_retry() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("test_passed".into(), serde_json::json!(false));
        state.metadata.insert("coder_retries".into(), serde_json::json!(1u64));
        let router = routers::task_test_router("coder", 3);
        assert_eq!(router(&state), Some("coder".to_string())); // FAIL + retries < 3 → retry
    }

    #[test]
    fn test_task_test_router_fail_exhausted() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("test_passed".into(), serde_json::json!(false));
        state.metadata.insert("coder_retries".into(), serde_json::json!(3u64));
        let router = routers::task_test_router("coder", 3);
        assert!(router(&state).is_none()); // FAIL + retries >= 3 → force END
    }

    #[test]
    fn test_task_test_router_no_metadata_first_run() {
        // No test_passed, no coder_retries → fail + retries(0) < 3 → retry
        let state = DefaultState::new(Task::new("test"));
        let router = routers::task_test_router("coder", 3);
        assert_eq!(router(&state), Some("coder".to_string()));
    }

    #[test]
    fn test_task_test_router_max_retries_zero() {
        // max_retries=0 → always forced END even on first failure
        let state = DefaultState::new(Task::new("test"));
        let router = routers::task_test_router("coder", 0);
        assert!(router(&state).is_none()); // retries(0) >= 0 → force END
    }

    #[test]
    fn test_task_test_router_pass_ignores_retry_count() {
        let mut state = DefaultState::new(Task::new("test"));
        state.metadata.insert("test_passed".into(), serde_json::json!(true));
        state.metadata.insert("coder_retries".into(), serde_json::json!(99u64));
        let router = routers::task_test_router("coder", 3);
        assert!(router(&state).is_none()); // PASS always wins → END
    }
}
