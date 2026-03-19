//! Agent module for zcode
//!
//! This module implements the main agent loop and orchestration.

/// Agent state
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

    #[test]
    fn test_agent_creation() {
        let agent = Agent::new("zcode".to_string());
        assert_eq!(agent.name(), "zcode");
    }
}
