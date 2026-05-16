//! StateGraph — graph-based agent orchestration
//!
//! Inspired by LangGraph: define nodes (async functions), connect them with
//! edges (unconditional or conditional), and execute them in order with
//! shared mutable state flowing through.
//!
//! # Quick start
//!
//! ```rust,ignore
//! let graph = StateGraph::new("planner")
//!     .add_node(FnNode::new("planner", |s| { ... }))
//!     .add_node(FnNode::new("coder",   |s| { ... }))
//!     .add_node(FnNode::new("reviewer",|s| { ... }))
//!     .add_edge("planner", "coder")
//!     .add_edge("coder", "reviewer")
//!     .add_conditional_edge("reviewer", review_router, vec!["coder", "__end__"])
//!     .compile()?;
//!
//! let output = graph.execute(&mut state).await?;
//! ```

use std::collections::HashMap;
use std::fmt;

use crate::agent::graph::edge::Edge;
use crate::agent::graph::node::GraphNode;
use crate::agent::graph::state::{DefaultState, GraphState};
use tracing::{debug, error, info, warn};
use zcode_core::{Result, ZcodeError};

// ─── EndReason ────────────────────────────────────────────────────────────────

/// Why the graph stopped executing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndReason {
    /// Reached a node with no outgoing edge (natural completion)
    NaturalEnd,
    /// Router returned `None` (explicit termination)
    ExplicitEnd,
    /// Safety limit reached
    MaxIterations,
    /// A node returned an error
    Error(String),
}

impl fmt::Display for EndReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EndReason::NaturalEnd => write!(f, "natural end"),
            EndReason::ExplicitEnd => write!(f, "explicit end"),
            EndReason::MaxIterations => write!(f, "max iterations reached"),
            EndReason::Error(e) => write!(f, "error: {}", e),
        }
    }
}

// ─── GraphOutput ──────────────────────────────────────────────────────────────

/// Result of graph execution
#[derive(Debug, Clone)]
pub struct GraphOutput {
    /// Ordered list of node names that were executed
    pub nodes_executed: Vec<String>,
    /// Total iterations through the execution loop
    pub total_iterations: usize,
    /// Why the graph ended
    pub end_reason: EndReason,
}

impl GraphOutput {
    fn new() -> Self {
        Self {
            nodes_executed: Vec::new(),
            total_iterations: 0,
            end_reason: EndReason::NaturalEnd,
        }
    }
}

// ─── GraphEvent ───────────────────────────────────────────────────────────────

/// Events emitted during graph execution
#[derive(Debug, Clone)]
pub enum GraphEvent {
    /// A node started executing
    NodeStart { node: String, iteration: usize },
    /// A node finished executing
    NodeComplete {
        node: String,
        output_summary: String,
    },
    /// An edge was traversed
    EdgeTraversed { from: String, to: Option<String> },
    /// A supervisor task step started.
    StepStart {
        id: String,
        title: String,
        agent: String,
    },
    /// A supervisor task step completed.
    StepComplete {
        id: String,
        title: String,
        agent: String,
        success: bool,
    },
    /// A tool started executing inside an agent node.
    ToolStart {
        agent: String,
        tool_name: String,
        command: String,
    },
    /// A tool finished executing inside an agent node.
    ToolComplete {
        agent: String,
        tool_name: String,
        success: bool,
    },
    /// Graph execution ended
    End {
        reason: EndReason,
        output: GraphOutput,
    },
}

impl fmt::Display for GraphEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphEvent::NodeStart { node, iteration } => {
                write!(f, "[NodeStart] {} (iteration {})", node, iteration)
            }
            GraphEvent::NodeComplete { node, .. } => {
                write!(f, "[NodeComplete] {}", node)
            }
            GraphEvent::EdgeTraversed { from, to } => match to {
                Some(t) => write!(f, "[Edge] {} → {}", from, t),
                None => write!(f, "[Edge] {} → END", from),
            },
            GraphEvent::StepStart { id, title, agent } => {
                write!(f, "[StepStart] {} {}: {}", agent, id, title)
            }
            GraphEvent::StepComplete {
                id,
                title,
                agent,
                success,
            } => {
                write!(
                    f,
                    "[StepComplete] {} {} {}: {}",
                    agent,
                    id,
                    if *success { "succeeded" } else { "failed" },
                    title
                )
            }
            GraphEvent::ToolStart {
                agent,
                tool_name,
                command,
            } => {
                write!(f, "[ToolStart] {} {}: {}", agent, tool_name, command)
            }
            GraphEvent::ToolComplete {
                agent,
                tool_name,
                success,
            } => {
                write!(
                    f,
                    "[ToolComplete] {} {} {}",
                    agent,
                    tool_name,
                    if *success { "succeeded" } else { "failed" }
                )
            }
            GraphEvent::End { reason, .. } => {
                write!(f, "[End] {}", reason)
            }
        }
    }
}

