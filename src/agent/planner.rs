//! Planner Agent
//!
//! Analyzes the codebase (using glob + search + file_read) and produces
//! a structured execution plan for the Orchestrator to delegate.

use crate::agent::traits::AgentTrait;
use crate::agent::types::{AgentId, AgentMessage, AgentState, AgentType, Task, TaskResult};
use crate::error::Result;
use crate::tools::ToolRegistry;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Agent that explores the codebase and creates an execution plan
pub struct PlannerAgent {
    id: AgentId,
    state: AgentState,
    registry: Arc<ToolRegistry>,
}

impl PlannerAgent {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self {
            id: AgentId::new(),
            state: AgentState::Idle,
            registry,
        }
    }

    fn transition(&mut self, next: AgentState) {
        if self.state.can_transition_to(next) {
            self.state = next;
        }
    }

    /// Explore the project structure using glob + file_read tools
    pub fn explore_project(&self, cwd: Option<&str>) -> Vec<String> {
        let base = cwd.unwrap_or(".");

        // Try to glob for source files
        let result = self.registry.execute(
            "glob",
            json!({
                "pattern": "**/*.rs",
                "path": base,
                "max_files": 50
            }),
        );

        match result {
            Ok(v) => v["files"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|f| f.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    /// Read a key file to understand its structure
    pub fn read_file_summary(&self, path: &str, max_lines: usize) -> String {
        match self.registry.execute(
            "file_read",
            json!({
                "path": path,
                "limit": max_lines
            }),
        ) {
            Ok(v) => v["content"]
                .as_str()
                .unwrap_or("")
                .lines()
                .take(20)
                .collect::<Vec<_>>()
                .join("\n"),
            Err(_) => format!("(Could not read {})", path),
        }
    }

    /// Generate a simple plan string based on the task and explored files
    pub fn generate_plan(&self, task: &Task) -> String {
        let files = self.explore_project(task.cwd.as_deref());
        let file_count = files.len();
        let file_list = files.iter().take(5).cloned().collect::<Vec<_>>().join(", ");

        format!(
            "Plan for: {}\n\
             Found {} source file(s). Key files: {}\n\
             Steps:\n\
             1. Read relevant source files\n\
             2. Identify the code to modify\n\
             3. Apply changes using file_edit\n\
             4. Verify with shell (cargo check or tests)",
            task.description,
            file_count,
            if file_list.is_empty() { "(none found)".to_string() } else { file_list }
        )
    }
}

#[async_trait]
impl AgentTrait for PlannerAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Planner
    }

    fn state(&self) -> AgentState {
        self.state
    }

    async fn handle(&mut self, message: AgentMessage) -> Result<Option<AgentMessage>> {
        match message {
            AgentMessage::TaskAssigned { task, .. } => {
                self.transition(AgentState::Planning);
                let plan = self.generate_plan(&task);
                self.transition(AgentState::Executing);
                self.transition(AgentState::Completed);

                let mut result = TaskResult::success(&task.id, plan);
                result.tool_calls = 1;

                Ok(Some(AgentMessage::TaskCompleted {
                    agent: self.id.clone(),
                    result,
                }))
            }
            _ => Ok(None),
        }
    }

    async fn reset(&mut self) -> Result<()> {
        self.state = AgentState::Idle;
        Ok(())
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::register_default_tools;
    use tempfile::TempDir;
    use std::fs;

    fn make_planner() -> PlannerAgent {
        let mut r = ToolRegistry::new();
        register_default_tools(&mut r);
        PlannerAgent::new(Arc::new(r))
    }

    #[test]
    fn test_planner_new() {
        let p = make_planner();
        assert_eq!(p.state(), AgentState::Idle);
        assert_eq!(p.agent_type(), AgentType::Planner);
    }

    #[test]
    fn test_explore_project_finds_rs_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("lib.rs"), "pub mod foo;").unwrap();
        fs::write(dir.path().join("readme.md"), "# Readme").unwrap();

        let p = make_planner();
        let files = p.explore_project(Some(dir.path().to_str().unwrap()));
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.ends_with(".rs")));
    }

    #[test]
    fn test_explore_project_empty_dir() {
        let dir = TempDir::new().unwrap();
        let p = make_planner();
        let files = p.explore_project(Some(dir.path().to_str().unwrap()));
        assert!(files.is_empty());
    }

    #[test]
    fn test_generate_plan_contains_steps() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

        let p = make_planner();
        let task = Task::new("Add logging").with_cwd(dir.path().to_str().unwrap());
        let plan = p.generate_plan(&task);

        assert!(plan.contains("Plan for:"));
        assert!(plan.contains("Steps:"));
        assert!(plan.contains("file_edit"));
    }

    #[test]
    fn test_read_file_summary() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, "fn main() {\n    println!(\"hello\");\n}").unwrap();

        let p = make_planner();
        let summary = p.read_file_summary(path.to_str().unwrap(), 10);
        assert!(summary.contains("fn main"));
    }

    #[tokio::test]
    async fn test_handle_task_assigned_returns_plan() {
        let mut p = make_planner();
        let task = Task::new("Implement sorting algorithm");
        let msg = AgentMessage::TaskAssigned {
            from: AgentId::named("orchestrator"),
            to: p.id().clone(),
            task,
        };

        let result = p.handle(msg).await.unwrap().unwrap();
        match result {
            AgentMessage::TaskCompleted { result, .. } => {
                assert!(result.success);
                assert!(result.output.contains("Plan for:"));
            }
            _ => panic!("Expected TaskCompleted"),
        }
        assert_eq!(p.state(), AgentState::Completed);
    }

    #[tokio::test]
    async fn test_reset() {
        let mut p = make_planner();
        p.state = AgentState::Completed;
        p.reset().await.unwrap();
        assert_eq!(p.state(), AgentState::Idle);
    }
}
