//! Search tools for zcode
//!
//! Provides ripgrep-style content search and glob pattern matching.

use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

use crate::error::ZcodeError;
use super::{Tool, ToolResult};

/// Tool for searching file contents (ripgrep-style)
pub struct SearchTool;

impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search"
    }

    fn description(&self) -> &str {
        "Search file contents for a pattern (ripgrep-style). \
         Input: {\"pattern\": \"<regex>\", \"path\": \"<dir>\" (default: \".\"), \"glob\": \"<file_pattern>\" (optional)}"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": "Regex pattern to search for"},
                "path": {"type": "string", "description": "Directory to search in (default: current directory)"},
                "glob": {"type": "string", "description": "Optional file glob filter (e.g. \"*.rs\")"}
            },
            "required": ["pattern"]
        })
    }

    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = ToolResult<Value>> + Send + '_>> {
        Box::pin(async move {
            let pattern = input
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ZcodeError::InvalidToolInput("Missing 'pattern' field".to_string())
                })?;

            let search_path = input
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or(".");

            let glob_filter = input.get("glob").and_then(|v| v.as_str());

            let regex = regex::Regex::new(pattern).map_err(|e| {
                ZcodeError::InvalidToolInput(format!("Invalid regex: {}", e))
            })?;

            let mut results: Vec<Value> = Vec::new();
            let max_results = 100;

            for entry in walkdir::WalkDir::new(search_path)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if results.len() >= max_results {
                    break;
                }

                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                // Skip binary files (check extension)
                if let Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "pdf"
                        | "zip" | "tar" | "gz" | "exe" | "dll" | "so" | "dylib") = path.extension().and_then(|e| e.to_str()) {
                    continue;
                }

                // Apply glob filter if provided
                if let Some(glob_pattern) = glob_filter {
                    let pat = glob::Pattern::new(glob_pattern).map_err(|e| {
                        ZcodeError::InvalidToolInput(format!("Invalid glob: {}", e))
                    })?;
                    if !pat.matches_path(path) {
                        continue;
                    }
                }

                // Try to read file, skip on error (binary, permissions, etc.)
                let content = match tokio::fs::read_to_string(path).await {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                for (line_num, line) in content.lines().enumerate() {
                    if results.len() >= max_results {
                        break;
                    }
                    if regex.is_match(line) {
                        results.push(serde_json::json!({
                            "path": path.to_string_lossy(),
                            "line_number": line_num + 1,
                            "line": line.trim(),
                        }));
                    }
                }
            }

            Ok(serde_json::json!({
                "matches": results,
                "total": results.len(),
                "truncated": results.len() >= max_results,
            }))
        })
    }
}

/// Tool for finding files by glob pattern
pub struct GlobTool;

impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern. \
         Input: {\"pattern\": \"<glob>\", \"path\": \"<dir>\" (default: \".\")}"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": "Glob pattern to match files (e.g. \"**/*.rs\")"},
                "path": {"type": "string", "description": "Directory to search in (default: current directory)"}
            },
            "required": ["pattern"]
        })
    }

    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = ToolResult<Value>> + Send + '_>> {
        Box::pin(async move {
            let pattern = input
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ZcodeError::InvalidToolInput("Missing 'pattern' field".to_string())
                })?;

            let search_path = input
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or(".");

            let glob_pattern = glob::Pattern::new(pattern).map_err(|e| {
                ZcodeError::InvalidToolInput(format!("Invalid glob pattern: {}", e))
            })?;

            let mut files: Vec<String> = Vec::new();
            let max_results = 500;

            for entry in walkdir::WalkDir::new(search_path)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if files.len() >= max_results {
                    break;
                }

                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                let matched = glob_pattern.matches_path(path);
                if matched {
                    files.push(path.to_string_lossy().to_string());
                }
            }

            files.sort();

            Ok(serde_json::json!({
                "files": files,
                "total": files.len(),
                "truncated": files.len() >= max_results,
            }))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_finds_pattern() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("test.txt");
        std::fs::write(&file_path, "hello world\nfoo bar\nhello again\n").unwrap();

        let tool = SearchTool;
        let input = serde_json::json!({
            "pattern": "hello",
            "path": tmp.path().to_str().unwrap()
        });
        let result = tool.execute(input).await.unwrap();
        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0]["line_number"], 1);
        assert_eq!(matches[1]["line_number"], 3);
    }

    #[tokio::test]
    async fn test_search_no_matches() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("test.txt");
        std::fs::write(&file_path, "nothing here\n").unwrap();

        let tool = SearchTool;
        let input = serde_json::json!({
            "pattern": "missing",
            "path": tmp.path().to_str().unwrap()
        });
        let result = tool.execute(input).await.unwrap();
        assert_eq!(result["matches"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_search_missing_pattern() {
        let tool = SearchTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_search_with_glob_filter() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.rs"), "fn main() {}\n").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "fn main() {}\n").unwrap();

        let tool = SearchTool;
        let input = serde_json::json!({
            "pattern": "fn main",
            "path": tmp.path().to_str().unwrap(),
            "glob": "*.rs"
        });
        let result = tool.execute(input).await.unwrap();
        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0]["path"].as_str().unwrap().ends_with(".rs"));
    }

    #[tokio::test]
    async fn test_search_invalid_regex() {
        let tool = SearchTool;
        let input = serde_json::json!({"pattern": "[invalid"});
        let result = tool.execute(input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_glob_finds_files() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("a.rs"), "").unwrap();
        std::fs::write(sub.join("b.rs"), "").unwrap();
        std::fs::write(tmp.path().join("c.txt"), "").unwrap();

        let tool = GlobTool;
        let input = serde_json::json!({
            "pattern": "**/*.rs",
            "path": tmp.path().to_str().unwrap()
        });
        let result = tool.execute(input).await.unwrap();
        let files = result["files"].as_array().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[tokio::test]
    async fn test_glob_no_matches() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "").unwrap();

        let tool = GlobTool;
        let input = serde_json::json!({
            "pattern": "**/*.rs",
            "path": tmp.path().to_str().unwrap()
        });
        let result = tool.execute(input).await.unwrap();
        assert_eq!(result["files"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_glob_missing_pattern() {
        let tool = GlobTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }
}