fn take_custom_events(state: &mut DefaultState, metadata_key: &str) -> Vec<GraphEvent> {
    let Some(value) = state.metadata.remove(metadata_key) else {
        return Vec::new();
    };
    let Some(events) = value.as_array() else {
        return Vec::new();
    };

    events
        .iter()
        .filter_map(|event| {
            let kind = event.get("kind").and_then(|value| value.as_str())?;
            let agent = event
                .get("agent")
                .and_then(|value| value.as_str())
                .unwrap_or("agent")
                .to_string();
            match kind {
                "tool_start" | "start" => Some(GraphEvent::ToolStart {
                    agent,
                    tool_name: event
                        .get("tool_name")
                        .and_then(|value| value.as_str())
                        .unwrap_or("tool")
                        .to_string(),
                    command: event
                        .get("command")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                }),
                "tool_complete" | "complete" => Some(GraphEvent::ToolComplete {
                    agent,
                    tool_name: event
                        .get("tool_name")
                        .and_then(|value| value.as_str())
                        .unwrap_or("tool")
                        .to_string(),
                    success: event
                        .get("success")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                }),
                "step_start" => Some(GraphEvent::StepStart {
                    id: event
                        .get("id")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                    title: event
                        .get("title")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                    agent,
                }),
                "step_complete" => Some(GraphEvent::StepComplete {
                    id: event
                        .get("id")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                    title: event
                        .get("title")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                    agent,
                    success: event
                        .get("success")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                }),
                _ => None,
            }
        })
        .collect()
}

fn take_graph_events(state: &mut DefaultState) -> Vec<GraphEvent> {
    let mut events = take_custom_events(state, "__step_events");
    events.extend(take_custom_events(state, "__tool_events"));
    events
}

// ─── StateGraph builder ──────────────────────────────────────────────────────

/// Build a state graph by adding nodes and edges, then compile and execute.
pub struct StateGraph {
    nodes: HashMap<String, Box<dyn GraphNode>>,
    edges: Vec<Edge>,
    entry: String,
    max_iterations: usize,
    graph_id: String,
}

impl StateGraph {
    /// Create a new graph with the given entry node name
    pub fn new(entry: impl Into<String>) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            entry: entry.into(),
            max_iterations: 50,
            graph_id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
        }
    }

    /// Set a custom graph ID (for checkpointing)
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.graph_id = id.into();
        self
    }

    /// Set max iterations (safety limit). Default: 50
    pub fn max_iterations(mut self, n: usize) -> Self {
        self.max_iterations = n;
        self
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: impl GraphNode + 'static) -> &mut Self {
        let name = node.name().to_string();
        self.nodes.insert(name, Box::new(node));
        self
    }

    /// Add an unconditional edge: `from` → `to`
    pub fn add_edge(&mut self, from: &str, to: &str) -> &mut Self {
        self.edges.push(Edge::always(from, to));
        self
    }

    /// Add a conditional edge: `from` → router(state) → next node or END
    pub fn add_conditional_edge(
        &mut self,
        from: &str,
        router: impl Fn(&DefaultState) -> Option<String> + Send + Sync + 'static,
        branches: Vec<&str>,
    ) -> &mut Self {
        self.edges.push(Edge::conditional(from, router, branches));
        self
    }

    /// Compile the graph (validates structure). Returns a `CompiledGraph`.
    pub fn compile(self) -> Result<CompiledGraph> {
        // Validate entry node exists
        if !self.nodes.contains_key(&self.entry) {
            return Err(ZcodeError::InternalError(format!(
                "Entry node '{}' not found in graph",
                self.entry
            )));
        }

        // Validate edge sources exist
        for edge in &self.edges {
            let from = edge.from_node();
            if !self.nodes.contains_key(from) {
                return Err(ZcodeError::InternalError(format!(
                    "Edge source node '{}' not found in graph",
                    from
                )));
            }
        }

        Ok(CompiledGraph {
            nodes: self.nodes,
            edges: self.edges,
            entry: self.entry,
            max_iterations: self.max_iterations,
            graph_id: self.graph_id,
        })
    }

    /// Generate a DOT representation of the graph (before compile)
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph state_graph {\n");
        dot.push_str("    rankdir=LR;\n");

        // Entry node
        dot.push_str(&format!(
            "    {} [shape=box, style=filled, fillcolor=lightblue];\n",
            self.entry
        ));

        // All nodes
        for name in self.nodes.keys() {
            if name != &self.entry {
                dot.push_str(&format!("    {};\n", name));
            }
        }

        // Edges
        for edge in &self.edges {
            match edge {
                Edge::Always { from, to } => {
                    dot.push_str(&format!("    {} -> {};\n", from, to));
                }
                Edge::Conditional { from, branches, .. } => {
                    for branch in branches {
                        if branch == "__end__" {
                            dot.push_str(&format!("    {} -> __end__ [style=dashed];\n", from));
                        } else {
                            dot.push_str(&format!("    {} -> {} [style=dashed];\n", from, branch));
                        }
                    }
                }
            }
        }

        dot.push_str("    __end__ [shape=doublecircle];\n");
        dot.push_str("}\n");
        dot
    }
}

