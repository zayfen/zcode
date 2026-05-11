//! Capability management for zcode.
//!
//! This layer owns skills, MCP integration, shared global prompt/context, and
//! the OpenAI-compatible tool-call abstraction used by LLM providers and agents.

pub mod mcp;
pub mod skills;
pub mod context;
pub mod tool;

pub use context::GlobalSharedContext;
pub use mcp::{McpClient, McpError, McpRequest, McpResponse, McpServerConfig, McpTool, McpToolAdapter, McpTransport};
pub use skills::{Skill, SkillPriority, SkillsLoader};
pub use tool::{
    execute_tool_call, execute_tool_calls, generate_tool_schemas, register_default_tools, Tool,
    ToolCallRequest, ToolCallResponse, ToolRegistry, ToolResult, ToolSchema,
};
