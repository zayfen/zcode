//! File operation tools for zcode

use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

use crate::error::ZcodeError;
use super::{Tool, ToolResult};

/// Tool for reading file contents
pub struct FileReadTool;

impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Input: {\"path\": \"<file_path>\"}"
    }

    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = ToolResult<Value>> + Send + '_>> {
        Box::pin(async move {
            let path = input
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ZcodeError::InvalidToolInput("Missing 'path' field".to_string())
                })?;

            let content = tokio::fs::read_to_string(path).await.map_err(|e| {
                ZcodeError::ToolExecutionFailed {
                    name: self.name().to_string(),
                    message: e.to_string(),
                }
            })?;

            Ok(serde_json::json!({ "content": content }))
        })
    }
}

/// Tool for writing content to a file
pub struct FileWriteTool;

impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file. Input: {\"path\": \"<file_path>\", \"content\": \"<text>\"}"
    }

    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = ToolResult<Value>> + Send + '_>> {
        Box::pin(async move {
            let path = input
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ZcodeError::InvalidToolInput("Missing 'path' field".to_string())
                })?;

            let content = input
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ZcodeError::InvalidToolInput("Missing 'content' field".to_string())
                })?;

            // Create parent directories if needed
            if let Some(parent) = std::path::Path::new(path).parent() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    ZcodeError::ToolExecutionFailed {
                        name: self.name().to_string(),
                        message: e.to_string(),
                    }
                })?;
            }

            tokio::fs::write(path, content).await.map_err(|e| {
                ZcodeError::ToolExecutionFailed {
                    name: self.name().to_string(),
                    message: e.to_string(),
                }
            })?;

            Ok(serde_json::json!({ "success": true, "path": path }))
        })
    }
}

/// Tool for executing shell commands
pub struct ShellExecTool;

impl Tool for ShellExecTool {
    fn name(&self) -> &str {
        "shell_exec"
    }

    fn description(&self) -> &str {
        "Execute a shell command. Input: {\"command\": \"<cmd>\", \"cwd\": \"<dir>\" (optional)}"
    }

    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = ToolResult<Value>> + Send + '_>> {
        Box::pin(async move {
            let command = input
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ZcodeError::InvalidToolInput("Missing 'command' field".to_string())
                })?;

            let cwd = input.get("cwd").and_then(|v| v.as_str());

            let mut cmd = if cfg!(target_os = "windows") {
                let mut c = tokio::process::Command::new("cmd");
                c.args(["/C", command]);
                c
            } else {
                let mut c = tokio::process::Command::new("sh");
                c.args(["-c", command]);
                c
            };

            if let Some(dir) = cwd {
                cmd.current_dir(dir);
            }

            let output = cmd.output().await.map_err(|e| {
                ZcodeError::ToolExecutionFailed {
                    name: self.name().to_string(),
                    message: e.to_string(),
                }
            })?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);
            let success = output.status.success();

            Ok(serde_json::json!({
                "stdout": stdout,
                "stderr": stderr,
                "exit_code": exit_code,
                "success": success
            }))
        })
    }
}

/// Tool for editing specific line ranges in a file
pub struct FileEditTool;

impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    fn description(&self) -> &str {
        "Edit specific lines in a file by line number range (1-indexed, inclusive). \
         Input: {\"path\": \"<file>\", \"start_line\": <n>, \"end_line\": <n>, \"content\": \"<replacement text>\"}"
    }

    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = ToolResult<Value>> + Send + '_>> {
        Box::pin(async move {
            let path = input
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ZcodeError::InvalidToolInput("Missing 'path' field".to_string())
                })?;

            let start_line: usize = input
                .get("start_line")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| {
                    ZcodeError::InvalidToolInput("Missing 'start_line' field".to_string())
                })? as usize;

            let end_line: usize = input
                .get("end_line")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| {
                    ZcodeError::InvalidToolInput("Missing 'end_line' field".to_string())
                })? as usize;

            let new_content = input
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ZcodeError::InvalidToolInput("Missing 'content' field".to_string())
                })?;

            if start_line == 0 || end_line == 0 {
                return Err(ZcodeError::InvalidToolInput(
                    "Line numbers must be >= 1 (1-indexed)".to_string(),
                ));
            }

            if start_line > end_line {
                return Err(ZcodeError::InvalidToolInput(
                    "start_line must be <= end_line".to_string(),
                ));
            }

            let file_content = tokio::fs::read_to_string(path).await.map_err(|e| {
                ZcodeError::ToolExecutionFailed {
                    name: self.name().to_string(),
                    message: e.to_string(),
                }
            })?;

            let lines: Vec<&str> = file_content.lines().collect();

            if end_line > lines.len() {
                return Err(ZcodeError::ToolExecutionFailed {
                    name: self.name().to_string(),
                    message: format!(
                        "Line {} out of range (file has {} lines)",
                        end_line,
                        lines.len()
                    ),
                });
            }

            let mut result_lines = Vec::new();
            for line in lines.iter().take(start_line - 1) {
                result_lines.push((*line).to_string());
            }
            for line in new_content.lines() {
                result_lines.push(line.to_string());
            }
            for line in lines.iter().skip(end_line) {
                result_lines.push((*line).to_string());
            }

            let mut output = result_lines.join("\n");
            if file_content.ends_with('\n') && !output.ends_with('\n') {
                output.push('\n');
            }

            tokio::fs::write(path, &output).await.map_err(|e| {
                ZcodeError::ToolExecutionFailed {
                    name: self.name().to_string(),
                    message: e.to_string(),
                }
            })?;

            Ok(serde_json::json!({ "success": true }))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_read_success() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "hello world").unwrap();
        let path = file.path().to_str().unwrap().to_string();

        let tool = FileReadTool;
        let input = serde_json::json!({ "path": path });
        let result = tool.execute(input).await.unwrap();

        assert_eq!(result["content"], "hello world");
    }

    #[tokio::test]
    async fn test_file_read_missing_path() {
        let tool = FileReadTool;
        let input = serde_json::json!({});
        let result = tool.execute(input).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing 'path'"));
    }

    #[tokio::test]
    async fn test_file_read_not_found() {
        let tool = FileReadTool;
        let input = serde_json::json!({ "path": "/nonexistent/path/to/file.txt" });
        let result = tool.execute(input).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("file_read"));
    }

    #[tokio::test]
    async fn test_file_write_success() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("output.txt");
        let tool = FileWriteTool;
        let input = serde_json::json!({"path": path.to_str().unwrap(), "content": "Written by zcode!"});
        let result = tool.execute(input).await;
        assert!(result.is_ok());
        let written = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(written, "Written by zcode!");
    }

    #[tokio::test]
    async fn test_file_write_missing_fields() {
        let tool = FileWriteTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
        let result = tool.execute(serde_json::json!({"path": "/tmp/test"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_shell_exec_echo() {
        let tool = ShellExecTool;
        let input = serde_json::json!({"command": "echo hello"});
        let result = tool.execute(input).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["stdout"].as_str().unwrap().trim(), "hello");
        assert_eq!(output["exit_code"].as_i64().unwrap(), 0);
        assert!(output["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_shell_exec_failing_command() {
        let tool = ShellExecTool;
        let input = serde_json::json!({"command": "exit 1"});
        let result = tool.execute(input).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["exit_code"].as_i64().unwrap(), 1);
        assert!(!output["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_shell_exec_with_cwd() {
        let tool = ShellExecTool;
        let tmp = tempfile::tempdir().unwrap();
        let input = serde_json::json!({"command": "pwd", "cwd": tmp.path().to_str().unwrap()});
        let result = tool.execute(input).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["stdout"].as_str().unwrap().contains(tmp.path().to_str().unwrap()));
    }

    #[tokio::test]
    async fn test_file_edit_line_range() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("edit.txt");
        tokio::fs::write(&path, "line1\nline2\nline3\nline4\nline5\n").await.unwrap();

        let tool = FileEditTool;
        let input = serde_json::json!({
            "path": path.to_str().unwrap(),
            "start_line": 2,
            "end_line": 4,
            "content": "new_line2\nnew_line3"
        });
        let result = tool.execute(input).await.unwrap();
        assert_eq!(result["success"], true);

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "line1\nnew_line2\nnew_line3\nline5\n");
    }

    #[tokio::test]
    async fn test_file_edit_missing_path() {
        let tool = FileEditTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_edit_invalid_line_range() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("edit.txt");
        tokio::fs::write(&path, "line1\nline2\n").await.unwrap();

        let tool = FileEditTool;
        let input = serde_json::json!({
            "path": path.to_str().unwrap(),
            "start_line": 5,
            "end_line": 10,
            "content": "new"
        });
        let result = tool.execute(input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_edit_start_greater_than_end() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("edit.txt");
        tokio::fs::write(&path, "line1\nline2\n").await.unwrap();

        let tool = FileEditTool;
        let input = serde_json::json!({
            "path": path.to_str().unwrap(),
            "start_line": 3,
            "end_line": 1,
            "content": "new"
        });
        let result = tool.execute(input).await;
        assert!(result.is_err());
    }
}
