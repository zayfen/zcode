use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// 工具结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub output: String,
    pub success: bool,
}

/// Agent 错误
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Tool error: {0}")]
    Tool(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Timeout")]
    Timeout,
    
    #[error("Max iterations exceeded")]
    MaxIterations,
}

/// 工具 trait
pub trait ToolExecutor: Send + Sync {
    fn execute(&self, call: ToolCall) -> Result<ToolResult, AgentError>;
}

/// 文件读取工具
pub struct ReadFileTool;

impl ToolExecutor for ReadFileTool {
    fn execute(&self, call: ToolCall) -> Result<ToolResult, AgentError> {
        let path = call.arguments["path"].as_str()
            .ok_or_else(|| AgentError::Tool("Missing path".to_string()))?;
        
        let content = std::fs::read_to_string(path)?;
        
        Ok(ToolResult {
            tool_name: call.name,
            output: content,
            success: true,
        })
    }
}

/// 文件写入工具
pub struct WriteFileTool;

impl ToolExecutor for WriteFileTool {
    fn execute(&self, call: ToolCall) -> Result<ToolResult, AgentError> {
        let path = call.arguments["path"].as_str()
            .ok_or_else(|| AgentError::Tool("Missing path".to_string()))?;
        let content = call.arguments["content"].as_str()
            .ok_or_else(|| AgentError::Tool("Missing content".to_string()))?;
        
        // 创建父目录
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::write(path, content)?;
        
        Ok(ToolResult {
            tool_name: call.name,
            output: format!("Wrote {} bytes to {}", content.len(), path),
            success: true,
        })
    }
}

/// 文件编辑工具
pub struct EditFileTool;

impl ToolExecutor for EditFileTool {
    fn execute(&self, call: ToolCall) -> Result<ToolResult, AgentError> {
        let path = call.arguments["path"].as_str()
            .ok_or_else(|| AgentError::Tool("Missing path".to_string()))?;
        let old_text = call.arguments["old_text"].as_str()
            .ok_or_else(|| AgentError::Tool("Missing old_text".to_string()))?;
        let new_text = call.arguments["new_text"].as_str()
            .ok_or_else(|| AgentError::Tool("Missing new_text".to_string()))?;
        
        // 读取文件
        let content = std::fs::read_to_string(path)?;
        
        // 统计匹配次数
        let match_count = content.matches(old_text).count();
        
        if match_count == 0 {
            return Ok(ToolResult {
                tool_name: call.name,
                output: format!("Pattern not found: {}", old_text),
                success: false,
            });
        }
        
        if match_count > 1 {
            return Ok(ToolResult {
                tool_name: call.name,
                output: format!("Found {} matches, need more context", match_count),
                success: false,
            });
        }
        
        // 执行替换
        let new_content = content.replace(old_text, new_text);
        std::fs::write(path, new_content)?;
        
        Ok(ToolResult {
            tool_name: call.name,
            output: format!("Replaced 1 occurrence in {}", path),
            success: true,
        })
    }
}

/// 命令执行工具
pub struct ExecuteTool;

