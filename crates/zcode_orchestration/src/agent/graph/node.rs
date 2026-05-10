//! Graph node definitions — LangGraph 风格
//!
//! Node 是图中的最小执行单元：接收可变 state，返回 `NodeOutput`（将被 reduce 进 state）。
//!
//! # 用法
//!
//! ```
//! let node = FnNode::new("planner", |state| {
//!     state.metadata.insert("plan".into(), serde_json::json!("step1"));
//!     Ok(NodeOutput::None)
//! });
//! ```

use async_trait::async_trait;

use crate::agent::graph::state::{DefaultState, NodeOutput};
use zcode_core::Result;

// ─── GraphNode trait ──────────────────────────────────────────────────────────

/// Trait implemented by every node in a `StateGraph`.
///
/// Nodes are the building blocks of LangGraph-style orchestration:
/// - They receive a mutable reference to the shared state
/// - They return a `NodeOutput` which is reduced into the state after execution
/// - They must be `Send + Sync` so the graph can be executed across threads
#[async_trait]
pub trait GraphNode: Send + Sync {
    /// Unique node name within the graph (used for routing and logging)
    fn name(&self) -> &str;

    /// Execute this node: read/modify state and return an output to reduce
    async fn execute(&self, state: &mut DefaultState) -> Result<NodeOutput>;
}

// ─── FnNode ── closure-based synchronous node ─────────────────────────────────

/// A lightweight node wrapping a **synchronous** closure.
///
/// This covers the majority of use cases where the node logic is CPU-bound
/// or does I/O within a blocking context.  For async I/O inside a node,
/// implement `GraphNode` directly on your own struct.
///
/// # Example
///
/// ```rust
/// let node = FnNode::new("scorer", |state| {
///     let score = compute_score(state);
///     Ok(NodeOutput::Custom("score".into(), serde_json::json!(score)))
/// });
/// ```
pub struct FnNode {
    name: String,
    f: Box<dyn Fn(&mut DefaultState) -> Result<NodeOutput> + Send + Sync>,
}

impl FnNode {
    /// Create a new `FnNode` from a closure.
    pub fn new<F>(name: impl Into<String>, f: F) -> Self
    where
        F: Fn(&mut DefaultState) -> Result<NodeOutput> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            f: Box::new(f),
        }
    }
}

#[async_trait]
impl GraphNode for FnNode {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, state: &mut DefaultState) -> Result<NodeOutput> {
        (self.f)(state)
    }
}

// ─── AsyncFnNode ── Future-based node ─────────────────────────────────────────

/// A node wrapping an **async** closure or future factory.
///
/// Use this when the node needs to perform real async I/O (e.g., LLM calls).
///
/// # Example
///
/// ```rust
/// let node = AsyncFnNode::new("llm_call", |state| async move {
///     let result = call_llm(&state.messages).await?;
///     Ok(NodeOutput::Messages(vec![ConversationMessage::assistant_text(result)]))
/// });
/// ```
pub struct AsyncFnNode {
    name: String,
    f: Box<
        dyn Fn(&mut DefaultState) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<NodeOutput>> + Send>>
            + Send
            + Sync,
    >,
}

impl AsyncFnNode {
    /// Create a new `AsyncFnNode` from an async closure factory.
    pub fn new<F, Fut>(name: impl Into<String>, f: F) -> Self
    where
        F: Fn(&mut DefaultState) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<NodeOutput>> + Send + 'static,
    {
        Self {
            name: name.into(),
            f: Box::new(move |state| Box::pin(f(state))),
        }
    }
}

#[async_trait]
impl GraphNode for AsyncFnNode {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, state: &mut DefaultState) -> Result<NodeOutput> {
        (self.f)(state).await
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::AgentState;

    // ── FnNode ──

    #[tokio::test]
    async fn test_fn_node_name() {
        let node = FnNode::new("my_node", |_s| Ok(NodeOutput::None));
        assert_eq!(node.name(), "my_node");
    }

    #[tokio::test]
    async fn test_fn_node_returns_none() {
        let node = FnNode::new("noop", |_s| Ok(NodeOutput::None));
        let mut state = DefaultState::default();
        let out = node.execute(&mut state).await.unwrap();
        assert!(matches!(out, NodeOutput::None));
    }

    #[tokio::test]
    async fn test_fn_node_mutates_state() {
        let node = FnNode::new("setter", |s| {
            s.agent_state = AgentState::Executing;
            Ok(NodeOutput::None)
        });
        let mut state = DefaultState::default();
        node.execute(&mut state).await.unwrap();
        assert_eq!(state.agent_state, AgentState::Executing);
    }

    #[tokio::test]
    async fn test_fn_node_returns_custom() {
        let node = FnNode::new("meta", |_s| {
            Ok(NodeOutput::Custom("answer".into(), serde_json::json!(42)))
        });
        let mut state = DefaultState::default();
        let out = node.execute(&mut state).await.unwrap();
        assert!(matches!(out, NodeOutput::Custom(_, _)));
    }

    #[tokio::test]
    async fn test_fn_node_propagates_error() {
        use zcode_core::ZcodeError;
        let node = FnNode::new("fail", |_s| {
            Err(ZcodeError::InternalError("node failed".into()))
        });
        let mut state = DefaultState::default();
        let result = node.execute(&mut state).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fn_node_sends_message() {
        use crate::agent::loop_exec::ConversationMessage;
        let node = FnNode::new("talker", |_s| {
            Ok(NodeOutput::Messages(vec![ConversationMessage::user("hello")]))
        });
        let mut state = DefaultState::default();
        let out = node.execute(&mut state).await.unwrap();
        if let NodeOutput::Messages(msgs) = out {
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0].role, "user");
        } else {
            panic!("Expected Messages output");
        }
    }

    // ── AsyncFnNode ──

    #[tokio::test]
    async fn test_async_fn_node_name() {
        let node = AsyncFnNode::new("async_node", |_s| async { Ok(NodeOutput::None) });
        assert_eq!(node.name(), "async_node");
    }

    #[tokio::test]
    async fn test_async_fn_node_execute() {
        let node = AsyncFnNode::new("async_setter", |s| {
            s.metadata.insert("async_ran".into(), serde_json::json!(true));
            async { Ok(NodeOutput::None) }
        });
        let mut state = DefaultState::default();
        node.execute(&mut state).await.unwrap();
        assert_eq!(
            state.metadata.get("async_ran").unwrap(),
            &serde_json::json!(true)
        );
    }

    // ── GraphNode object safety ──

    #[test]
    fn test_trait_object_fn_node() {
        let node: Box<dyn GraphNode> = Box::new(FnNode::new("obj", |_s| Ok(NodeOutput::None)));
        assert_eq!(node.name(), "obj");
    }

    #[test]
    fn test_trait_object_async_node() {
        let node: Box<dyn GraphNode> =
            Box::new(AsyncFnNode::new("async_obj", |_s| async { Ok(NodeOutput::None) }));
        assert_eq!(node.name(), "async_obj");
    }
}
