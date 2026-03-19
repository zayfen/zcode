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
}