impl ToolExecutor for ExecuteTool {
    fn execute(&self, call: ToolCall) -> Result<ToolResult, AgentError> {
        let command = call.arguments["command"].as_str()
            .ok_or_else(|| AgentError::Tool("Missing command".to_string()))?;
        
        let args: Vec<String> = call.arguments["args"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        
        let cwd = call.arguments["cwd"].as_str();
        
        let mut cmd = std::process::Command::new(command);
        cmd.args(&args);
        
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        
        let output = cmd.output()
            .map_err(|e| AgentError::Tool(format!("Failed to execute: {}", e)))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        let mut result = String::new();
        if !stdout.is_empty() {
            result.push_str(&format!("stdout:\n{}\n", stdout));
        }
        if !stderr.is_empty() {
            result.push_str(&format!("stderr:\n{}\n", stderr));
        }
        result.push_str(&format!("exit code: {}", output.status.code().unwrap_or(-1)));
        
        let success = output.status.success();
        
        Ok(ToolResult {
            tool_name: call.name,
            output: result,
            success,
        })
    }
}

/// 代码搜索工具
pub struct SearchTool;

impl ToolExecutor for SearchTool {
    fn execute(&self, call: ToolCall) -> Result<ToolResult, AgentError> {
        let pattern = call.arguments["pattern"].as_str()
            .ok_or_else(|| AgentError::Tool("Missing pattern".to_string()))?;
        
        let path = call.arguments["path"].as_str().unwrap_or(".");
        
        let mut matches = Vec::new();
        
        fn search_dir(dir: &std::path::Path, pattern: &str, matches: &mut Vec<String>) -> std::io::Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    if !name.starts_with('.') {
                        search_dir(&path, pattern, matches)?;
                    }
                } else if path.is_file() {
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    if !name.starts_with('.') {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            for (line_num, line) in content.lines().enumerate() {
                                if line.contains(pattern) {
                                    matches.push(format!(
                                        "{}:{}: {}",
                                        path.display(),
                                        line_num + 1,
                                        line.trim()
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        }
        
        search_dir(std::path::Path::new(path), pattern, &mut matches)?;
        
        let output = if matches.is_empty() {
            "No matches found".to_string()
        } else {
            matches.join("\n")
        };
        
        Ok(ToolResult {
            tool_name: call.name,
            output,
            success: true,
        })
    }
}

/// 工具注册表
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ToolExecutor>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        registry.register("read_file", Box::new(ReadFileTool));
        registry.register("write_file", Box::new(WriteFileTool));
        registry.register("edit_file", Box::new(EditFileTool));
        registry.register("execute", Box::new(ExecuteTool));
        registry.register("search", Box::new(SearchTool));
        registry
    }
    
    pub fn register(&mut self, name: &str, tool: Box<dyn ToolExecutor>) {
        self.tools.insert(name.to_string(), tool);
    }
    
    pub fn execute(&self, call: ToolCall) -> Result<ToolResult, AgentError> {
        let tool = self.tools.get(&call.name)
            .ok_or_else(|| AgentError::Tool(format!("Unknown tool: {}", call.name)))?;
        tool.execute(call)
    }
    
    #[allow(dead_code)]
    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
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
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_tool_creation() {
        let tool = Tool {
            name: "test".to_string(),
            description: "Test tool".to_string(),
            parameters: serde_json::json!({}),
        };
        assert_eq!(tool.name, "test");
    }

    #[test]
    fn test_tool_call() {
        let call = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "test.txt"}),
        };
        assert_eq!(call.name, "read_file");
    }

    #[test]
    fn test_tool_result() {
        let result = ToolResult {
            tool_name: "read_file".to_string(),
            output: "content".to_string(),
            success: true,
        };
        assert!(result.success);
    }

    #[test]
    fn test_registry_creation() {
        let registry = ToolRegistry::new();
        assert!(registry.tools.contains_key("read_file"));
        assert!(registry.tools.contains_key("write_file"));
        assert!(registry.tools.contains_key("edit_file"));
        assert!(registry.tools.contains_key("execute"));
        assert!(registry.tools.contains_key("search"));
    }

