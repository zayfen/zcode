//! MCP (Model Context Protocol) module

pub mod types;
pub mod client;

pub use types::{McpError, McpRequest, McpResponse, McpServerConfig, McpTool, McpTransport};
pub use client::{McpClient, McpToolAdapter};
