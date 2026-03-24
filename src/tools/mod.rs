//! Tool system for zcode
//!
//! This module defines the tool trait and registry for managing and executing tools.

pub mod file;
pub mod glob;
pub mod search;
pub mod shell;
pub mod ast_tools;

pub use file::{FileEditTool, FileReadTool, FileWriteTool};
pub use glob::GlobTool;
pub use search::SearchTool;
pub use shell::ShellTool;

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use crate::error::{ZcodeError, Result};

/// Result type for tool execution
pub type ToolResult<T> = Result<T>;

/// Trait for implementing tools
pub trait Tool: Send + Sync {
    /// Get the tool name
    fn name(&self) -> &str;

    /// Get the tool description
    fn description(&self) -> &str;

    /// Execute the tool with the given input
    fn execute(&self, input: Value) -> ToolResult<Value>;
}

/// Registry for managing and executing tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool in the registry
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        let name = tool.name().to_string();
        self.tools.insert(name, Arc::new(tool));
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Execute a tool by name
    pub fn execute(&self, name: &str, input: Value) -> ToolResult<Value> {
        let tool = self.tools.get(name).ok_or_else(|| ZcodeError::ToolNotFound {
            name: name.to_string(),
        })?;

        tool.execute(input)
    }

    /// List all registered tools
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Register all built-in Phase 2 tools into a registry.
///
/// This populates the registry with: `file_read`, `file_write`, `file_edit`,
/// `search`, `shell`, `glob`.
///
/// AST tools (`ast_search`, `ast_edit`) require a `LanguageRegistry` and must be
/// registered separately via `AstSearchTool::new(registry)`.
pub fn register_default_tools(registry: &mut ToolRegistry) {
    registry.register(FileReadTool);
    registry.register(FileWriteTool);
    registry.register(FileEditTool);
    registry.register(SearchTool);
    registry.register(ShellTool);
    registry.register(GlobTool);
}



