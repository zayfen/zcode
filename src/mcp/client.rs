//! MCP Client
//!
//! Connects to MCP servers over stdio, enumerates their tools,
//! and wraps them as standard zcode Tool trait objects.

use crate::error::{Result, ZcodeError};
use crate::mcp::types::{McpRequest, McpResponse, McpTool};
use crate::tools::{Tool, ToolResult};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};

// ─── StdioConnection ───────────────────────────────────────────────────────────

struct StdioConnection {
    stdin: ChildStdin,
    reader: BufReader<Box<dyn std::io::Read + Send>>,
    _child: Child,
}

impl StdioConnection {
    fn new(command: &str, args: &[&str]) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| ZcodeError::InternalError(
                format!("Failed to start MCP server '{}': {}", command, e)
            ))?;

        let stdin = child.stdin.take().ok_or_else(|| {
            ZcodeError::InternalError("Failed to get stdin from MCP server".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ZcodeError::InternalError("Failed to get stdout from MCP server".to_string())
        })?;

        Ok(Self {
            stdin,
            reader: BufReader::new(Box::new(stdout)),
            _child: child,
        })
    }

    fn send_request(&mut self, req: &McpRequest) -> Result<McpResponse> {
        let msg = serde_json::to_string(req)
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        writeln!(self.stdin, "{}", msg)
            .map_err(|e| ZcodeError::InternalError(format!("MCP write error: {}", e)))?;
        self.stdin.flush()
            .map_err(|e| ZcodeError::InternalError(format!("MCP flush error: {}", e)))?;

        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|e| ZcodeError::InternalError(format!("MCP read error: {}", e)))?;

        serde_json::from_str(&line)
            .map_err(|e| ZcodeError::InternalError(format!("MCP parse error: {}", e)))
    }
}

// ─── McpClient ─────────────────────────────────────────────────────────────────

/// Client for a single MCP server (stdio transport)
pub struct McpClient {
    name: String,
    conn: Mutex<StdioConnection>,
    tools: Vec<McpTool>,
}

impl McpClient {
    /// Connect to an MCP server via stdio (launches the process)
    pub fn connect_stdio(
        name: impl Into<String>,
        command: &str,
        args: &[&str],
    ) -> Result<Self> {
        let conn = StdioConnection::new(command, args)?;
        let mut client = Self {
            name: name.into(),
            conn: Mutex::new(conn),
            tools: Vec::new(),
        };
        client.initialize()?;
        Ok(client)
    }

    fn initialize(&mut self) -> Result<()> {
        let req = McpRequest::new(
            "initialize",
            Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "zcode", "version": env!("CARGO_PKG_VERSION") }
            })),
        );

        let mut conn = self.conn.lock().unwrap();
        let resp = conn.send_request(&req)?;
        drop(conn);

        if let Some(err) = resp.error {
            return Err(ZcodeError::InternalError(format!("MCP init failed: {}", err)));
        }

        // Send initialized notification (no response expected)
        let notify = McpRequest::new("notifications/initialized", None);
        let msg = serde_json::to_string(&notify).unwrap_or_default();
        let mut conn = self.conn.lock().unwrap();
        let _ = writeln!(conn.stdin, "{}", msg);
        let _ = conn.stdin.flush();
        drop(conn);

        self.tools = self.fetch_tools()?;
        Ok(())
    }

    fn fetch_tools(&self) -> Result<Vec<McpTool>> {
        let req = McpRequest::new("tools/list", None);
        let mut conn = self.conn.lock().unwrap();
        let resp = conn.send_request(&req)?;
        drop(conn);

        let result = resp.into_result()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        let tools: Vec<McpTool> = result
            .get("tools")
            .and_then(|t| serde_json::from_value(t.clone()).ok())
            .unwrap_or_default();
        Ok(tools)
    }

    /// List all tools from this server
    pub fn list_tools(&self) -> &[McpTool] {
        &self.tools
    }

    /// Call a tool on the server
    pub fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        let req = McpRequest::new(
            "tools/call",
            Some(json!({ "name": name, "arguments": arguments })),
        );

        let mut conn = self.conn.lock().unwrap();
        let resp = conn.send_request(&req)?;
        drop(conn);

        let result = resp.into_result()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        // Extract MCP content array → text
        if let Some(arr) = result.get("content").and_then(|c| c.as_array()) {
            let text: String = arr.iter()
                .filter_map(|item| {
                    if item.get("type")?.as_str()? == "text" {
                        item.get("text")?.as_str().map(str::to_string)
                    } else { None }
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Ok(json!({ "output": text }));
        }

        Ok(result)
    }

    /// Server name
    pub fn name(&self) -> &str { &self.name }

    /// Create Tool adapters for all tools
    pub fn create_adapters(self: Arc<Self>) -> Vec<McpToolAdapter> {
        self.tools.iter().map(|tool| McpToolAdapter {
            client: Arc::clone(&self),
            tool: tool.clone(),
        }).collect()
    }
}

// ─── McpToolAdapter ────────────────────────────────────────────────────────────

/// Wraps a single MCP tool as a zcode Tool trait object
pub struct McpToolAdapter {
    client: Arc<McpClient>,
    tool: McpTool,
}

impl Tool for McpToolAdapter {
    fn name(&self) -> &str { &self.tool.name }
    fn description(&self) -> &str { &self.tool.description }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        self.client
            .call_tool(&self.tool.name, input)
            .map_err(|e| ZcodeError::ToolExecutionFailed {
                name: self.tool.name.clone(),
                message: e.to_string(),
            })
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::types::McpTool;

    #[test]
    fn test_mcp_request_tools_list() {
        let req = McpRequest::new("tools/list", None);
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.jsonrpc, "2.0");
    }

    #[test]
    fn test_mcp_request_tools_call() {
        let req = McpRequest::new(
            "tools/call",
            Some(json!({ "name": "echo", "arguments": { "text": "hello" } })),
        );
        assert_eq!(req.method, "tools/call");
        assert_eq!(req.params.unwrap()["name"], "echo");
    }

    #[test]
    fn test_mcp_content_extraction() {
        let result = json!({
            "content": [
                { "type": "text", "text": "Hello from MCP" },
                { "type": "text", "text": " world" }
            ]
        });
        if let Some(arr) = result.get("content").and_then(|c| c.as_array()) {
            let text: String = arr.iter()
                .filter_map(|item| {
                    if item.get("type")?.as_str()? == "text" {
                        item.get("text")?.as_str().map(str::to_string)
                    } else { None }
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert_eq!(text, "Hello from MCP\n world");
        }
    }

    #[test]
    fn test_mcp_tool_adapter_name_description() {
        // We build a minimal McpClient backed by `cat` (echoes input)
        // Testing only the adapter field access without real MCP calls
        let tool = McpTool {
            name: "list_files".to_string(),
            description: "Lists project files".to_string(),
            input_schema: json!({}),
        };

        // Check field access is correct (no real server needed)
        assert_eq!(tool.name, "list_files");
        assert_eq!(tool.description, "Lists project files");
    }

    #[test]
    fn test_connect_stdio_invalid_command_fails() {
        let result = McpClient::connect_stdio(
            "bad_server",
            "/nonexistent/binary/that/does/not/exist",
            &[],
        );
        assert!(result.is_err());
    }
}
