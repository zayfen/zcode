//! Tool-call capability abstractions.
//!
//! Zcode no longer ships local built-in tools such as file, shell, search, glob,
//! or AST tools in this layer. Tools should come from MCP/capability providers.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use zcode_core::{Result, ZcodeError};

pub type ToolResult<T> = Result<T>;

/// Trait for capability-backed tools.
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, input: Value) -> ToolResult<Value>;

    fn input_schema(&self) -> Value {
        json!({
            "name": self.name(),
            "description": self.description(),
            "input_schema": {
                "type": "object",
                "properties": {},
                "additionalProperties": true
            }
        })
    }

    fn openai_schema(&self) -> Value {
        let schema = self.input_schema();
        let name = schema
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| self.name());
        let description = schema
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| self.description());
        let parameters = schema
            .get("input_schema")
            .cloned()
            .unwrap_or_else(|| json!({"type": "object", "properties": {}, "additionalProperties": true}));

        json!({
            "type": "function",
            "function": {
                "name": name,
                "description": description,
                "parameters": parameters
            }
        })
    }
}

/// Registry for capability-backed tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn execute(&self, name: &str, input: Value) -> ToolResult<Value> {
        let tool = self.tools.get(name).ok_or_else(|| ZcodeError::ToolNotFound {
            name: name.to_string(),
        })?;
        tool.execute(input)
    }

    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    pub fn input_schemas(&self) -> Vec<Value> {
        self.tools.values().map(|tool| tool.input_schema()).collect()
    }

    pub fn openai_schemas(&self) -> Vec<Value> {
        self.tools.values().map(|tool| tool.openai_schema()).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Compatibility hook. Built-in local tools were intentionally removed.
pub fn register_default_tools(_registry: &mut ToolRegistry) {}

/// JSON Schema definition for a single tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl ToolSchema {
    pub fn from_tool(tool: &dyn Tool) -> Self {
        let schema = tool.input_schema();
        Self {
            name: schema
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| tool.name())
                .to_string(),
            description: schema
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| tool.description())
                .to_string(),
            parameters: schema
                .get("input_schema")
                .cloned()
                .unwrap_or_else(|| json!({"type": "object", "properties": {}, "additionalProperties": true})),
        }
    }

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

pub fn generate_tool_schemas(registry: &ToolRegistry) -> Vec<Value> {
    registry.openai_schemas()
}

/// A tool call request returned by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

impl ToolCallRequest {
    pub fn from_openai(value: &Value) -> Option<Self> {
        let id = value.get("id")?.as_str()?.to_string();
        let function = value.get("function")?;
        let name = function.get("name")?.as_str()?.to_string();
        let args_str = function.get("arguments")?.as_str().unwrap_or("{}");
        let arguments = serde_json::from_str(args_str).unwrap_or(json!({}));
        Some(Self { id, name, arguments })
    }
}

/// Result of executing a tool call, to be sent back to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    pub tool_call_id: String,
    pub name: String,
    pub content: String,
    pub success: bool,
}

impl ToolCallResponse {
    pub fn success(request: &ToolCallRequest, result: Value) -> Self {
        Self {
            tool_call_id: request.id.clone(),
            name: request.name.clone(),
            content: result.to_string(),
            success: true,
        }
    }

    pub fn error(request: &ToolCallRequest, error: impl Into<String>) -> Self {
        Self {
            tool_call_id: request.id.clone(),
            name: request.name.clone(),
            content: json!({ "error": error.into() }).to_string(),
            success: false,
        }
    }

    pub fn to_openai_message(&self) -> Value {
        json!({
            "role": "tool",
            "tool_call_id": self.tool_call_id,
            "name": self.name,
            "content": self.content
        })
    }
}

pub fn execute_tool_call(registry: &ToolRegistry, request: &ToolCallRequest) -> ToolCallResponse {
    match registry.execute(&request.name, request.arguments.clone()) {
        Ok(result) => ToolCallResponse::success(request, result),
        Err(e) => ToolCallResponse::error(request, e.to_string()),
    }
}

pub fn execute_tool_calls(
    registry: &ToolRegistry,
    requests: &[ToolCallRequest],
) -> Vec<ToolCallResponse> {
    requests
        .iter()
        .map(|request| execute_tool_call(registry, request))
        .collect()
}
