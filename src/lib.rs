//! Zcode - A programming agent CLI tool
//!
//! This package is the CLI/application shell. Core behavior lives in layered
//! workspace crates:
//! `zcode_ui`, `zcode_requirements`, `zcode_orchestration`,
//! `zcode_llm_provider`, `zcode_capabilities`, `zcode_session`, and
//! `zcode_core`.

pub mod cli;
pub mod ast;
pub mod memory;
pub mod script;
pub mod git;
pub mod lsp;
pub mod workspace;

pub use zcode_capabilities as capabilities;
pub mod tools {
    pub use zcode_capabilities::*;
    pub mod ast_tools {}
}
pub use zcode_capabilities::{register_default_tools, Tool, ToolRegistry, ToolResult};
pub use zcode_capabilities::{McpClient, McpServerConfig, McpTool, McpToolAdapter, McpTransport};
pub use zcode_capabilities::{Skill, SkillPriority, SkillsLoader};
pub use zcode_core as core;
pub use zcode_core::config;
pub use zcode_core::error;
pub use zcode_core::{
    GrammarConfig, HookConfig, LspServerConfig, ProjectConfig, Result, ScriptConfig, Settings,
    SnapshotConfig, ZcodeError,
};
pub use zcode_llm_provider as llm;
pub use zcode_llm_provider::{LlmConfig, LlmProvider, Message, RigProvider};
pub mod agent {
    pub use zcode_orchestration::*;
    pub mod graph {
        pub use zcode_orchestration::agent::graph::*;
        pub mod pipeline {
            pub use zcode_orchestration::agent::graph::pipeline::*;
        }
        pub mod state {
            pub use zcode_orchestration::agent::graph::state::*;
        }
    }
    pub mod loop_exec {
        pub use zcode_orchestration::agent::loop_exec::*;
    }
    pub mod types {
        pub use zcode_orchestration::agent::types::*;
    }
}
pub use zcode_orchestration::{
    build_reviewer_pipeline, build_task_pipeline, build_task_pipeline_with_limit, routers, AgentId,
    AgentState, AgentType, AsyncFnNode, CoderAgent, CompiledGraph, ConversationMessage,
    DefaultState, Edge, EndReason, FnNode, GraphEvent, GraphNode, GraphOutput, GraphState,
    NodeOutput, OrchestratorAgent, PlannerAgent, ReviewResult, ReviewerAgent, StateGraph, Task,
    TaskResult,
};
pub use zcode_requirements as requirements;
pub mod docs {
    pub use zcode_requirements::docs::*;
    pub mod parser {
        pub use zcode_requirements::docs::parser::*;
    }
}
pub use zcode_requirements::task_store;
pub use zcode_requirements::{generate_docs_scaffold, DocsValidator, TaskRecord, TaskStatus, TaskStore};
pub use zcode_session as session;
pub use zcode_session::{Snapshot, SnapshotManager};
pub mod tui {
    pub use zcode_ui::*;
    pub mod chat {
        pub use zcode_ui::tui::chat::*;
    }
}
pub use zcode_ui::{ChatInterface, TuiApp};

pub use ast::{LanguageProvider, LanguageRegistry, GrammarRegistry};
pub use memory::{WorkingMemory, ProjectMemory, SemanticIndex, ContextAssembler, TokenBudget};
pub use script::{ScriptManager, ScriptContext, HookRegistry, HookType, default_script_manager};
pub use git::{GitDiff, DiffContext};
pub use workspace::{Workspace, WorkspaceContext, WorkspaceInfo};


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_load_or_default() {
        let settings = Settings::load().unwrap_or_default();
        assert_eq!(settings.llm.provider, "openai-compatible");
    }

    #[test]
    fn test_tool_registry_creation() {
        let registry = ToolRegistry::new();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_error_creation() {
        let error = ZcodeError::ToolNotFound {
            name: "test".to_string(),
        };
        assert!(error.to_string().contains("test"));
    }
}
