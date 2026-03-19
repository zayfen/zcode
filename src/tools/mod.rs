//! Tool system for zcode
//!
//! This module defines the tool trait and registry for managing and executing tools.

use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
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
    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = ToolResult<Value>> + Send + '_>>;
}

pub mod file_ops;
pub use file_ops::{FileReadTool, FileWriteTool, ShellExecTool, FileEditTool};

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
    pub async fn execute(&self, name: &str, input: Value) -> ToolResult<Value> {
        let tool = self.tools.get(name).ok_or_else(|| ZcodeError::ToolNotFound {
            name: name.to_string(),
        })?;

        tool.execute(input).await
    }

    /// Register all built-in tools
    pub fn register_built_in_tools(&mut self) {
        self.register(file_ops::FileReadTool);
        self.register(file_ops::FileWriteTool);
        self.register(file_ops::ShellExecTool);
        self.register(file_ops::FileEditTool);
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

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTool;

    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test"
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        fn execute(&self, _input: Value) -> Pin<Box<dyn Future<Output = ToolResult<Value>> + Send + '_>> {
            Box::pin(async { Ok(Value::String("test result".to_string())) })
        }
    }

    #[test]
    fn test_registry_register() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);

        assert!(registry.get("test").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_registry_execute() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);

        let result = registry.execute("test", Value::Null).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("test result".to_string()));
    }

    #[tokio::test]
    async fn test_registry_unknown_tool() {
        let registry = ToolRegistry::new();

        let result = registry.execute("unknown", Value::Null).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_registry_execute_async() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);

        let result = registry.execute("test", Value::Null).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("test result".to_string()));
    }

    #[test]
    fn test_register_built_in_tools() {
        let mut registry = ToolRegistry::new();
        registry.register_built_in_tools();

        let tools = registry.list();
        assert!(tools.contains(&"file_read"));
        assert!(tools.contains(&"file_write"));
        assert!(tools.contains(&"shell_exec"));
        assert_eq!(tools.len(), 4);
    }
}
