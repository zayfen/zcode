//! Search tools for zcode
//!
//! This module provides grep-style search capabilities using regex.

use crate::error::ZcodeError;
use crate::tools::{Tool, ToolResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

// ─── SearchTool ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SearchInput {
    pattern: String,
    path: String,
    /// Optional glob-style file extension filter (e.g. ".rs", ".py")
    #[serde(default)]
    file_extension: Option<String>,
    /// Whether search is case-sensitive (default: false = case-insensitive)
    #[serde(default)]
    case_sensitive: bool,
    /// Number of context lines to include before/after each match
    #[serde(default)]
    context_lines: usize,
    /// Maximum number of matches to return (default: 100)
    #[serde(default = "default_max_matches")]
    max_matches: usize,
}

fn default_max_matches() -> usize {
    100
}

#[derive(Debug, Serialize)]
struct SearchMatch {
    file: String,
    line: usize,
    column: usize,
    text: String,
    context_before: Vec<String>,
    context_after: Vec<String>,
}

/// Grep-style search tool with regex support
pub struct SearchTool;

impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search"
    }

    fn description(&self) -> &str {
        "Search for a regex pattern in files, with optional context lines, file extension filter, and case sensitivity"
    }

    fn anthropic_schema(&self) -> Value {
        serde_json::json!({
            "name": self.name(),
            "description": self.description(),
            "input_schema": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory or file path to search within"
                    },
                    "file_extension": {
                        "type": "string",
                        "description": "Optional file extension to filter by (e.g., '.rs', '.py')"
                    },
                    "case_sensitive": {
                        "type": "boolean",
                        "description": "Whether the search should be case sensitive (default: false)"
                    },
                    "context_lines": {
                        "type": "integer",
                        "description": "Number of context lines to return before and after each match (default: 0)"
                    },
                    "max_matches": {
                        "type": "integer",
                        "description": "Maximum number of matches to return (default: 100)"
                    }
                },
                "required": ["pattern", "path"]
            }
        })
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let params: SearchInput = serde_json::from_value(input)
            .map_err(|e| ZcodeError::InvalidToolInput(e.to_string()))?;

        let pattern = build_regex(&params.pattern, params.case_sensitive)?;
        let path = Path::new(&params.path);
        let mut matches: Vec<SearchMatch> = Vec::new();

        if path.is_file() {
            search_file(path, &pattern, &params, &mut matches);
        } else if path.is_dir() {
            for entry in WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
            {
                if matches.len() >= params.max_matches {
                    break;
                }
                if let Some(ref ext) = params.file_extension {
                    let path_str = entry.path().to_string_lossy();
                    if !path_str.ends_with(ext.as_str()) {
                        continue;
                    }
                }
                search_file(entry.path(), &pattern, &params, &mut matches);
            }
        } else {
            return Err(ZcodeError::FileNotFound {
                path: params.path.clone(),
            });
        }

        let total = matches.len();
        Ok(serde_json::json!({
            "matches": matches,
            "total": total
        }))
    }
}

fn build_regex(pattern: &str, case_sensitive: bool) -> ToolResult<Regex> {
    let pat = if case_sensitive {
        pattern.to_string()
    } else {
        format!("(?i){}", pattern)
    };
    Regex::new(&pat)
        .map_err(|e| ZcodeError::InvalidToolInput(format!("Invalid regex pattern: {}", e)))
}

