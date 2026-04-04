//! Zcode - A programming agent CLI tool
//!
//! This crate provides the core functionality for the zcode programming agent,
//! including tool execution, LLM integration, and configuration management.
//!
//! # Architecture
//!
//! Zcode is built as a modular monolith with the following main components:
//!
//! - **error**: Error types and result aliases
//! - **config**: Configuration management (user-level settings and project-level configs)
//! - **tools**: Tool registry and execution system
//! - **llm**: LLM provider integration with streaming support
//! - **agent**: Agent orchestration and state management
//! - **tui**: Terminal user interface with chat capabilities
//!
//! # Example
//!
//! ```rust,no_run
//! use zcode::{Settings, ToolRegistry};
//!
//! // Load user settings
//! let settings = Settings::load().unwrap_or_default();
//!
//! // Initialize tool registry
//! let registry = ToolRegistry::new();
//! ```

pub mod error;
pub mod config;
pub mod tools;
pub mod llm;
pub mod agent;
pub mod tui;
pub mod cli;
pub mod ast;
pub mod memory;
pub mod mcp;
pub mod script;
pub mod session;
pub mod git;
pub mod lsp;
pub mod workspace;
pub mod docs;
pub mod task_store;
pub mod skills;
pub mod verification;
pub mod execution;
pub mod delivery;
pub mod cognition;
pub mod pipeline;

// Re-exports for convenience
pub use error::{ZcodeError, Result};
pub use config::{Settings, ProjectConfig};
pub use tools::{ToolRegistry, Tool, ToolResult, register_default_tools};
pub use llm::{LlmProvider, LlmConfig, Message};
pub use tui::{TuiApp, ChatInterface};
pub use ast::{LanguageProvider, LanguageRegistry, GrammarRegistry};
pub use agent::{AgentId, AgentState, AgentType, Task, TaskResult, OrchestratorAgent, CoderAgent, PlannerAgent};
pub use memory::{WorkingMemory, ProjectMemory, SemanticIndex, ContextAssembler, TokenBudget};
pub use script::{ScriptManager, ScriptContext, HookRegistry, HookType, default_script_manager};
pub use mcp::{McpTool, McpTransport, McpServerConfig};
pub use session::{SnapshotManager, Snapshot};
pub use git::{GitDiff, DiffContext};
pub use agent::{ReviewerAgent, ReviewResult};
pub use workspace::{Workspace, WorkspaceContext, WorkspaceInfo};
pub use config::{LspServerConfig, GrammarConfig, ScriptConfig, SnapshotConfig, HookConfig};
pub use verification::{
    VerificationPipeline, VerificationContext, VerificationPolicy, VerificationScore,
    VerificationFeedback, VerificationIssue, VerificationResult, IssueSeverity, Verifier,
    TestVerifier, LintVerifier, ReviewerVerifier,
};
pub use execution::{
    TaskGraph, TaskNode, TaskId, TaskNodeStatus,
    ExecutionBudget, BudgetTracker, BudgetReport,
    CheckpointPolicy, CheckpointMode, HighRiskPattern,
    PlanVerifier, PlanVerificationResult, PlanIssue,
};
pub use delivery::{
    DeliveryPipeline, DeliveryConfig, DeliveryResult, DeliveryContext,
    GitPlatform, CiConfig, CiPlatform, GateCheck, GateCheckType,
    ChangelogGenerator, TaskRecord, ChangeEntry, ChangeCategory,
    PullRequestCreator, PrOptions, PrResult,
    CiMonitor, CiStatus,
    GateChecker, GateResult,
};
pub use cognition::{
    CognitionEngine, CognitionEngineImpl, IndexMode, IndexStats,
    EmbeddingModel, MockEmbedding,
    CodeChunker, CodeBlock, BlockKind,
    VectorIndex, SearchResult as CognitionSearchResult,
    CognitionStorage,
    SessionMemory, DecisionRecord, LearnedPattern, MemoryRecall, MemoryType,
    KnowledgeSource, KnowledgeQuery, KnowledgeFragment, KnowledgeKind, KnowledgeContext,
    cosine_similarity, vec_to_blob, blob_to_vec,
};
pub use pipeline::{
    Pipeline, PipelineConfig, PipelineContext, PipelineState,
    PipelineResult, PipelineMetrics, PipelinePhase,
    PhaseResult, PhaseStatus, PhaseMetrics, TokenUsage,
    PipelineHooks, HookConfig as PipelineHookConfig, HookAction,
    CognitionPhase, PlanningPhase, ExecutionPhase,
    VerificationPhase, DeliveryPhase,
};


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_load_or_default() {
        let settings = Settings::load().unwrap_or_default();
        assert_eq!(settings.llm.provider, "anthropic");
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
