//! Agent trait definitions
//!
//! Defines the `AgentTrait` that all agents must implement.

use crate::agent::types::{AgentId, AgentMessage, AgentState, AgentType};
use crate::error::Result;
use async_trait::async_trait;

// ─── AgentTrait ────────────────────────────────────────────────────────────────

/// Core trait that all agents must implement
#[async_trait]
pub trait AgentTrait: Send + Sync {
    /// Unique identifier for this agent instance
    fn id(&self) -> &AgentId;

    /// The type/role of this agent
    fn agent_type(&self) -> AgentType;

    /// Current state in the lifecycle state machine
    fn state(&self) -> AgentState;

    /// Handle an incoming message and optionally produce a response message
    async fn handle(&mut self, message: AgentMessage) -> Result<Option<AgentMessage>>;

    /// Reset agent to idle state (for reuse)
    async fn reset(&mut self) -> Result<()>;

    // NOTE: generate_plan is not implemented here because it requires
    // concrete implementations of transition_to and explore_project
    // which are agent-specific. Implement it in each concrete agent.

    /// Human-readable name for display
    fn display_name(&self) -> String {
        let id_str = &self.id().0;
        let short = if id_str.len() > 8 { &id_str[..8] } else { id_str.as_str() };
        format!("{}-{}", self.agent_type(), short)
    }

    /// Whether this agent is currently busy (not idle or terminal)
    fn is_busy(&self) -> bool {
        !matches!(
            self.state(),
            AgentState::Idle | AgentState::Completed | AgentState::Failed
        )
    }

    /// Whether this agent has finished (terminal state)
    fn is_done(&self) -> bool {
        self.state().is_terminal()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::*;

    /// A minimal test agent implementation
    struct MockAgent {
        id: AgentId,
        state: AgentState,
    }

    impl MockAgent {
        fn new(state: AgentState) -> Self {
            Self {
                id: AgentId::named("mock"),
                state,
            }
        }
    }

    #[async_trait]
    impl AgentTrait for MockAgent {
        fn id(&self) -> &AgentId {
            &self.id
        }

        fn agent_type(&self) -> AgentType {
            AgentType::Coder
        }

        fn state(&self) -> AgentState {
            self.state
        }

        async fn handle(&mut self, _message: AgentMessage) -> Result<Option<AgentMessage>> {
            self.state = AgentState::Executing;
            Ok(None)
        }

        async fn reset(&mut self) -> Result<()> {
            self.state = AgentState::Idle;
            Ok(())
        }
    }

    #[test]
    fn test_agent_trait_id() {
        let agent = MockAgent::new(AgentState::Idle);
        assert_eq!(agent.id().0, "mock");
    }

    #[test]
    fn test_agent_trait_type() {
        let agent = MockAgent::new(AgentState::Idle);
        assert_eq!(agent.agent_type(), AgentType::Coder);
    }

    #[test]
    fn test_agent_trait_is_busy_idle() {
        let agent = MockAgent::new(AgentState::Idle);
        assert!(!agent.is_busy());
    }

    #[test]
    fn test_agent_trait_is_busy_executing() {
        let agent = MockAgent::new(AgentState::Executing);
        assert!(agent.is_busy());
    }

    #[test]
    fn test_agent_trait_is_busy_planning() {
        let agent = MockAgent::new(AgentState::Planning);
        assert!(agent.is_busy());
    }

    #[test]
    fn test_agent_trait_is_done_completed() {
        let agent = MockAgent::new(AgentState::Completed);
        assert!(agent.is_done());
        assert!(!agent.is_busy());
    }

    #[test]
    fn test_agent_trait_is_done_failed() {
        let agent = MockAgent::new(AgentState::Failed);
        assert!(agent.is_done());
    }

    #[test]
    fn test_agent_trait_is_done_executing() {
        let agent = MockAgent::new(AgentState::Executing);
        assert!(!agent.is_done());
    }

    #[test]
    fn test_agent_display_name() {
        let agent = MockAgent::new(AgentState::Idle);
        let name = agent.display_name();
        assert!(name.contains("Coder"));
        assert!(name.contains("mock"));
    }

    #[tokio::test]
    async fn test_agent_handle_changes_state() {
        let mut agent = MockAgent::new(AgentState::Idle);
        let msg = AgentMessage::ProgressUpdate {
            agent: AgentId::named("other"),
            progress: 0.0,
            message: "start".to_string(),
        };
        let result = agent.handle(msg).await;
        assert!(result.is_ok());
        assert_eq!(agent.state(), AgentState::Executing);
    }

    #[tokio::test]
    async fn test_agent_reset() {
        let mut agent = MockAgent::new(AgentState::Executing);
        agent.reset().await.unwrap();
        assert_eq!(agent.state(), AgentState::Idle);
    }
}