fn search_file(
    path: &Path,
    pattern: &Regex,
    params: &SearchInput,
    matches: &mut Vec<SearchMatch>,
) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return, // Skip binary or unreadable files
    };

    let lines: Vec<&str> = content.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        if matches.len() >= params.max_matches {
            break;
        }
        if let Some(mat) = pattern.find(line) {
            let ctx_start = idx.saturating_sub(params.context_lines);
            let context_before: Vec<String> = lines[ctx_start..idx]
                .iter()
                .map(|s| s.to_string())
                .collect();

            let ctx_end = (idx + 1 + params.context_lines).min(lines.len());
            let context_after: Vec<String> = lines[(idx + 1)..ctx_end]
                .iter()
                .map(|s| s.to_string())
                .collect();

            matches.push(SearchMatch {
                file: path.to_string_lossy().to_string(),
                line: idx + 1,
                column: mat.start() + 1,
                text: line.to_string(),
                context_before,
                context_after,
            });
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_search_basic() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, "fn hello() {}\nfn world() {}\nfn hello_world() {}").unwrap();

        let result = SearchTool.execute(serde_json::json!({
            "pattern": "hello",
            "path": path.to_str().unwrap()
        })).unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert!(matches.len() >= 2);
    }

    #[test]
    fn test_search_case_insensitive_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "HELLO\nhello\nHeLLo").unwrap();

        let result = SearchTool.execute(serde_json::json!({
            "pattern": "hello",
            "path": path.to_str().unwrap()
        })).unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_search_case_sensitive() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "HELLO\nhello\nHeLLo").unwrap();

        let result = SearchTool.execute(serde_json::json!({
            "pattern": "hello",
            "path": path.to_str().unwrap(),
            "case_sensitive": true
        })).unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["line"], 2);
    }

    #[test]
    fn test_search_with_context_lines() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ctx.txt");
        fs::write(&path, "Line 1\nLine 2\nTARGET\nLine 4\nLine 5").unwrap();

        let result = SearchTool.execute(serde_json::json!({
            "pattern": "TARGET",
            "path": path.to_str().unwrap(),
            "context_lines": 1
        })).unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["context_before"].as_array().unwrap().len(), 1);
        assert_eq!(matches[0]["context_after"].as_array().unwrap().len(), 1);
        assert_eq!(matches[0]["context_before"][0], "Line 2");
        assert_eq!(matches[0]["context_after"][0], "Line 4");
    }

    #[test]
    fn test_search_directory() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.rs"), "fn foo() {}").unwrap();
        fs::write(dir.path().join("b.rs"), "fn bar() {}\nfn foo_helper() {}").unwrap();
        fs::write(dir.path().join("c.txt"), "no match here").unwrap();

        let result = SearchTool.execute(serde_json::json!({
            "pattern": "foo",
            "path": dir.path().to_str().unwrap()
        })).unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert!(matches.len() >= 2);
    }

    #[test]
    fn test_search_with_file_extension_filter() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("match.rs"), "fn foo() {}").unwrap();
        fs::write(dir.path().join("no_match.txt"), "fn foo() {}").unwrap();

        let result = SearchTool.execute(serde_json::json!({
            "pattern": "foo",
            "path": dir.path().to_str().unwrap(),
            "file_extension": ".rs"
        })).unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0]["file"].as_str().unwrap().ends_with(".rs"));
    }

    #[test]
    fn test_search_no_matches() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "Hello World").unwrap();

        let result = SearchTool.execute(serde_json::json!({
            "pattern": "nonexistent_pattern_xyz",
            "path": path.to_str().unwrap()
        })).unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert!(matches.is_empty());
        assert_eq!(result["total"], 0);
    }

    #[test]
    fn test_search_regex_pattern() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("code.rs");
        fs::write(&path, "let x = 42;\nlet y = 100;\nlet z = \"hello\";").unwrap();

        let result = SearchTool.execute(serde_json::json!({
            "pattern": r"let \w+ = \d+",
            "path": path.to_str().unwrap(),
            "case_sensitive": true
        })).unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 2); // x and y, not z (string)
    }

    #[test]
    fn test_search_invalid_regex() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "content").unwrap();

        let result = SearchTool.execute(serde_json::json!({
            "pattern": "[invalid regex",
            "path": path.to_str().unwrap()
        }));

        assert!(result.is_err());
    }

    #[test]
    fn test_search_line_and_column_numbers() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "first line\nsecond TARGET line").unwrap();

        let result = SearchTool.execute(serde_json::json!({
            "pattern": "TARGET",
            "path": path.to_str().unwrap(),
            "case_sensitive": true
        })).unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches[0]["line"], 2);
        assert_eq!(matches[0]["column"].as_u64().unwrap(), 8); // 1-indexed
    }

    #[test]
    fn test_search_nonexistent_path() {
        let result = SearchTool.execute(serde_json::json!({
            "pattern": "foo",
            "path": "/nonexistent/path"
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_search_max_matches() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("many.txt");
        let content: String = (0..200).map(|i| format!("line with pattern {}\n", i)).collect();
        fs::write(&path, &content).unwrap();

        let result = SearchTool.execute(serde_json::json!({
            "pattern": "pattern",
            "path": path.to_str().unwrap(),
            "max_matches": 10
        })).unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 10);
    }
}
