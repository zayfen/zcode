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
}
