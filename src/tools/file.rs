//! File I/O tools for zcode
//!
//! This module provides tools for reading, writing, and editing files.

use crate::error::ZcodeError;
use crate::tools::{Tool, ToolResult};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::Path;

// ─── FileReadTool ──────────────────────────────────────────────────────────────

/// Input parameters for the file_read tool
#[derive(Debug, Deserialize)]
struct FileReadInput {
    path: String,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
}

/// Read file contents with optional offset (line-based) and limit
pub struct FileReadTool;

impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read file contents with optional line offset and limit"
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let params: FileReadInput = serde_json::from_value(input)
            .map_err(|e| ZcodeError::InvalidToolInput(e.to_string()))?;

        let path = Path::new(&params.path);
        if !path.exists() {
            return Err(ZcodeError::FileNotFound {
                path: params.path.clone(),
            });
        }

        let content = fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        let offset = params.offset.unwrap_or(0);
        let limit = params.limit.unwrap_or(total_lines.saturating_sub(offset));

        let selected: Vec<&str> = lines
            .iter()
            .skip(offset)
            .take(limit)
            .copied()
            .collect();

        let end_line = offset + selected.len().saturating_sub(1);

        Ok(serde_json::json!({
            "content": selected.join("\n"),
            "total_lines": total_lines,
            "start_line": offset,
            "end_line": end_line
        }))
    }
}

// ─── FileWriteTool ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct FileWriteInput {
    path: String,
    content: String,
}

/// Write content to a file, creating parent directories as needed
pub struct FileWriteTool;

impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file, creating it and parent directories if they don't exist"
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let params: FileWriteInput = serde_json::from_value(input)
            .map_err(|e| ZcodeError::InvalidToolInput(e.to_string()))?;

        let path = Path::new(&params.path);

        // Create parent directories as needed
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let bytes_written = params.content.len();
        fs::write(path, &params.content)?;

        Ok(serde_json::json!({
            "success": true,
            "path": params.path,
            "bytes_written": bytes_written
        }))
    }
}

// ─── FileEditTool ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct FileEditInput {
    path: String,
    old_text: String,
    new_text: String,
    #[serde(default)]
    replace_all: bool,
}

/// Edit a file by replacing text occurrences
pub struct FileEditTool;

impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing text occurrences (first occurrence by default, or all with replace_all=true)"
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let params: FileEditInput = serde_json::from_value(input)
            .map_err(|e| ZcodeError::InvalidToolInput(e.to_string()))?;

        let path = Path::new(&params.path);
        if !path.exists() {
            return Err(ZcodeError::FileNotFound {
                path: params.path.clone(),
            });
        }

        let content = fs::read_to_string(path)?;

        let (new_content, replacements) = if params.replace_all {
            let count = content.matches(&params.old_text).count();
            (content.replace(&params.old_text, &params.new_text), count)
        } else {
            let new_content = content.replacen(&params.old_text, &params.new_text, 1);
            let count = if new_content != content { 1 } else { 0 };
            (new_content, count)
        };

        if replacements == 0 {
            return Err(ZcodeError::ToolExecutionFailed {
                name: "file_edit".to_string(),
                message: format!("Text '{}' not found in file", params.old_text),
            });
        }

        fs::write(path, &new_content)?;

        Ok(serde_json::json!({
            "success": true,
            "path": params.path,
            "replacements": replacements
        }))
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── FileReadTool ──

    #[test]
    fn test_file_read_basic() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "Hello, World!").unwrap();

        let result = FileReadTool.execute(serde_json::json!({
            "path": path.to_str().unwrap()
        })).unwrap();

        assert_eq!(result["content"], "Hello, World!");
        assert_eq!(result["total_lines"], 1);
    }

    #[test]
    fn test_file_read_with_offset_and_limit() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("lines.txt");
        fs::write(&path, "Line 1\nLine 2\nLine 3\nLine 4").unwrap();

        let result = FileReadTool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "offset": 1,
            "limit": 2
        })).unwrap();

        let content = result["content"].as_str().unwrap();
        assert!(content.contains("Line 2"));
        assert!(content.contains("Line 3"));
        assert!(!content.contains("Line 1"));
        assert!(!content.contains("Line 4"));
    }

    #[test]
    fn test_file_read_nonexistent() {
        let result = FileReadTool.execute(serde_json::json!({
            "path": "/nonexistent/path.txt"
        }));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ZcodeError::FileNotFound { .. }));
    }

    #[test]
    fn test_file_read_empty_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("empty.txt");
        fs::write(&path, "").unwrap();

        let result = FileReadTool.execute(serde_json::json!({
            "path": path.to_str().unwrap()
        })).unwrap();

        assert_eq!(result["content"], "");
        assert_eq!(result["total_lines"], 0);
    }

    #[test]
    fn test_file_read_unicode() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("unicode.txt");
        fs::write(&path, "Hello 你好 🎉\nSecond line").unwrap();

        let result = FileReadTool.execute(serde_json::json!({
            "path": path.to_str().unwrap()
        })).unwrap();

        assert!(result["content"].as_str().unwrap().contains("你好"));
        assert_eq!(result["total_lines"], 2);
    }

    #[test]
    fn test_file_read_invalid_input() {
        let result = FileReadTool.execute(serde_json::json!({}));
        assert!(result.is_err());
    }

    // ── FileWriteTool ──

    #[test]
    fn test_file_write_basic() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("output.txt");

        let result = FileWriteTool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": "Test content"
        })).unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), "Test content");
    }

    #[test]
    fn test_file_write_creates_directories() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested/deep/dir/file.txt");

        let result = FileWriteTool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": "Nested content"
        })).unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(path.exists());
    }

    #[test]
    fn test_file_write_overwrites_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("overwrite.txt");
        fs::write(&path, "Original").unwrap();

        FileWriteTool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": "New content"
        })).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "New content");
    }

    #[test]
    fn test_file_write_returns_bytes_written() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("out.txt");

        let result = FileWriteTool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": "Hello"
        })).unwrap();

        assert_eq!(result["bytes_written"], 5);
    }

    #[test]
    fn test_file_write_empty_content() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("empty.txt");

        let result = FileWriteTool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": ""
        })).unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["bytes_written"], 0);
    }

    // ── FileEditTool ──

    #[test]
    fn test_file_edit_basic() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("edit.txt");
        fs::write(&path, "Hello World").unwrap();

        let result = FileEditTool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "old_text": "World",
            "new_text": "Rust"
        })).unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["replacements"], 1);
        assert_eq!(fs::read_to_string(&path).unwrap(), "Hello Rust");
    }

    #[test]
    fn test_file_edit_replace_all() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("multi.txt");
        fs::write(&path, "foo bar foo baz foo").unwrap();

        let result = FileEditTool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "old_text": "foo",
            "new_text": "qux",
            "replace_all": true
        })).unwrap();

        assert_eq!(result["replacements"], 3);
        assert_eq!(fs::read_to_string(&path).unwrap(), "qux bar qux baz qux");
    }

    #[test]
    fn test_file_edit_replace_first_only() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("first.txt");
        fs::write(&path, "foo foo foo").unwrap();

        let result = FileEditTool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "old_text": "foo",
            "new_text": "bar"
        })).unwrap();

        assert_eq!(result["replacements"], 1);
        assert_eq!(fs::read_to_string(&path).unwrap(), "bar foo foo");
    }

    #[test]
    fn test_file_edit_not_found_text() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("notfound.txt");
        fs::write(&path, "Hello World").unwrap();

        let result = FileEditTool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "old_text": "NotPresent",
            "new_text": "Replaced"
        }));

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ZcodeError::ToolExecutionFailed { .. }));
    }

    #[test]
    fn test_file_edit_nonexistent_file() {
        let result = FileEditTool.execute(serde_json::json!({
            "path": "/nonexistent/file.txt",
            "old_text": "x",
            "new_text": "y"
        }));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ZcodeError::FileNotFound { .. }));
    }

    #[test]
    fn test_file_edit_multiline() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("multiline.txt");
        fs::write(&path, "fn old_name() {\n    println!(\"hello\");\n}").unwrap();

        let result = FileEditTool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "old_text": "old_name",
            "new_text": "new_name"
        })).unwrap();

        assert_eq!(result["replacements"], 1);
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("fn new_name()"));
    }
}
