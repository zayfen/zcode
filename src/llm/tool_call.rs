//! LLM Tool Calling support
//!
//! Handles serialization/deserialization of tool calls between the agent loop and LLM API.

use crate::tools::{Tool, ToolRegistry};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ─── ToolSchema ────────────────────────────────────────────────────────────────

/// JSON Schema definition for a single tool (sent to LLM for tool selection)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl ToolSchema {
    /// Create a basic tool schema from a Tool trait object
    pub fn from_tool(tool: &dyn Tool) -> Self {
        Self {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            // Generic schema: accept any JSON object
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": true
            }),
        }
    }

    /// Convert to the OpenAI function calling format
    pub fn to_openai_format(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters
            }
        })
    }
}

/// Generate tool schemas for all tools in a registry
pub fn generate_tool_schemas(registry: &ToolRegistry) -> Vec<Value> {
    registry
        .list()
        .into_iter()
        .filter_map(|name| registry.get(name))
        .map(|tool| {
            let schema = ToolSchema::from_tool(tool.as_ref());
            schema.to_openai_format()
        })
        .collect()
}

// ─── ToolCallRequest ───────────────────────────────────────────────────────────

/// A tool call request returned by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    /// Unique ID for this call (from LLM)
    pub id: String,
    /// Tool name to invoke
    pub name: String,
    /// Arguments as JSON
    pub arguments: Value,
}

impl ToolCallRequest {
    /// Parse from OpenAI tool_calls format
    pub fn from_openai(value: &Value) -> Option<Self> {
        let id = value.get("id")?.as_str()?.to_string();
        let function = value.get("function")?;
        let name = function.get("name")?.as_str()?.to_string();
        let args_str = function.get("arguments")?.as_str().unwrap_or("{}");
        let arguments = serde_json::from_str(args_str).unwrap_or(json!({}));

        Some(Self { id, name, arguments })
    }

    /// Parse from Anthropic tool_use block format:
    /// `{"type": "tool_use", "id": "...", "name": "...", "input": {...}}`
    pub fn from_anthropic(value: &Value) -> Option<Self> {
        if value.get("type")?.as_str()? != "tool_use" {
            return None;
        }
        let id = value.get("id")?.as_str()?.to_string();
        let name = value.get("name")?.as_str()?.to_string();
        let arguments = value.get("input").cloned().unwrap_or(json!({}));
        Some(Self { id, name, arguments })
    }
}

// ─── ToolCallResponse ──────────────────────────────────────────────────────────

/// Result of executing a tool call, to be sent back to the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    /// Matches the request ID
    pub tool_call_id: String,
    /// Tool name
    pub name: String,
    /// Result as a JSON string (LLM API expects string content)
    pub content: String,
    /// Whether the execution succeeded
    pub success: bool,
}

impl ToolCallResponse {
    /// Create a success response
    pub fn success(request: &ToolCallRequest, result: Value) -> Self {
        Self {
            tool_call_id: request.id.clone(),
            name: request.name.clone(),
            content: result.to_string(),
            success: true,
        }
    }

    /// Create an error response
    pub fn error(request: &ToolCallRequest, error: impl Into<String>) -> Self {
        let err = error.into();
        Self {
            tool_call_id: request.id.clone(),
            name: request.name.clone(),
            content: json!({ "error": err }).to_string(),
            success: false,
        }
    }

    /// Convert to OpenAI tool message format
    pub fn to_openai_message(&self) -> Value {
        json!({
            "role": "tool",
            "tool_call_id": self.tool_call_id,
            "name": self.name,
            "content": self.content
        })
    }
}

// ─── Execution helper ──────────────────────────────────────────────────────────

/// Execute a single tool call request against a registry
pub fn execute_tool_call(
    registry: &ToolRegistry,
    request: &ToolCallRequest,
) -> ToolCallResponse {
    match registry.execute(&request.name, request.arguments.clone()) {
        Ok(result) => ToolCallResponse::success(request, result),
        Err(e) => ToolCallResponse::error(request, e.to_string()),
    }
}

