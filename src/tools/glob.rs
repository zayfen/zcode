//! Glob pattern matching tool for zcode
//!
//! This module provides file pattern matching (glob-style).

use crate::error::ZcodeError;
use crate::tools::{Tool, ToolResult};
use glob::glob;
use serde::Deserialize;
use serde_json::Value;

// ─── GlobTool ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GlobInput {
    pattern: String,
    /// Optional base path to prepend to pattern
    #[serde(default)]
    path: Option<String>,
    /// Maximum number of results (default: 1000)
    #[serde(default = "default_max_files")]
    max_files: usize,
}

fn default_max_files() -> usize {
    1000
}

/// Find files matching glob patterns
pub struct GlobTool;

impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern (e.g. '**/*.rs')"
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let params: GlobInput = serde_json::from_value(input)
            .map_err(|e| ZcodeError::InvalidToolInput(e.to_string()))?;

        // Combine base path and pattern
        let full_pattern = match &params.path {
            Some(base) => {
                let base = base.trim_end_matches('/');
                format!("{}/{}", base, params.pattern)
            }
            None => params.pattern.clone(),
        };

        let paths: Vec<String> = glob(&full_pattern)
            .map_err(|e| ZcodeError::InvalidToolInput(format!("Invalid glob pattern: {}", e)))?
            .filter_map(|entry| entry.ok())
            .take(params.max_files)
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let count = paths.len();

        Ok(serde_json::json!({
            "files": paths,
            "count": count,
            "pattern": full_pattern
        }))
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_glob_basic_rs_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "").unwrap();
        fs::write(dir.path().join("lib.rs"), "").unwrap();
        fs::write(dir.path().join("readme.txt"), "").unwrap();

        let result = GlobTool.execute(serde_json::json!({
            "pattern": "*.rs",
            "path": dir.path().to_str().unwrap()
        })).unwrap();

        let files = result["files"].as_array().unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.as_str().unwrap().ends_with(".rs")));
    }

    #[test]
    fn test_glob_recursive() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("a/b/c");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("deep.rs"), "").unwrap();
        fs::write(dir.path().join("top.rs"), "").unwrap();

        let result = GlobTool.execute(serde_json::json!({
            "pattern": "**/*.rs",
            "path": dir.path().to_str().unwrap()
        })).unwrap();

        let files = result["files"].as_array().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_glob_no_matches() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("file.txt"), "").unwrap();

        let result = GlobTool.execute(serde_json::json!({
            "pattern": "**/*.py",
            "path": dir.path().to_str().unwrap()
        })).unwrap();

        let files = result["files"].as_array().unwrap();
        assert!(files.is_empty());
        assert_eq!(result["count"], 0);
    }

    #[test]
    fn test_glob_without_base_path() {
        // Using absolute pattern
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("file.json"), "{}").unwrap();

        let full_pattern = format!("{}/*.json", dir.path().to_str().unwrap());
        let result = GlobTool.execute(serde_json::json!({
            "pattern": full_pattern
        })).unwrap();

        let files = result["files"].as_array().unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_glob_wildcard_prefix() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test_foo.rs"), "").unwrap();
        fs::write(dir.path().join("test_bar.rs"), "").unwrap();
        fs::write(dir.path().join("main.rs"), "").unwrap();

        let result = GlobTool.execute(serde_json::json!({
            "pattern": "test_*.rs",
            "path": dir.path().to_str().unwrap()
        })).unwrap();

        let files = result["files"].as_array().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_glob_max_files_limit() {
        let dir = TempDir::new().unwrap();
        for i in 0..20 {
            fs::write(dir.path().join(format!("file_{}.rs", i)), "").unwrap();
        }

        let result = GlobTool.execute(serde_json::json!({
            "pattern": "*.rs",
            "path": dir.path().to_str().unwrap(),
            "max_files": 5
        })).unwrap();

        let files = result["files"].as_array().unwrap();
        assert_eq!(files.len(), 5);
    }

    #[test]
    fn test_glob_returns_pattern_in_result() {
        let dir = TempDir::new().unwrap();

        let result = GlobTool.execute(serde_json::json!({
            "pattern": "**/*.rs",
            "path": dir.path().to_str().unwrap()
        })).unwrap();

        let pattern = result["pattern"].as_str().unwrap();
        assert!(pattern.contains("**/*.rs"));
    }

    #[test]
    fn test_glob_invalid_pattern() {
        // Invalid glob pattern with unmatched bracket
        let result = GlobTool.execute(serde_json::json!({
            "pattern": "[invalid"
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_glob_multiple_patterns_separate() {
        // The `glob` crate does NOT support `{a,b}` alternation.
        // Instead, perform two separate glob calls and combine results.
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.rs"), "").unwrap();
        fs::write(dir.path().join("b.py"), "").unwrap();
        fs::write(dir.path().join("c.js"), "").unwrap();

        // First: match .rs files
        let rs_result = GlobTool.execute(serde_json::json!({
            "pattern": "*.rs",
            "path": dir.path().to_str().unwrap()
        })).unwrap();

        // Second: match .py files
        let py_result = GlobTool.execute(serde_json::json!({
            "pattern": "*.py",
            "path": dir.path().to_str().unwrap()
        })).unwrap();

        let rs_count = rs_result["count"].as_u64().unwrap();
        let py_count = py_result["count"].as_u64().unwrap();
        assert_eq!(rs_count, 1);
        assert_eq!(py_count, 1);
    }

    #[test]
    fn test_glob_invalid_input() {
        let result = GlobTool.execute(serde_json::json!({}));
        assert!(result.is_err());
    }
}
