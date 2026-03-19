//! Agent module for zcode
//!
//! This module implements the main agent loop and orchestration.

/// Agent state
#[derive(Debug, Clone)]
pub struct Agent {
    name: String,
}

impl Agent {
    /// Create a new agent
    pub fn new(name: String) -> Self {
        Self { name }
    }

    /// Get the agent name
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Agent creation tests
    // ============================================================

    #[test]
    fn test_agent_new() {
        let agent = Agent::new("zcode".to_string());
        assert_eq!(agent.name(), "zcode");
    }

    #[test]
    fn test_agent_new_empty_name() {
        let agent = Agent::new("".to_string());
        assert_eq!(agent.name(), "");
    }

    #[test]
    fn test_agent_new_long_name() {
        let long_name = "x".repeat(1000);
        let agent = Agent::new(long_name.clone());
        assert_eq!(agent.name(), long_name);
    }

    #[test]
    fn test_agent_new_special_characters() {
        let agent = Agent::new("agent-123_test".to_string());
        assert_eq!(agent.name(), "agent-123_test");
    }

    #[test]
    fn test_agent_new_unicode() {
        let agent = Agent::new("智能代理 🤖".to_string());
        assert_eq!(agent.name(), "智能代理 🤖");
    }

    // ============================================================
    // Agent name accessor tests
    // ============================================================

    #[test]
    fn test_agent_name_returns_reference() {
        let agent = Agent::new("test".to_string());
        let name: &str = agent.name();
        assert_eq!(name, "test");
    }

    #[test]
    fn test_agent_name_borrowed() {
        let agent = Agent::new("borrowed".to_string());
        let name1 = agent.name();
        let name2 = agent.name();
        assert_eq!(name1, name2);
    }

    // ============================================================
    // Agent struct tests
    // ============================================================

    #[test]
    fn test_agent_struct_field_access() {
        let agent = Agent::new("field-test".to_string());
        // Verify we can access the name field
        assert_eq!(agent.name, "field-test");
    }

    #[test]
    fn test_agent_struct_field_mutation() {
        let mut agent = Agent::new("original".to_string());
        agent.name = "modified".to_string();
        assert_eq!(agent.name(), "modified");
    }

    // ============================================================
    // Agent different names tests
    // ============================================================

    #[test]
    fn test_agent_different_names() {
        let agent1 = Agent::new("agent1".to_string());
        let agent2 = Agent::new("agent2".to_string());

        assert_ne!(agent1.name(), agent2.name());
        assert_eq!(agent1.name(), "agent1");
        assert_eq!(agent2.name(), "agent2");
    }

    // ============================================================
    // Agent multiple instances tests
    // ============================================================

    #[test]
    fn test_agent_multiple_instances() {
        let agents: Vec<Agent> = (0..10)
            .map(|i| Agent::new(format!("agent-{}", i)))
            .collect();

        for (i, agent) in agents.iter().enumerate() {
            assert_eq!(agent.name(), format!("agent-{}", i));
        }
    }

    // ============================================================
    // Agent ownership tests
    // ============================================================

    #[test]
    fn test_agent_ownership_transfer() {
        let agent = Agent::new("owned".to_string());
        let name = agent.name().to_string();
        drop(agent);
        assert_eq!(name, "owned");
    }

    #[test]
    fn test_agent_clone() {
        let agent = Agent::new("cloneable".to_string());
        let cloned = agent.clone();
        assert_eq!(agent.name(), cloned.name());
    }

    #[test]
    fn test_agent_debug() {
        let agent = Agent::new("debug-test".to_string());
        let debug_str = format!("{:?}", agent);
        // Agent derives Debug through the struct definition
        assert!(debug_str.contains("Agent"));
    }

    // ============================================================
    // Edge cases
    // ============================================================

    #[test]
    fn test_agent_whitespace_name() {
        let agent = Agent::new("   ".to_string());
        assert_eq!(agent.name(), "   ");
    }

    #[test]
    fn test_agent_newlines_in_name() {
        let agent = Agent::new("line1\nline2".to_string());
        assert!(agent.name().contains('\n'));
    }

    #[test]
    fn test_agent_tabs_in_name() {
        let agent = Agent::new("tab\there".to_string());
        assert!(agent.name().contains('\t'));
    }
}
