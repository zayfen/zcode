use std::future::Future;
use std::pin::Pin;
use zcode::tools::{Tool, ToolRegistry, ToolResult};

// Mock tool for testing
struct MockTool {
    name: String,
}

impl MockTool {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl Tool for MockTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "A mock tool for testing"
    }

    fn execute(&self, _input: serde_json::Value) -> Pin<Box<dyn Future<Output = ToolResult<serde_json::Value>> + Send + '_>> {
        Box::pin(async { Ok(serde_json::json!({ "result": "mock executed" })) })
    }
}

#[tokio::test]
async fn test_registry_registers_tool() {
    let mut registry = ToolRegistry::new();
    let tool = MockTool::new("test_tool");

    registry.register(tool);

    assert!(registry.get("test_tool").is_some());
}

#[tokio::test]
async fn test_registry_executes_tool() {
    let mut registry = ToolRegistry::new();
    let tool = MockTool::new("execute_tool");
    registry.register(tool);

    let input = serde_json::json!({ "param": "value" });
    let result = registry.execute("execute_tool", input).await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["result"], "mock executed");
}

#[tokio::test]
async fn test_registry_unknown_tool() {
    let registry = ToolRegistry::new();

    let result = registry.execute("unknown_tool", serde_json::json!({})).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("unknown_tool"));
}