// ─── CompiledGraph ────────────────────────────────────────────────────────────

/// An immutable, validated graph ready for execution.
pub struct CompiledGraph {
    nodes: HashMap<String, Box<dyn GraphNode>>,
    edges: Vec<Edge>,
    entry: String,
    max_iterations: usize,
    graph_id: String,
}

impl CompiledGraph {
    /// Execute the graph with the given state (async)
    pub async fn execute(&self, state: &mut DefaultState) -> Result<GraphOutput> {
        self.execute_with_events(state, |_| {}).await
    }

    /// Execute the graph, calling `on_event` for each event
    pub async fn execute_with_events<F>(
        &self,
        state: &mut DefaultState,
        mut on_event: F,
    ) -> Result<GraphOutput>
    where
        F: FnMut(GraphEvent),
    {
        self.execute_with_events_and_cancel(state, |event| {
            on_event(event);
            false
        })
        .await
    }

    /// Execute the graph, calling `on_event` for each event and stopping before
    /// the next node when the callback returns `true`.
    pub async fn execute_with_events_and_cancel<F>(
        &self,
        state: &mut DefaultState,
        mut on_event: F,
    ) -> Result<GraphOutput>
    where
        F: FnMut(GraphEvent) -> bool,
    {
        let mut output = GraphOutput::new();
        let mut current = self.entry.clone();
        info!(graph_id = %self.graph_id, entry = %current, max_iter = self.max_iterations, "Graph execution starting");

        loop {
            // Safety: max iterations
            if output.total_iterations >= self.max_iterations {
                warn!(graph_id = %self.graph_id, iterations = output.total_iterations, "Graph hit max iterations safety limit");
                output.end_reason = EndReason::MaxIterations;
                let _ = on_event(GraphEvent::End {
                    reason: output.end_reason.clone(),
                    output: output.clone(),
                });
                return Ok(output);
            }

            // Execute current node
            let node = self.nodes.get(&current).ok_or_else(|| {
                ZcodeError::InternalError(format!("Node '{}' not found", current))
            })?;

            info!(graph_id = %self.graph_id, node = %current, iteration = output.total_iterations, "Executing node");
            if on_event(GraphEvent::NodeStart {
                node: current.clone(),
                iteration: output.total_iterations,
            }) {
                output.end_reason = EndReason::ExplicitEnd;
                let _ = on_event(GraphEvent::End {
                    reason: output.end_reason.clone(),
                    output: output.clone(),
                });
                return Ok(output);
            }

            state.metadata.insert(
                "__active_graph_node".to_string(),
                serde_json::Value::String(current.clone()),
            );
            let result = node.execute(state).await;
            state.metadata.remove("__active_graph_node");
            match result {
                Ok(node_output) => {
                    let summary = format!("{:?}", node_output);
                    debug!(graph_id = %self.graph_id, node = %current, "Node output: {}", &summary[..summary.len().min(200)]);
                    state.reduce(node_output);
                    for event in take_graph_events(state) {
                        if on_event(event) {
                            output.end_reason = EndReason::ExplicitEnd;
                            let _ = on_event(GraphEvent::End {
                                reason: output.end_reason.clone(),
                                output: output.clone(),
                            });
                            return Ok(output);
                        }
                    }
                    state.inc_iteration();
                    output.nodes_executed.push(current.clone());
                    output.total_iterations += 1;
                    info!(graph_id = %self.graph_id, node = %current, "Node completed (iteration {})", output.total_iterations);

                    if on_event(GraphEvent::NodeComplete {
                        node: current.clone(),
                        output_summary: summary,
                    }) {
                        output.end_reason = EndReason::ExplicitEnd;
                        let _ = on_event(GraphEvent::End {
                            reason: output.end_reason.clone(),
                            output: output.clone(),
                        });
                        return Ok(output);
                    }
                }
                Err(e) => {
                    error!(graph_id = %self.graph_id, node = %current, error = %e, "Node failed");
                    output.end_reason = EndReason::Error(e.to_string());
                    let _ = on_event(GraphEvent::End {
                        reason: output.end_reason.clone(),
                        output: output.clone(),
                    });
                    return Err(e);
                }
            }

            // Find next node via edges
            let next = self.resolve_next(&current, state);
            match next {
                Some(target) => {
                    info!(graph_id = %self.graph_id, from = %current, to = %target, "Edge traversed");
                    if on_event(GraphEvent::EdgeTraversed {
                        from: current.clone(),
                        to: Some(target.clone()),
                    }) {
                        output.end_reason = EndReason::ExplicitEnd;
                        let _ = on_event(GraphEvent::End {
                            reason: output.end_reason.clone(),
                            output: output.clone(),
                        });
                        return Ok(output);
                    }
                    current = target;
                }
                None => {
                    let _ = on_event(GraphEvent::EdgeTraversed {
                        from: current.clone(),
                        to: None,
                    });
                    output.end_reason = if self.has_edge_from(&current) {
                        EndReason::ExplicitEnd
                    } else {
                        EndReason::NaturalEnd
                    };
                    info!(
                        graph_id = %self.graph_id,
                        reason = %output.end_reason,
                        total_iterations = output.total_iterations,
                        nodes = ?output.nodes_executed,
                        "Graph execution finished"
                    );
                    let _ = on_event(GraphEvent::End {
                        reason: output.end_reason.clone(),
                        output: output.clone(),
                    });
                    return Ok(output);
                }
            }
        }
    }

