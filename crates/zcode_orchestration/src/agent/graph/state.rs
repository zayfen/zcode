//! Graph state definitions
//!
//! Defines the shared state that flows through the graph,
//! the node output type, and the state trait with reduce semantics.

use zcode_core::agent::ConversationMessage;
pub use zcode_core::agent::DefaultState;
use crate::agent::types::{AgentState, TaskResult};

// ─── NodeOutput ───────────────────────────────────────────────────────────────

/// Output from a node execution, reduced into the shared state.
#[derive(Debug, Clone)]
pub enum NodeOutput {
    /// Append messages to conversation history
    Messages(Vec<ConversationMessage>),
    /// Set the task result
    TaskResult(TaskResult),
    /// Update agent lifecycle state
    AgentState(AgentState),
    /// Set a custom metadata key-value pair
    Custom(String, serde_json::Value),
    /// Multiple outputs sequentially
    Multiple(Vec<NodeOutput>),
    /// No output (no-op)
    None,
}

// ─── GraphState trait ─────────────────────────────────────────────────────────

/// Trait for shared state that flows through the graph.
///
/// Implementors define how node outputs are merged (reduced) into state.
pub trait GraphState: Send + Sync {
    /// Merge a node output into this state (reducer pattern)
    fn reduce(&mut self, output: NodeOutput);

    /// Read a metadata value by key
    fn get_metadata(&self, key: &str) -> Option<&serde_json::Value>;

    /// Current iteration count
    fn iteration(&self) -> usize;

    /// Increment iteration counter
    fn inc_iteration(&mut self);
}

impl GraphState for DefaultState {
    fn reduce(&mut self, output: NodeOutput) {
        match output {
            NodeOutput::Messages(msgs) => {
                self.messages.extend(msgs);
            }
            NodeOutput::TaskResult(result) => {
                self.result = Some(result);
            }
            NodeOutput::AgentState(state) => {
                self.agent_state = state;
            }
            NodeOutput::Custom(key, value) => {
                self.metadata.insert(key, value);
            }
            NodeOutput::Multiple(outputs) => {
                for out in outputs {
                    self.reduce(out);
                }
            }
            NodeOutput::None => {}
        }
    }

    fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }

    fn iteration(&self) -> usize {
        self.iteration
    }

    fn inc_iteration(&mut self) {
        self.iteration += 1;
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::Task;

    #[test]
    fn test_default_state_new() {
        let task = Task::new("test task");
        let state = DefaultState::new(task);
        assert!(state.task.is_some());
        assert!(state.messages.is_empty());
        assert_eq!(state.agent_state, AgentState::Idle);
    }

    #[test]
    fn test_reduce_messages() {
        let mut state = DefaultState::default();
        state.reduce(NodeOutput::Messages(vec![
            ConversationMessage::user("hello"),
            ConversationMessage::assistant_text("hi"),
        ]));
        assert_eq!(state.messages.len(), 2);
    }

    #[test]
    fn test_reduce_task_result() {
        let mut state = DefaultState::default();
        state.reduce(NodeOutput::TaskResult(TaskResult::success("t1", "done")));
        assert!(state.result.is_some());
        assert!(state.result.as_ref().unwrap().success);
    }

    #[test]
    fn test_reduce_agent_state() {
        let mut state = DefaultState::default();
        state.reduce(NodeOutput::AgentState(AgentState::Executing));
        assert_eq!(state.agent_state, AgentState::Executing);
    }

    #[test]
    fn test_reduce_custom() {
        let mut state = DefaultState::default();
        state.reduce(NodeOutput::Custom("score".into(), serde_json::json!(42)));
        assert_eq!(state.get_metadata("score").unwrap(), &serde_json::json!(42));
    }

    #[test]
    fn test_reduce_none() {
        let mut state = DefaultState::default();
        state.reduce(NodeOutput::None);
        assert!(state.messages.is_empty());
    }

    #[test]
    fn test_iteration() {
        let mut state = DefaultState::default();
        assert_eq!(state.iteration(), 0);
        state.inc_iteration();
        assert_eq!(state.iteration(), 1);
    }

    #[test]
    fn test_builder_pattern() {
        let state = DefaultState::new(Task::new("fix bug"))
            .with_system_prompt("You are helpful")
            .with_metadata("key", serde_json::json!("value"));
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].role, "system");
    }
}