#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Test tool implementations
    // ============================================================

    struct TestTool;

    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test"
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        fn execute(&self, _input: Value) -> ToolResult<Value> {
            Ok(Value::String("test result".to_string()))
        }
    }

    struct EchoTool;

    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }

        fn description(&self) -> &str {
            "Echoes the input"
        }

        fn execute(&self, input: Value) -> ToolResult<Value> {
            Ok(input)
        }
    }

    struct FailingTool;

    impl Tool for FailingTool {
        fn name(&self) -> &str {
            "failing"
        }

        fn description(&self) -> &str {
            "A tool that always fails"
        }

        fn execute(&self, _input: Value) -> ToolResult<Value> {
            Err(ZcodeError::ToolExecutionFailed {
                name: "failing".to_string(),
                message: "intentional failure".to_string(),
            })
        }
    }

    struct JsonTool;

    impl Tool for JsonTool {
        fn name(&self) -> &str {
            "json"
        }

        fn description(&self) -> &str {
            "Returns a JSON object"
        }

        fn execute(&self, input: Value) -> ToolResult<Value> {
            let mut result = serde_json::Map::new();
            result.insert("input".to_string(), input);
            result.insert("status".to_string(), Value::String("ok".to_string()));
            Ok(Value::Object(result))
        }
    }

    // ============================================================
    // ToolRegistry creation tests
    // ============================================================

    #[test]
    fn test_registry_new() {
        let registry = ToolRegistry::new();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_registry_default() {
        let registry = ToolRegistry::default();
        assert!(registry.list().is_empty());
    }

    // ============================================================
    // ToolRegistry register tests
    // ============================================================

    #[test]
    fn test_registry_register_single_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);

        assert_eq!(registry.list().len(), 1);
        assert!(registry.list().contains(&"test"));
    }

    #[test]
    fn test_registry_register_multiple_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);
        registry.register(EchoTool);
        registry.register(JsonTool);

        assert_eq!(registry.list().len(), 3);
        assert!(registry.list().contains(&"test"));
        assert!(registry.list().contains(&"echo"));
        assert!(registry.list().contains(&"json"));
    }

    #[test]
    fn test_registry_register_overwrites_same_name() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);

        // Register another tool with same name
        struct AnotherTestTool;
        impl Tool for AnotherTestTool {
            fn name(&self) -> &str { "test" }
            fn description(&self) -> &str { "Another test tool" }
            fn execute(&self, _input: Value) -> ToolResult<Value> {
                Ok(Value::String("another result".to_string()))
            }
        }
        registry.register(AnotherTestTool);

        assert_eq!(registry.list().len(), 1);
        let result = registry.execute("test", Value::Null).unwrap();
        assert_eq!(result, Value::String("another result".to_string()));
    }

    // ============================================================
    // ToolRegistry get tests
    // ============================================================

    #[test]
    fn test_registry_get_existing_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);

        let tool = registry.get("test");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "test");
    }

    #[test]
    fn test_registry_get_nonexistent_tool() {
        let registry = ToolRegistry::new();

        let tool = registry.get("nonexistent");
        assert!(tool.is_none());
    }

    #[test]
    fn test_registry_get_returns_arc() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);

        let tool1 = registry.get("test").unwrap();
        let tool2 = registry.get("test").unwrap();

        // Both should point to the same tool
        assert_eq!(tool1.name(), tool2.name());
    }

    // ============================================================
    // ToolRegistry execute tests
    // ============================================================

    #[test]
    fn test_registry_execute_success() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);

        let result = registry.execute("test", Value::Null);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("test result".to_string()));
    }

    #[test]
    fn test_registry_execute_unknown_tool() {
        let registry = ToolRegistry::new();

        let result = registry.execute("unknown", Value::Null);
        assert!(result.is_err());
        match result.unwrap_err() {
            ZcodeError::ToolNotFound { name } => {
                assert_eq!(name, "unknown");
            }
            _ => panic!("Expected ToolNotFound error"),
        }
    }

    #[test]
    fn test_registry_execute_echo_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);

        let input = Value::String("hello".to_string());
        let result = registry.execute("echo", input.clone()).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_registry_execute_failing_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(FailingTool);

        let result = registry.execute("failing", Value::Null);
        assert!(result.is_err());
        match result.unwrap_err() {
            ZcodeError::ToolExecutionFailed { name, message } => {
                assert_eq!(name, "failing");
                assert_eq!(message, "intentional failure");
            }
            _ => panic!("Expected ToolExecutionFailed error"),
        }
    }

    #[test]
    fn test_registry_execute_json_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(JsonTool);

        let input = Value::String("test input".to_string());
        let result = registry.execute("json", input).unwrap();

        assert!(result.is_object());
        let obj = result.as_object().unwrap();
        assert!(obj.contains_key("input"));
        assert!(obj.contains_key("status"));
        assert_eq!(obj.get("status").unwrap(), &Value::String("ok".to_string()));
    }

    #[test]
    fn test_registry_execute_with_null_input() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);

        let result = registry.execute("echo", Value::Null).unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn test_registry_execute_with_complex_input() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);

        let input = serde_json::json!({
            "key": "value",
            "nested": {
                "array": [1, 2, 3]
            }
        });

        let result = registry.execute("echo", input.clone()).unwrap();
        assert_eq!(result, input);
    }

    // ============================================================
    // ToolRegistry list tests
    // ============================================================

    #[test]
    fn test_registry_list_empty() {
        let registry = ToolRegistry::new();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_registry_list_single_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);

        let list = registry.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], "test");
    }

    #[test]
    fn test_registry_list_multiple_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);
        registry.register(EchoTool);
        registry.register(JsonTool);

        let list = registry.list();
        assert_eq!(list.len(), 3);

        // Check all tools are in the list
        let list_set: std::collections::HashSet<&str> = list.into_iter().collect();
        assert!(list_set.contains("test"));
        assert!(list_set.contains("echo"));
        assert!(list_set.contains("json"));
    }

    // ============================================================
    // Tool trait tests
    // ============================================================

    #[test]
    fn test_tool_name() {
        let tool = TestTool;
        assert_eq!(tool.name(), "test");
    }

    #[test]
    fn test_tool_description() {
        let tool = TestTool;
        assert_eq!(tool.description(), "A test tool");
    }

    #[test]
    fn test_tool_execute_returns_value() {
        let tool = TestTool;
        let result = tool.execute(Value::Null).unwrap();
        assert_eq!(result, Value::String("test result".to_string()));
    }

    // ============================================================
    // ToolResult type tests
    // ============================================================

    #[test]
    fn test_tool_result_ok() {
        let result: ToolResult<Value> = Ok(Value::Bool(true));
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_result_err() {
        let result: ToolResult<Value> = Err(ZcodeError::InvalidToolInput("test".to_string()));
        assert!(result.is_err());
    }

    // ============================================================
    // Thread safety tests
    // ============================================================

    #[test]
    fn test_registry_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let mut registry = ToolRegistry::new();
        registry.register(TestTool);
        let registry = Arc::new(registry);

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let registry_clone = Arc::clone(&registry);
                thread::spawn(move || {
                    let result = registry_clone.execute("test", Value::Null);
                    assert!(result.is_ok());
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_tool_arc_clone() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);

        let tool1 = registry.get("test").unwrap();
        let tool2 = tool1.clone();

        assert_eq!(tool1.name(), tool2.name());
    }

    // ============================================================
    // Edge case tests
    // ============================================================

    #[test]
    fn test_registry_execute_empty_name() {
        let registry = ToolRegistry::new();

        let result = registry.execute("", Value::Null);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_with_empty_name() {
        struct EmptyNameTool;
        impl Tool for EmptyNameTool {
            fn name(&self) -> &str { "" }
            fn description(&self) -> &str { "Tool with empty name" }
            fn execute(&self, _input: Value) -> ToolResult<Value> {
                Ok(Value::Null)
            }
        }

        let mut registry = ToolRegistry::new();
        registry.register(EmptyNameTool);

        let result = registry.execute("", Value::Null);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_with_empty_description() {
        struct EmptyDescTool;
        impl Tool for EmptyDescTool {
            fn name(&self) -> &str { "empty_desc" }
            fn description(&self) -> &str { "" }
            fn execute(&self, _input: Value) -> ToolResult<Value> {
                Ok(Value::Null)
            }
        }

        let tool = EmptyDescTool;
        assert_eq!(tool.description(), "");
    }

    #[test]
    fn test_tool_execute_with_large_input() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);

        // Create a large JSON input
        let large_array: Vec<Value> = (0..1000).map(Value::from).collect();
        let input = Value::Array(large_array);

        let result = registry.execute("echo", input.clone()).unwrap();
        assert_eq!(result, input);
    }
}