    /// Get the graph ID
    pub fn graph_id(&self) -> &str {
        &self.graph_id
    }

    /// Get node names
    pub fn node_names(&self) -> Vec<&str> {
        self.nodes.keys().map(|s| s.as_str()).collect()
    }

    /// Generate DOT representation
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph state_graph {\n");
        dot.push_str("    rankdir=LR;\n");
        dot.push_str(&format!(
            "    {} [shape=box, style=filled, fillcolor=lightblue];\n",
            self.entry
        ));
        for name in self.nodes.keys() {
            if name != &self.entry {
                dot.push_str(&format!("    {};\n", name));
            }
        }
        for edge in &self.edges {
            match edge {
                Edge::Always { from, to } => {
                    dot.push_str(&format!("    {} -> {};\n", from, to));
                }
                Edge::Conditional { from, branches, .. } => {
                    for branch in branches {
                        if branch == "__end__" {
                            dot.push_str(&format!("    {} -> __end__ [style=dashed];\n", from));
                        } else {
                            dot.push_str(&format!("    {} -> {} [style=dashed];\n", from, branch));
                        }
                    }
                }
            }
        }
        dot.push_str("    __end__ [shape=doublecircle];\n");
        dot.push_str("}\n");
        dot
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    /// Resolve the next node from `from_node` given current state
    fn resolve_next(&self, from_node: &str, state: &DefaultState) -> Option<String> {
        for edge in &self.edges {
            if edge.from_node() != from_node {
                continue;
            }
            match edge {
                Edge::Always { to, .. } => return Some(to.clone()),
                Edge::Conditional { router, .. } => return router(state),
            }
        }
        None
    }

    /// Check if any edge originates from `from_node`
    fn has_edge_from(&self, from_node: &str) -> bool {
        self.edges.iter().any(|e| e.from_node() == from_node)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::graph::node::FnNode;
    use crate::agent::graph::state::NodeOutput;
    use crate::agent::loop_exec::ConversationMessage;
    use crate::agent::types::{AgentState, TaskResult};

    // ── Helper constructors ──

    fn ok_node(name: &str, output: NodeOutput) -> FnNode {
        let output = std::sync::Arc::new(std::sync::Mutex::new(output));
        FnNode::new(name, move |_s| Ok(output.lock().unwrap().clone()))
    }

    fn state_changer_node(name: &str, new_state: AgentState) -> FnNode {
        FnNode::new(name, move |_s| Ok(NodeOutput::AgentState(new_state)))
    }

    // ── Basic execution ──

    #[tokio::test]
    async fn test_single_node() {
        let mut g = StateGraph::new("only");
        g.add_node(ok_node("only", NodeOutput::None));
        let cg = g.compile().unwrap();

        let mut state = DefaultState::default();
        let out = cg.execute(&mut state).await.unwrap();
        assert_eq!(out.nodes_executed, vec!["only"]);
        assert_eq!(out.end_reason, EndReason::NaturalEnd);
    }

    #[tokio::test]
    async fn test_linear_chain() {
        let mut g = StateGraph::new("a");
        g.add_node(ok_node("a", NodeOutput::None));
        g.add_node(ok_node("b", NodeOutput::None));
        g.add_node(ok_node("c", NodeOutput::None));
        g.add_edge("a", "b");
        g.add_edge("b", "c");
        let cg = g.compile().unwrap();

        let mut state = DefaultState::default();
        let out = cg.execute(&mut state).await.unwrap();
        assert_eq!(out.nodes_executed, vec!["a", "b", "c"]);
        assert_eq!(out.total_iterations, 3);
    }

    #[tokio::test]
    async fn test_conditional_branch_yes() {
        let mut g = StateGraph::new("start");
        g.add_node(FnNode::new("start", |s| {
            s.metadata.insert("go".into(), serde_json::json!("yes"));
            Ok(NodeOutput::None)
        }));
        g.add_node(ok_node("yes_node", NodeOutput::None));
        g.add_node(ok_node("no_node", NodeOutput::None));
        g.add_conditional_edge(
            "start",
            |s| match s.metadata.get("go").and_then(|v| v.as_str()) {
                Some("yes") => Some("yes_node".into()),
                _ => Some("no_node".into()),
            },
            vec!["yes_node", "no_node"],
        );
        let cg = g.compile().unwrap();

        let mut state = DefaultState::default();
        let out = cg.execute(&mut state).await.unwrap();
        assert_eq!(out.nodes_executed, vec!["start", "yes_node"]);
    }

    #[tokio::test]
    async fn test_conditional_branch_no() {
        let mut g = StateGraph::new("start");
        g.add_node(FnNode::new("start", |s| {
            s.metadata.insert("go".into(), serde_json::json!("no"));
            Ok(NodeOutput::None)
        }));
        g.add_node(ok_node("yes_node", NodeOutput::None));
        g.add_node(ok_node("no_node", NodeOutput::None));
        g.add_conditional_edge(
            "start",
            |s| match s.metadata.get("go").and_then(|v| v.as_str()) {
                Some("yes") => Some("yes_node".into()),
                _ => Some("no_node".into()),
            },
            vec!["yes_node", "no_node"],
        );
        let cg = g.compile().unwrap();

        let mut state = DefaultState::default();
        let out = cg.execute(&mut state).await.unwrap();
        assert_eq!(out.nodes_executed, vec!["start", "no_node"]);
    }

    // ── Cycle (loop) ──

    #[tokio::test]
    async fn test_cycle_graph() {
        let mut g = StateGraph::new("counter").max_iterations(10);
        g.add_node(FnNode::new("counter", |s| {
            let count = s
                .metadata
                .get("count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            s.metadata
                .insert("count".into(), serde_json::json!(count + 1));
            Ok(NodeOutput::None)
        }));
        g.add_conditional_edge(
            "counter",
            |s| {
                let count = s
                    .metadata
                    .get("count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                if count >= 3 {
                    None // END
                } else {
                    Some("counter".into())
                }
            },
            vec!["counter", "__end__"],
        );
        let cg = g.compile().unwrap();

        let mut state = DefaultState::default();
        let out = cg.execute(&mut state).await.unwrap();
        assert_eq!(out.total_iterations, 3);
        assert_eq!(state.metadata.get("count").unwrap(), &serde_json::json!(3));
        assert_eq!(out.end_reason, EndReason::ExplicitEnd);
    }

    #[tokio::test]
    async fn test_max_iterations() {
        let mut g = StateGraph::new("loop").max_iterations(3);
        g.add_node(ok_node("loop", NodeOutput::None));
        g.add_edge("loop", "loop"); // infinite loop
        let cg = g.compile().unwrap();

        let mut state = DefaultState::default();
        let out = cg.execute(&mut state).await.unwrap();
        assert_eq!(out.end_reason, EndReason::MaxIterations);
        assert_eq!(out.total_iterations, 3);
    }

    // ── State propagation ──

    #[tokio::test]
    async fn test_state_flows_between_nodes() {
        let mut g = StateGraph::new("set");
        g.add_node(FnNode::new("set", |s| {
            s.metadata.insert("value".into(), serde_json::json!(42));
            Ok(NodeOutput::None)
        }));
        g.add_node(FnNode::new("check", |s| {
            let v = s.metadata.get("value").cloned();
            Ok(NodeOutput::Custom(
                "received".into(),
                v.unwrap_or(serde_json::Value::Null),
            ))
        }));
        g.add_edge("set", "check");
        let cg = g.compile().unwrap();

        let mut state = DefaultState::default();
        cg.execute(&mut state).await.unwrap();
        assert_eq!(state.metadata.get("value").unwrap(), &serde_json::json!(42));
        assert_eq!(
            state.metadata.get("received").unwrap(),
            &serde_json::json!(42)
        );
    }

    #[tokio::test]
    async fn test_messages_accumulate() {
        let mut g = StateGraph::new("a");
        g.add_node(FnNode::new("a", |_s| {
            Ok(NodeOutput::Messages(vec![ConversationMessage::user(
                "hello",
            )]))
        }));
        g.add_node(FnNode::new("b", |_s| {
            Ok(NodeOutput::Messages(vec![
                ConversationMessage::assistant_text("world"),
            ]))
        }));
        g.add_edge("a", "b");
        let cg = g.compile().unwrap();

        let mut state = DefaultState::default();
        cg.execute(&mut state).await.unwrap();
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.messages[0].role, "user");
        assert_eq!(state.messages[1].role, "assistant");
    }

    // ── Events ──

    #[tokio::test]
    async fn test_execute_with_events() {
        let mut g = StateGraph::new("a");
        g.add_node(state_changer_node("a", AgentState::Executing));
        g.add_node(state_changer_node("b", AgentState::Completed));
        g.add_edge("a", "b");
        let cg = g.compile().unwrap();

        let mut events: Vec<String> = Vec::new();
        let mut state = DefaultState::default();
        cg.execute_with_events(&mut state, |e| events.push(format!("{}", e)))
            .await
            .unwrap();

        assert!(events.iter().any(|e| e.contains("NodeStart")));
        assert!(events.iter().any(|e| e.contains("NodeComplete")));
        assert!(events.iter().any(|e| e.contains("End")));
    }

    // ── Compilation validation ──

    #[test]
    fn test_compile_missing_entry_node() {
        let mut g = StateGraph::new("missing");
        g.add_node(ok_node("other", NodeOutput::None));
        assert!(g.compile().is_err());
    }

    #[test]
    fn test_compile_missing_edge_source() {
        let mut g = StateGraph::new("a");
        g.add_node(ok_node("a", NodeOutput::None));
        g.add_edge("ghost", "a"); // ghost doesn't exist
        assert!(g.compile().is_err());
    }

    // ── DOT output ──

    #[test]
    fn test_to_dot() {
        let mut g = StateGraph::new("a");
        g.add_node(ok_node("a", NodeOutput::None));
        g.add_node(ok_node("b", NodeOutput::None));
        g.add_edge("a", "b");
        g.add_conditional_edge("b", |_| None, vec!["__end__"]);
        let dot = g.to_dot();
        assert!(dot.contains("digraph"));
        assert!(dot.contains("a -> b"));
        assert!(dot.contains("style=dashed"));
    }

    // ── Node output types ──

    #[tokio::test]
    async fn test_task_result_output() {
        let mut g = StateGraph::new("worker");
        g.add_node(FnNode::new("worker", |_s| {
            Ok(NodeOutput::TaskResult(TaskResult::success(
                "t1", "all done",
            )))
        }));
        let cg = g.compile().unwrap();

        let mut state = DefaultState::default();
        cg.execute(&mut state).await.unwrap();
        assert!(state.result.is_some());
        let r = state.result.unwrap();
        assert!(r.success);
        assert_eq!(r.output, "all done");
    }
}