    #[test]
    fn test_read_file_tool() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "hello world").unwrap();
        
        let tool = ReadFileTool;
        let call = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({
                "path": path.to_str().unwrap()
            }),
        };
        
        let result = tool.execute(call).unwrap();
        assert!(result.success);
        assert_eq!(result.output, "hello world");
    }

    #[test]
    fn test_read_file_missing_path() {
        let tool = ReadFileTool;
        let call = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({}),
        };
        
        let result = tool.execute(call);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_file_nonexistent() {
        let tool = ReadFileTool;
        let call = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({
                "path": "/nonexistent/file.txt"
            }),
        };
        
        let result = tool.execute(call);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_file_tool() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        
        let tool = WriteFileTool;
        let call = ToolCall {
            name: "write_file".to_string(),
            arguments: serde_json::json!({
                "path": path.to_str().unwrap(),
                "content": "hello"
            }),
        };
        
        let result = tool.execute(call).unwrap();
        assert!(result.success);
        assert!(result.output.contains("5 bytes"));
        
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn test_write_file_creates_dirs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("subdir/nested/test.txt");
        
        let tool = WriteFileTool;
        let call = ToolCall {
            name: "write_file".to_string(),
            arguments: serde_json::json!({
                "path": path.to_str().unwrap(),
                "content": "test"
            }),
        };
        
        let result = tool.execute(call).unwrap();
        assert!(result.success);
        assert!(path.exists());
    }

    #[test]
    fn test_write_file_missing_content() {
        let tool = WriteFileTool;
        let call = ToolCall {
            name: "write_file".to_string(),
            arguments: serde_json::json!({
                "path": "test.txt"
            }),
        };
        
        let result = tool.execute(call);
        assert!(result.is_err());
    }

    #[test]
    fn test_edit_file_tool() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "hello world").unwrap();
        
        let tool = EditFileTool;
        let call = ToolCall {
            name: "edit_file".to_string(),
            arguments: serde_json::json!({
                "path": path.to_str().unwrap(),
                "old_text": "world",
                "new_text": "Rust"
            }),
        };
        
        let result = tool.execute(call).unwrap();
        assert!(result.success);
        
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello Rust");
    }

    #[test]
    fn test_edit_file_not_found() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "hello world").unwrap();
        
        let tool = EditFileTool;
        let call = ToolCall {
            name: "edit_file".to_string(),
            arguments: serde_json::json!({
                "path": path.to_str().unwrap(),
                "old_text": "nonexistent",
                "new_text": "replacement"
            }),
        };
        
        let result = tool.execute(call).unwrap();
        assert!(!result.success);
    }

    #[test]
    fn test_edit_file_multiple_matches() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "test test test").unwrap();
        
        let tool = EditFileTool;
        let call = ToolCall {
            name: "edit_file".to_string(),
            arguments: serde_json::json!({
                "path": path.to_str().unwrap(),
                "old_text": "test",
                "new_text": "replacement"
            }),
        };
        
        let result = tool.execute(call).unwrap();
        assert!(!result.success);
    }

    #[test]
    fn test_execute_tool_echo() {
        let tool = ExecuteTool;
        let call = ToolCall {
            name: "execute".to_string(),
            arguments: serde_json::json!({
                "command": "echo",
                "args": ["hello"]
            }),
        };
        
        let result = tool.execute(call).unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[test]
    fn test_execute_tool_with_cwd() {
        let dir = TempDir::new().unwrap();
        
        let tool = ExecuteTool;
        let call = ToolCall {
            name: "execute".to_string(),
            arguments: serde_json::json!({
                "command": "pwd",
                "cwd": dir.path().to_str().unwrap()
            }),
        };
        
        let result = tool.execute(call).unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_search_tool() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, "fn main() {}\nfn test() {}").unwrap();
        
        let tool = SearchTool;
        let call = ToolCall {
            name: "search".to_string(),
            arguments: serde_json::json!({
                "pattern": "fn",
                "path": dir.path().to_str().unwrap()
            }),
        };
        
        let result = tool.execute(call).unwrap();
        assert!(result.success);
        assert!(result.output.contains("fn main"));
        assert!(result.output.contains("fn test"));
    }

    #[test]
    fn test_search_tool_no_matches() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, "hello world").unwrap();
        
        let tool = SearchTool;
        let call = ToolCall {
            name: "search".to_string(),
            arguments: serde_json::json!({
                "pattern": "nonexistent",
                "path": dir.path().to_str().unwrap()
            }),
        };
        
        let result = tool.execute(call).unwrap();
        assert_eq!(result.output, "No matches found");
    }

    #[test]
    fn test_registry_execute() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "test").unwrap();
        
        let registry = ToolRegistry::new();
        let call = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({
                "path": path.to_str().unwrap()
            }),
        };
        
        let result = registry.execute(call).unwrap();
        assert_eq!(result.output, "test");
    }

    #[test]
    fn test_registry_unknown_tool() {
        let registry = ToolRegistry::new();
        let call = ToolCall {
            name: "unknown".to_string(),
            arguments: serde_json::json!({}),
        };
        
        let result = registry.execute(call);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_display() {
        let err = AgentError::Tool("test error".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("test error"));
    }

    #[test]
    fn test_tool_result_serialization() {
        let result = ToolResult {
            tool_name: "test".to_string(),
            output: "output".to_string(),
            success: true,
        };
        
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_tool_call_deserialization() {
        let json = r#"{"name":"test","arguments":{}}"#;
        let call: ToolCall = serde_json::from_str(json).unwrap();
        assert_eq!(call.name, "test");
    }

    #[test]
    fn test_default_registry() {
        let registry = ToolRegistry::default();
        assert!(registry.tools.contains_key("read_file"));
    }

    #[test]
    fn test_list_tools() {
        let registry = ToolRegistry::new();
        let tools = registry.list_tools();
        assert_eq!(tools.len(), 5);
    }
}
