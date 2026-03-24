//! MCP (Model Context Protocol) types
//!
//! Implements the JSON-RPC 2.0 message format used by the MCP specification.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};

// ─── ID generation ─────────────────────────────────────────────────────────────

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

// ─── McpTransport ──────────────────────────────────────────────────────────────

/// How to connect to an MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpTransport {
    /// Launch a process and communicate over stdio
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: std::collections::HashMap<String, String>,
    },
    /// Connect to an HTTP endpoint (SSE or REST)
    Http {
        url: String,
        #[serde(default)]
        headers: std::collections::HashMap<String, String>,
    },
}

// ─── McpTool ───────────────────────────────────────────────────────────────────

/// A tool exposed by an MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input_schema: Value,
}

// ─── JSON-RPC 2.0 Messages ─────────────────────────────────────────────────────

/// A JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl McpRequest {
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: next_id(),
            method: method.into(),
            params,
        }
    }
}

/// A JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

impl McpResponse {
    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.result.is_some()
    }

    pub fn into_result(self) -> Result<Value, McpError> {
        if let Some(err) = self.error {
            Err(err)
        } else {
            Ok(self.result.unwrap_or(Value::Null))
        }
    }
}

/// A JSON-RPC 2.0 error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MCP error {}: {}", self.code, self.message)
    }
}

/// Standard JSON-RPC error codes
pub mod error_codes {
    pub const PARSE_ERROR: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const INTERNAL_ERROR: i64 = -32603;
}

// ─── McpServer Config ──────────────────────────────────────────────────────────

/// Configuration for a single MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Friendly name for this server
    pub name: String,
    /// Transport configuration
    pub transport: McpTransport,
    /// Whether to auto-connect on startup
    #[serde(default = "default_true")]
    pub auto_connect: bool,
}

fn default_true() -> bool {
    true
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_request_new() {
        let req = McpRequest::new("tools/list", None);
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "tools/list");
        assert!(req.params.is_none());
        assert!(req.id > 0);
    }

    #[test]
    fn test_mcp_request_ids_increment() {
        let r1 = McpRequest::new("a", None);
        let r2 = McpRequest::new("b", None);
        assert!(r2.id > r1.id);
    }

    #[test]
    fn test_mcp_request_with_params() {
        let req = McpRequest::new("tools/call", Some(json!({ "name": "echo", "arguments": {} })));
        assert_eq!(req.params.unwrap()["name"], "echo");
    }

    #[test]
    fn test_mcp_response_success() {
        let resp = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: Some(json!({ "tools": [] })),
            error: None,
        };
        assert!(resp.is_success());
        assert!(resp.into_result().is_ok());
    }

    #[test]
    fn test_mcp_response_error() {
        let resp = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: None,
            error: Some(McpError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            }),
        };
        assert!(!resp.is_success());
        assert!(resp.into_result().is_err());
    }

    #[test]
    fn test_mcp_error_display() {
        let err = McpError {
            code: -32603,
            message: "Internal error".to_string(),
            data: None,
        };
        assert!(err.to_string().contains("-32603"));
        assert!(err.to_string().contains("Internal error"));
    }

    #[test]
    fn test_mcp_request_serialize() {
        let req = McpRequest::new("tools/list", None);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("jsonrpc"));
        assert!(json.contains("tools/list"));
    }

    #[test]
    fn test_mcp_response_deserialize() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let resp: McpResponse = serde_json::from_str(json).unwrap();
        assert!(resp.is_success());
        assert!(resp.result.unwrap()["tools"].is_array());
    }

    #[test]
    fn test_mcp_transport_stdio_serialize() {
        let transport = McpTransport::Stdio {
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()],
            env: Default::default(),
        };
        let json = serde_json::to_string(&transport).unwrap();
        assert!(json.contains("stdio"));
        assert!(json.contains("npx"));
    }

    #[test]
    fn test_mcp_transport_http_serialize() {
        let transport = McpTransport::Http {
            url: "http://localhost:3000/mcp".to_string(),
            headers: Default::default(),
        };
        let json = serde_json::to_value(&transport).unwrap();
        assert_eq!(json["type"], "http");
        assert_eq!(json["url"], "http://localhost:3000/mcp");
    }

    #[test]
    fn test_mcp_tool_deserialize() {
        let json = r#"{"name":"read_file","description":"Read a file","input_schema":{}}"#;
        let tool: McpTool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "read_file");
    }

    #[test]
    fn test_server_config_defaults() {
        let cfg = McpServerConfig {
            name: "test".to_string(),
            transport: McpTransport::Stdio {
                command: "test-server".to_string(),
                args: vec![],
                env: Default::default(),
            },
            auto_connect: default_true(),
        };
        assert!(cfg.auto_connect);
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(error_codes::PARSE_ERROR, -32700);
        assert_eq!(error_codes::METHOD_NOT_FOUND, -32601);
        assert_eq!(error_codes::INTERNAL_ERROR, -32603);
    }
}