/// Execute multiple tool call requests in sequence
pub fn execute_tool_calls(
    registry: &ToolRegistry,
    requests: &[ToolCallRequest],
) -> Vec<ToolCallResponse> {
    requests
        .iter()
        .map(|req| execute_tool_call(registry, req))
        .collect()
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{Tool, ToolRegistry, ToolResult};

    struct EchoTool;
    impl Tool for EchoTool {
        fn name(&self) -> &str { "echo" }
        fn description(&self) -> &str { "Echoes input back" }
        fn execute(&self, input: Value) -> ToolResult<Value> {
            Ok(json!({ "echo": input }))
        }
    }

    #[test]
    fn test_tool_schema_from_tool() {
        let schema = ToolSchema::from_tool(&EchoTool);
        assert_eq!(schema.name, "echo");
        assert_eq!(schema.description, "Echoes input back");
    }

    #[test]
    fn test_tool_schema_to_openai_format() {
        let schema = ToolSchema::from_tool(&EchoTool);
        let fmt = schema.to_openai_format();
        assert_eq!(fmt["type"], "function");
        assert_eq!(fmt["function"]["name"], "echo");
        assert_eq!(fmt["function"]["description"], "Echoes input back");
    }

    #[test]
    fn test_generate_tool_schemas() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);
        let schemas = generate_tool_schemas(&registry);
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0]["function"]["name"], "echo");
    }

    #[test]
    fn test_tool_call_request_from_openai() {
        let value = json!({
            "id": "call_abc123",
            "type": "function",
            "function": {
                "name": "file_read",
                "arguments": "{\"path\": \"/tmp/test.txt\"}"
            }
        });

        let req = ToolCallRequest::from_openai(&value).unwrap();
        assert_eq!(req.id, "call_abc123");
        assert_eq!(req.name, "file_read");
        assert_eq!(req.arguments["path"], "/tmp/test.txt");
    }

    #[test]
    fn test_tool_call_request_from_openai_invalid() {
        let value = json!({ "not": "valid" });
        assert!(ToolCallRequest::from_openai(&value).is_none());
    }

    #[test]
    fn test_tool_call_response_success() {
        let req = ToolCallRequest {
            id: "call-1".to_string(),
            name: "echo".to_string(),
            arguments: json!({}),
        };
        let resp = ToolCallResponse::success(&req, json!({ "result": "ok" }));
        assert!(resp.success);
        assert_eq!(resp.tool_call_id, "call-1");
        assert_eq!(resp.name, "echo");
    }

    #[test]
    fn test_tool_call_response_error() {
        let req = ToolCallRequest {
            id: "call-2".to_string(),
            name: "shell".to_string(),
            arguments: json!({}),
        };
        let resp = ToolCallResponse::error(&req, "Command not found");
        assert!(!resp.success);
        assert!(resp.content.contains("Command not found"));
    }

    #[test]
    fn test_tool_call_response_to_openai_message() {
        let req = ToolCallRequest {
            id: "call-3".to_string(),
            name: "glob".to_string(),
            arguments: json!({}),
        };
        let resp = ToolCallResponse::success(&req, json!({ "files": [] }));
        let msg = resp.to_openai_message();

        assert_eq!(msg["role"], "tool");
        assert_eq!(msg["tool_call_id"], "call-3");
        assert_eq!(msg["name"], "glob");
    }

    #[test]
    fn test_execute_tool_call_success() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);

        let req = ToolCallRequest {
            id: "call-1".to_string(),
            name: "echo".to_string(),
            arguments: json!({ "message": "hi" }),
        };

        let resp = execute_tool_call(&registry, &req);
        assert!(resp.success);
    }

    #[test]
    fn test_execute_tool_call_unknown_tool() {
        let registry = ToolRegistry::new();
        let req = ToolCallRequest {
            id: "call-x".to_string(),
            name: "nonexistent_tool".to_string(),
            arguments: json!({}),
        };
        let resp = execute_tool_call(&registry, &req);
        assert!(!resp.success);
        assert!(resp.content.contains("error") || resp.content.contains("nonexistent"));
    }

    #[test]
    fn test_execute_tool_calls_multiple() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);

        let requests = vec![
            ToolCallRequest { id: "1".into(), name: "echo".into(), arguments: json!({}) },
            ToolCallRequest { id: "2".into(), name: "echo".into(), arguments: json!({}) },
        ];

        let responses = execute_tool_calls(&registry, &requests);
        assert_eq!(responses.len(), 2);
        assert!(responses.iter().all(|r| r.success));
    }
}
