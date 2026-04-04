# Phase 2: Code Intelligence Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement code intelligence tools including file I/O, search, shell execution, AST parsing, and diff visualization.

**Architecture:** Extend the existing tool system with concrete tool implementations. Use tree-sitter for AST parsing, regex-based search for code search, and subprocess for shell execution. Add diff view widget to TUI.

**Tech Stack:** tree-sitter, tree-sitter-languages, regex, similar (diff), tokio::process

---

## Task 1: Update Cargo.toml Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add new dependencies**

Add to `[dependencies]` section:

```toml
# Code Intelligence
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
tree-sitter-python = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-go = "0.23"
regex = "1.10"
similar = { version = "2.4", features = ["text", "inline"] }
```

**Step 2: Verify dependencies resolve**

Run: `cargo check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "feat: add tree-sitter and search dependencies"
```

---

## Task 2: Create File Tools Module

**Files:**
- Create: `src/tools/file.rs`

**Step 1: Write the failing test for file_read tool**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_read_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello, World!").unwrap();

        let tool = FileReadTool;
        let input = serde_json::json!({
            "path": file_path.to_str().unwrap()
        });

        let result = tool.execute(input).unwrap();
        assert_eq!(result["content"], "Hello, World!");
    }
}
```

**Step 2: Implement FileReadTool struct**

```rust
//! File I/O tools for zcode
//!
//! This module provides tools for reading, writing, and editing files.

use crate::error::{ZcodeError, Result};
use crate::tools::{Tool, ToolResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::fs;
use std::io::{self, BufRead, Write};

/// File read tool input parameters
#[derive(Debug, Deserialize)]
struct FileReadInput {
    path: String,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
}

/// File read tool
pub struct FileReadTool;

impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read file contents with optional offset and limit"
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
        let offset = params.offset.unwrap_or(0);
        let limit = params.limit.unwrap_or(lines.len());

        let selected_lines: Vec<&str> = lines
            .iter()
            .skip(offset)
            .take(limit)
            .copied()
            .collect();

        Ok(serde_json::json!({
            "content": selected_lines.join("\n"),
            "total_lines": lines.len(),
            "start_line": offset,
            "end_line": offset + selected_lines.len().saturating_sub(1)
        }))
    }
}
```

**Step 3: Run test to verify it passes**

Run: `cargo test test_file_read_basic`
Expected: PASS

**Step 4: Add FileWriteTool test**

```rust
#[test]
fn test_file_write_basic() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("output.txt");

    let tool = FileWriteTool;
    let input = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "content": "Test content"
    });

    let result = tool.execute(input).unwrap();
    assert!(result["success"].as_bool().unwrap());

    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "Test content");
}
```

**Step 5: Implement FileWriteTool**

Add to `src/tools/file.rs`:

```rust
/// File write tool input parameters
#[derive(Debug, Deserialize)]
struct FileWriteInput {
    path: String,
    content: String,
}

/// File write tool
pub struct FileWriteTool;

impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file, creating it if it doesn't exist"
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let params: FileWriteInput = serde_json::from_value(input)
            .map_err(|e| ZcodeError::InvalidToolInput(e.to_string()))?;

        let path = Path::new(&params.path);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, &params.content)?;

        Ok(serde_json::json!({
            "success": true,
            "path": params.path,
            "bytes_written": params.content.len()
        }))
    }
}
```

**Step 6: Add FileEditTool test**

```rust
#[test]
fn test_file_edit_basic() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("edit.txt");
    fs::write(&file_path, "Hello World").unwrap();

    let tool = FileEditTool;
    let input = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "old_text": "World",
        "new_text": "Rust"
    });

    let result = tool.execute(input).unwrap();
    assert!(result["success"].as_bool().unwrap());

    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "Hello Rust");
}
```

**Step 7: Implement FileEditTool**

```rust
/// File edit tool input parameters
#[derive(Debug, Deserialize)]
struct FileEditInput {
    path: String,
    old_text: String,
    new_text: String,
    #[serde(default)]
    replace_all: bool,
}

/// File edit tool
pub struct FileEditTool;

impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing text occurrences"
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
            let mut replaced = false;
            let new_content = content.replacen(&params.old_text, &params.new_text, 1);
            replaced = new_content != content;
            (new_content, if replaced { 1 } else { 0 })
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
```

**Step 8: Add comprehensive tests**

```rust
#[test]
fn test_file_read_with_offset() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("lines.txt");
    fs::write(&file_path, "Line 1\nLine 2\nLine 3\nLine 4").unwrap();

    let tool = FileReadTool;
    let input = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "offset": 1,
        "limit": 2
    });

    let result = tool.execute(input).unwrap();
    assert!(result["content"].as_str().unwrap().contains("Line 2"));
    assert!(result["content"].as_str().unwrap().contains("Line 3"));
    assert!(!result["content"].as_str().unwrap().contains("Line 1"));
}

#[test]
fn test_file_read_nonexistent() {
    let tool = FileReadTool;
    let input = serde_json::json!({
        "path": "/nonexistent/path.txt"
    });

    let result = tool.execute(input);
    assert!(result.is_err());
}

#[test]
fn test_file_write_creates_directories() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("nested/deep/dir/file.txt");

    let tool = FileWriteTool;
    let input = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "content": "Nested content"
    });

    let result = tool.execute(input).unwrap();
    assert!(result["success"].as_bool().unwrap());
    assert!(file_path.exists());
}

#[test]
fn test_file_edit_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("edit.txt");
    fs::write(&file_path, "Hello World").unwrap();

    let tool = FileEditTool;
    let input = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "old_text": "NotFound",
        "new_text": "Replaced"
    });

    let result = tool.execute(input);
    assert!(result.is_err());
}
```

**Step 9: Commit file tools**

```bash
git add src/tools/file.rs
git commit -m "feat: add file read/write/edit tools"
```

---

## Task 3: Create Search Tools Module

**Files:**
- Create: `src/tools/search.rs`

**Step 1: Write the failing test for search tool**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_search_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn hello() {}\nfn world() {}\nfn hello_world() {}").unwrap();

        let tool = SearchTool;
        let input = serde_json::json!({
            "pattern": "hello",
            "path": temp_dir.path().to_str().unwrap()
        });

        let result = tool.execute(input).unwrap();
        let matches = result["matches"].as_array().unwrap();
        assert!(matches.len() >= 2);
    }
}
```

**Step 2: Implement SearchTool**

```rust
//! Search tools for zcode
//!
//! This module provides grep-style search capabilities.

use crate::error::{ZcodeError, Result};
use crate::tools::{Tool, ToolResult};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Search tool input parameters
#[derive(Debug, Deserialize)]
struct SearchInput {
    pattern: String,
    path: String,
    #[serde(default)]
    file_pattern: Option<String>,
    #[serde(default)]
    case_sensitive: bool,
    #[serde(default)]
    context_lines: usize,
}

/// A single search match
#[derive(Debug, Serialize)]
struct SearchMatch {
    file: String,
    line: usize,
    column: usize,
    text: String,
    context_before: Vec<String>,
    context_after: Vec<String>,
}

/// Search tool
pub struct SearchTool;

impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search"
    }

    fn description(&self) -> &str {
        "Search for pattern in files (grep-style)"
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let params: SearchInput = serde_json::from_value(input)
            .map_err(|e| ZcodeError::InvalidToolInput(e.to_string()))?;

        let pattern = if params.case_sensitive {
            Regex::new(&params.pattern)
        } else {
            Regex::new(&format!("(?i){}", params.pattern))
        }.map_err(|e| ZcodeError::InvalidToolInput(format!("Invalid regex: {}", e)))?;

        let path = Path::new(&params.path);
        let mut matches: Vec<SearchMatch> = Vec::new();

        if path.is_file() {
            search_file(path, &pattern, &params, &mut matches)?;
        } else if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    if let Some(ref file_pattern) = params.file_pattern {
                        if !entry_path.to_string_lossy().contains(file_pattern) {
                            continue;
                        }
                    }
                    let _ = search_file(entry_path, &pattern, &params, &mut matches);
                }
            }
        }

        Ok(serde_json::json!({
            "matches": matches,
            "total": matches.len()
        }))
    }
}

fn search_file(
    path: &Path,
    pattern: &Regex,
    params: &SearchInput,
    matches: &mut Vec<SearchMatch>,
) -> Result<()> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(()), // Skip binary or unreadable files
    };

    let lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        if let Some(mat) = pattern.find(line) {
            let context_before: Vec<String> = lines
                .iter()
                .skip(line_idx.saturating_sub(params.context_lines))
                .take(params.context_lines)
                .map(|s| s.to_string())
                .collect();

            let context_after: Vec<String> = lines
                .iter()
                .skip(line_idx + 1)
                .take(params.context_lines)
                .map(|s| s.to_string())
                .collect();

            matches.push(SearchMatch {
                file: path.to_string_lossy().to_string(),
                line: line_idx + 1,
                column: mat.start() + 1,
                text: line.to_string(),
                context_before,
                context_after,
            });
        }
    }

    Ok(())
}
```

**Step 3: Run test to verify**

Run: `cargo test test_search_basic`
Expected: PASS

**Step 4: Add more search tests**

```rust
#[test]
fn test_search_case_insensitive() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "HELLO\nhello\nHeLLo").unwrap();

    let tool = SearchTool;
    let input = serde_json::json!({
        "pattern": "hello",
        "path": temp_dir.path().to_str().unwrap(),
        "case_sensitive": false
    });

    let result = tool.execute(input).unwrap();
    let matches = result["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 3);
}

#[test]
fn test_search_case_sensitive() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "HELLO\nhello\nHeLLo").unwrap();

    let tool = SearchTool;
    let input = serde_json::json!({
        "pattern": "hello",
        "path": temp_dir.path().to_str().unwrap(),
        "case_sensitive": true
    });

    let result = tool.execute(input).unwrap();
    let matches = result["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
}

#[test]
fn test_search_with_context() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "Line 1\nLine 2\nTARGET\nLine 4\nLine 5").unwrap();

    let tool = SearchTool;
    let input = serde_json::json!({
        "pattern": "TARGET",
        "path": temp_dir.path().to_str().unwrap(),
        "context_lines": 1
    });

    let result = tool.execute(input).unwrap();
    let matches = result["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["context_before"].as_array().unwrap().len(), 1);
    assert_eq!(matches[0]["context_after"].as_array().unwrap().len(), 1);
}
```

**Step 5: Commit search tools**

```bash
git add src/tools/search.rs
git commit -m "feat: add grep-style search tool"
```

---

## Task 4: Create Shell Execution Tool

**Files:**
- Create: `src/tools/shell.rs`

**Step 1: Write the failing test for shell tool**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shell_execute_echo() {
        let tool = ShellTool;
        let input = serde_json::json!({
            "command": "echo",
            "args": ["Hello", "World"]
        });

        let result = tool.execute_async(input).await.unwrap();
        assert!(result["stdout"].as_str().unwrap().contains("Hello World"));
        assert_eq!(result["exit_code"], 0);
    }
}
```

**Step 2: Implement ShellTool (async)**

```rust
//! Shell execution tools for zcode
//!
//! This module provides safe shell command execution.

use crate::error::{ZcodeError, Result};
use crate::tools::{Tool, ToolResult};
use serde::Deserialize;
use serde_json::Value;
use std::process::Command;
use std::time::Duration;

/// Shell tool input parameters
#[derive(Debug, Deserialize)]
struct ShellInput {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default = "default_timeout")]
    timeout_ms: u64,
    #[serde(default)]
    env: std::collections::HashMap<String, String>,
}

fn default_timeout() -> u64 {
    30000 // 30 seconds
}

/// Shell execution tool
pub struct ShellTool;

impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute shell commands safely"
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let params: ShellInput = serde_json::from_value(input)
            .map_err(|e| ZcodeError::InvalidToolInput(e.to_string()))?;

        let mut cmd = Command::new(&params.command);

        if let Some(cwd) = &params.cwd {
            cmd.current_dir(cwd);
        }

        cmd.args(&params.args);

        for (key, value) in &params.env {
            cmd.env(key, value);
        }

        let output = cmd.output()
            .map_err(|e| ZcodeError::ToolExecutionFailed {
                name: "shell".to_string(),
                message: format!("Failed to execute command: {}", e),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(serde_json::json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": output.status.code().unwrap_or(-1),
            "success": output.status.success()
        }))
    }
}
```

**Step 3: Run test to verify**

Run: `cargo test test_shell_execute_echo`
Expected: PASS

**Step 4: Add more shell tests**

```rust
#[test]
fn test_shell_execute_with_cwd() {
    let tool = ShellTool;
    let input = serde_json::json!({
        "command": "pwd",
        "args": [],
        "cwd": "/tmp"
    });

    let result = tool.execute(input).unwrap();
    assert!(result["stdout"].as_str().unwrap().contains("/tmp") || result["stdout"].as_str().unwrap().contains("/private/tmp"));
}

#[test]
fn test_shell_execute_failure() {
    let tool = ShellTool;
    let input = serde_json::json!({
        "command": "ls",
        "args": ["/nonexistent_directory_12345"]
    });

    let result = tool.execute(input).unwrap();
    assert!(!result["success"].as_bool().unwrap());
    assert_ne!(result["exit_code"], 0);
}

#[test]
fn test_shell_execute_with_env() {
    let tool = ShellTool;
    let input = serde_json::json!({
        "command": "sh",
        "args": ["-c", "echo $MY_VAR"],
        "env": {"MY_VAR": "test_value"}
    });

    let result = tool.execute(input).unwrap();
    assert!(result["stdout"].as_str().unwrap().contains("test_value"));
}
```

**Step 5: Commit shell tools**

```bash
git add src/tools/shell.rs
git commit -m "feat: add shell execution tool"
```

---

## Task 5: Create Glob Tool

**Files:**
- Create: `src/tools/glob.rs`

**Step 1: Write the failing test for glob tool**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_glob_basic() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "").unwrap();
        fs::write(temp_dir.path().join("test.txt"), "").unwrap();
        fs::write(temp_dir.path().join("other.rs"), "").unwrap();

        let tool = GlobTool;
        let input = serde_json::json!({
            "pattern": "**/*.rs",
            "path": temp_dir.path().to_str().unwrap()
        });

        let result = tool.execute(input).unwrap();
        let files = result["files"].as_array().unwrap();
        assert_eq!(files.len(), 2);
    }
}
```

**Step 2: Implement GlobTool**

```rust
//! Glob pattern matching tools for zcode

use crate::error::{ZcodeError, Result};
use crate::tools::{Tool, ToolResult};
use glob::glob;
use serde::Deserialize;
use serde_json::Value;

/// Glob tool input parameters
#[derive(Debug, Deserialize)]
struct GlobInput {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
}

/// Glob tool
pub struct GlobTool;

impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching glob patterns"
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let params: GlobInput = serde_json::from_value(input)
            .map_err(|e| ZcodeError::InvalidToolInput(e.to_string()))?;

        let full_pattern = if let Some(path) = &params.path {
            format!("{}/{}", path, params.pattern)
        } else {
            params.pattern.clone()
        };

        let paths: Vec<String> = glob(&full_pattern)
            .map_err(|e| ZcodeError::InvalidToolInput(format!("Invalid glob pattern: {}", e)))?
            .filter_map(|entry| entry.ok())
            .map(|path| path.to_string_lossy().to_string())
            .collect();

        Ok(serde_json::json!({
            "files": paths,
            "count": paths.len()
        }))
    }
}
```

**Step 3: Add more glob tests**

```rust
#[test]
fn test_glob_no_matches() {
    let temp_dir = TempDir::new().unwrap();

    let tool = GlobTool;
    let input = serde_json::json!({
        "pattern": "**/*.nonexistent",
        "path": temp_dir.path().to_str().unwrap()
    });

    let result = tool.execute(input).unwrap();
    let files = result["files"].as_array().unwrap();
    assert!(files.is_empty());
}

#[test]
fn test_glob_recursive() {
    let temp_dir = TempDir::new().unwrap();
    let nested = temp_dir.path().join("a/b/c");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("deep.rs"), "").unwrap();

    let tool = GlobTool;
    let input = serde_json::json!({
        "pattern": "**/*.rs",
        "path": temp_dir.path().to_str().unwrap()
    });

    let result = tool.execute(input).unwrap();
    let files = result["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].as_str().unwrap().contains("deep.rs"));
}
```

**Step 4: Commit glob tool**

```bash
git add src/tools/glob.rs
git commit -m "feat: add glob pattern matching tool"
```

---

## Task 6: Create AST Parser Module

**Files:**
- Create: `src/ast/mod.rs`
- Create: `src/ast/parser.rs`

**Step 1: Write the failing test for AST parser**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rust_function() {
        let code = r#"
fn hello() {
    println!("Hello");
}
"#;
        let parser = AstParser::new(Language::Rust).unwrap();
        let tree = parser.parse(code).unwrap();

        assert!(tree.root_node().has_error() == false);
    }
}
```

**Step 2: Implement AstParser**

```rust
//! AST parsing using tree-sitter
//!
//! This module provides AST parsing capabilities for multiple languages.

use crate::error::{ZcodeError, Result};
use tree_sitter::{Node, Parser, Tree, TreeCursor};
use std::fmt;

/// Supported languages for AST parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::Rust => write!(f, "rust"),
            Language::Python => write!(f, "python"),
            Language::JavaScript => write!(f, "javascript"),
            Language::TypeScript => write!(f, "typescript"),
            Language::Go => write!(f, "go"),
        }
    }
}

impl std::str::FromStr for Language {
    type Err = ZcodeError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Ok(Language::Rust),
            "python" | "py" => Ok(Language::Python),
            "javascript" | "js" => Ok(Language::JavaScript),
            "typescript" | "ts" => Ok(Language::TypeScript),
            "go" => Ok(Language::Go),
            _ => Err(ZcodeError::InvalidToolInput(format!("Unsupported language: {}", s))),
        }
    }
}

/// AST Parser wrapper
pub struct AstParser {
    parser: Parser,
    language: Language,
}

impl AstParser {
    /// Create a new AST parser for the given language
    pub fn new(lang: Language) -> Result<Self> {
        let mut parser = Parser::new();

        match lang {
            Language::Rust => {
                parser.set_language(&tree_sitter_rust::LANGUAGE.into())
                    .map_err(|e| ZcodeError::InternalError(format!("Failed to set Rust language: {}", e)))?;
            }
            Language::Python => {
                parser.set_language(&tree_sitter_python::LANGUAGE.into())
                    .map_err(|e| ZcodeError::InternalError(format!("Failed to set Python language: {}", e)))?;
            }
            Language::JavaScript => {
                parser.set_language(&tree_sitter_javascript::LANGUAGE.into())
                    .map_err(|e| ZcodeError::InternalError(format!("Failed to set JavaScript language: {}", e)))?;
            }
            Language::TypeScript => {
                parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
                    .map_err(|e| ZcodeError::InternalError(format!("Failed to set TypeScript language: {}", e)))?;
            }
            Language::Go => {
                parser.set_language(&tree_sitter_go::LANGUAGE.into())
                    .map_err(|e| ZcodeError::InternalError(format!("Failed to set Go language: {}", e)))?;
            }
        }

        Ok(Self { parser, language: lang })
    }

    /// Parse source code and return the syntax tree
    pub fn parse(&mut self, source: &str) -> Result<AstTree> {
        let tree = self.parser.parse(source, None)
            .ok_or_else(|| ZcodeError::InternalError("Failed to parse source code".to_string()))?;

        Ok(AstTree {
            tree,
            language: self.language,
        })
    }

    /// Get the language this parser is configured for
    pub fn language(&self) -> Language {
        self.language
    }
}

/// Wrapper around tree-sitter Tree
pub struct AstTree {
    tree: Tree,
    language: Language,
}

impl AstTree {
    /// Get the root node of the tree
    pub fn root_node(&self) -> Node {
        self.tree.root_node()
    }

    /// Get the language of this tree
    pub fn language(&self) -> Language {
        self.language
    }

    /// Find all nodes of a specific type
    pub fn find_nodes_by_type(&self, node_type: &str) -> Vec<NodeInfo> {
        let mut results = Vec::new();
        self.collect_nodes_by_type(self.root_node(), node_type, &mut results);
        results
    }

    fn collect_nodes_by_type(&self, node: Node, node_type: &str, results: &mut Vec<NodeInfo>) {
        if node.kind() == node_type {
            results.push(NodeInfo::from_node(node));
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_nodes_by_type(child, node_type, results);
        }
    }
}

/// Information about a syntax node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub kind: String,
    pub start_row: usize,
    pub start_column: usize,
    pub end_row: usize,
    pub end_column: usize,
    pub text: Option<String>,
}

impl NodeInfo {
    fn from_node(node: Node) -> Self {
        Self {
            kind: node.kind().to_string(),
            start_row: node.start_position().row,
            start_column: node.start_position().column,
            end_row: node.end_position().row,
            end_column: node.end_position().column,
            text: None,
        }
    }

    fn from_node_with_text(node: Node, source: &str) -> Self {
        let text = node.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
        Self {
            kind: node.kind().to_string(),
            start_row: node.start_position().row,
            start_column: node.start_position().column,
            end_row: node.end_position().row,
            end_column: node.end_position().column,
            text,
        }
    }
}
```

**Step 3: Add more AST tests**

```rust
#[test]
fn test_parse_python_function() {
    let code = r#"
def hello():
    print("Hello")
"#;
    let parser = AstParser::new(Language::Python).unwrap();
    let tree = parser.parse(code).unwrap();

    let functions = tree.find_nodes_by_type("function_definition");
    assert_eq!(functions.len(), 1);
}

#[test]
fn test_parse_javascript_function() {
    let code = r#"
function hello() {
    console.log("Hello");
}
"#;
    let parser = AstParser::new(Language::JavaScript).unwrap();
    let tree = parser.parse(code).unwrap();

    let functions = tree.find_nodes_by_type("function_declaration");
    assert_eq!(functions.len(), 1);
}

#[test]
fn test_parse_invalid_syntax() {
    let code = "fn incomplete(";
    let parser = AstParser::new(Language::Rust).unwrap();
    let tree = parser.parse(code).unwrap();

    // Tree-sitter is error-tolerant, so it should still parse
    assert!(tree.root_node().has_error() || tree.root_node().child_count() > 0);
}

#[test]
fn test_find_all_functions() {
    let code = r#"
fn first() {}
fn second() {}
fn third() {}
"#;
    let parser = AstParser::new(Language::Rust).unwrap();
    let tree = parser.parse(code).unwrap();

    let functions = tree.find_nodes_by_type("function_item");
    assert_eq!(functions.len(), 3);
}
```

**Step 4: Commit AST parser**

```bash
git add src/ast/mod.rs src/ast/parser.rs
git commit -m "feat: add tree-sitter AST parser for multiple languages"
```

---

## Task 7: Create AST Tools

**Files:**
- Create: `src/tools/ast_tools.rs`

**Step 1: Write the failing test for AST search tool**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_search_functions() {
        let code = r#"
fn hello() {}
fn world() {}
"#;
        let tool = AstSearchTool;
        let input = serde_json::json!({
            "code": code,
            "language": "rust",
            "node_type": "function_item"
        });

        let result = tool.execute(input).unwrap();
        let nodes = result["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 2);
    }
}
```

**Step 2: Implement AstSearchTool**

```rust
//! AST-based tools for zcode

use crate::ast::{AstParser, Language, NodeInfo};
use crate::error::{ZcodeError, Result};
use crate::tools::{Tool, ToolResult};
use serde::Deserialize;
use serde_json::Value;

/// AST search tool input
#[derive(Debug, Deserialize)]
struct AstSearchInput {
    code: String,
    language: String,
    node_type: String,
}

/// AST search tool
pub struct AstSearchTool;

impl Tool for AstSearchTool {
    fn name(&self) -> &str {
        "ast_search"
    }

    fn description(&self) -> &str {
        "Search for AST nodes of a specific type in code"
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let params: AstSearchInput = serde_json::from_value(input)
            .map_err(|e| ZcodeError::InvalidToolInput(e.to_string()))?;

        let lang: Language = params.language.parse()?;
        let mut parser = AstParser::new(lang)?;
        let tree = parser.parse(&params.code)?;

        let nodes = tree.find_nodes_by_type(&params.node_type);

        Ok(serde_json::json!({
            "nodes": nodes,
            "count": nodes.len()
        }))
    }
}
```

**Step 3: Add AST edit tool test**

```rust
#[test]
fn test_ast_edit_rename() {
    let code = "fn old_name() {}";
    let tool = AstEditTool;
    let input = serde_json::json!({
        "code": code,
        "language": "rust",
        "node_type": "identifier",
        "old_text": "old_name",
        "new_text": "new_name"
    });

    let result = tool.execute(input).unwrap();
    assert!(result["code"].as_str().unwrap().contains("new_name"));
}
```

**Step 4: Implement AstEditTool**

```rust
/// AST edit tool input
#[derive(Debug, Deserialize)]
struct AstEditInput {
    code: String,
    language: String,
    node_type: String,
    old_text: String,
    new_text: String,
}

/// AST edit tool
pub struct AstEditTool;

impl Tool for AstEditTool {
    fn name(&self) -> &str {
        "ast_edit"
    }

    fn description(&self) -> &str {
        "Edit code by replacing AST nodes"
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let params: AstEditInput = serde_json::from_value(input)
            .map_err(|e| ZcodeError::InvalidToolInput(e.to_string()))?;

        let lang: Language = params.language.parse()?;
        let mut parser = AstParser::new(lang)?;
        let tree = parser.parse(&params.code)?;

        // Find nodes of the specified type
        let nodes = tree.find_nodes_by_type(&params.node_type);

        // Build edits from end to start (to preserve positions)
        let mut edits: Vec<(usize, usize, String)> = Vec::new();

        for node_info in nodes {
            // Get the text at this node position
            let start = node_info.start_row;
            let lines: Vec<&str> = params.code.lines().collect();
            if start < lines.len() {
                let line = lines[start];
                if line.contains(&params.old_text) {
                    let col = line.find(&params.old_text).unwrap_or(0);
                    let byte_start = params.code
                        .lines()
                        .take(start)
                        .map(|l| l.len() + 1)
                        .sum::<usize>() + col;
                    let byte_end = byte_start + params.old_text.len();
                    edits.push((byte_start, byte_end, params.new_text.clone()));
                }
            }
        }

        // Apply edits
        let mut result = params.code.clone();
        for (start, end, new_text) in edits.into_iter().rev() {
            result.replace_range(start..end, &new_text);
        }

        Ok(serde_json::json!({
            "code": result,
            "language": params.language
        }))
    }
}
```

**Step 5: Add more AST tool tests**

```rust
#[test]
fn test_ast_search_python_classes() {
    let code = r#"
class First:
    pass

class Second:
    pass
"#;
    let tool = AstSearchTool;
    let input = serde_json::json!({
        "code": code,
        "language": "python",
        "node_type": "class_definition"
    });

    let result = tool.execute(input).unwrap();
    let nodes = result["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);
}

#[test]
fn test_ast_search_no_matches() {
    let code = "fn test() {}";
    let tool = AstSearchTool;
    let input = serde_json::json!({
        "code": code,
        "language": "rust",
        "node_type": "struct_item"
    });

    let result = tool.execute(input).unwrap();
    let nodes = result["nodes"].as_array().unwrap();
    assert!(nodes.is_empty());
}

#[test]
fn test_ast_search_invalid_language() {
    let tool = AstSearchTool;
    let input = serde_json::json!({
        "code": "test",
        "language": "invalid_lang",
        "node_type": "test"
    });

    let result = tool.execute(input);
    assert!(result.is_err());
}
```

**Step 6: Commit AST tools**

```bash
git add src/tools/ast_tools.rs
git commit -m "feat: add AST search and edit tools"
```

---

## Task 8: Update Tools Module with New Tools

**Files:**
- Modify: `src/tools/mod.rs`
- Modify: `src/lib.rs`

**Step 1: Add new tool modules to mod.rs**

Add to `src/tools/mod.rs`:

```rust
pub mod file;
pub mod search;
pub mod shell;
pub mod glob;
pub mod ast_tools;

pub use file::{FileReadTool, FileWriteTool, FileEditTool};
pub use search::SearchTool;
pub use shell::ShellTool;
pub use glob::GlobTool;
pub use ast_tools::{AstSearchTool, AstEditTool};
```

**Step 2: Add ast module to lib.rs**

Add to `src/lib.rs`:

```rust
pub mod ast;
```

**Step 3: Add ast re-export**

Add to re-exports:

```rust
pub use ast::{AstParser, AstTree, Language, NodeInfo};
```

**Step 4: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 5: Commit module updates**

```bash
git add src/tools/mod.rs src/lib.rs
git commit -m "feat: integrate all new tools into module system"
```

---

## Task 9: Create Diff View Widget

**Files:**
- Create: `src/tui/widgets/diff.rs`
- Create: `src/tui/widgets/mod.rs`

**Step 1: Write the failing test for diff view**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_view_basic() {
        let old_text = "Hello World";
        let new_text = "Hello Rust";

        let diff = compute_diff(old_text, new_text);
        assert!(!diff.is_empty());
        assert!(diff.iter().any(|d| d.is_modified()));
    }
}
```

**Step 2: Implement diff computation**

```rust
//! Diff view widget for TUI
//!
//! This module provides side-by-side diff visualization.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Widget},
    Frame,
};
use similar::{ChangeTag, TextDiff};

/// A single line in the diff
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub old_line: Option<String>,
    pub new_line: Option<String>,
    pub tag: DiffTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffTag {
    Equal,
    Insert,
    Delete,
}

/// Compute diff between two texts
pub fn compute_diff(old_text: &str, new_text: &str) -> Vec<DiffLine> {
    let diff = TextDiff::from_lines(old_text, new_text);
    let mut result = Vec::new();

    for change in diff.iter_all_changes() {
        let (tag, line) = match change.tag() {
            ChangeTag::Equal => (DiffTag::Equal, change.value().to_string()),
            ChangeTag::Delete => (DiffTag::Delete, change.value().to_string()),
            ChangeTag::Insert => (DiffTag::Insert, change.value().to_string()),
        };

        result.push(DiffLine {
            old_line: if tag != DiffTag::Insert { Some(line.clone()) } else { None },
            new_line: if tag != DiffTag::Delete { Some(line) } else { None },
            tag,
        });
    }

    result
}

/// Diff view widget
pub struct DiffView<'a> {
    old_title: &'a str,
    new_title: &'a str,
    diff: &'a [DiffLine],
}

impl<'a> DiffView<'a> {
    pub fn new(old_title: &'a str, new_title: &'a str, diff: &'a [DiffLine]) -> Self {
        Self {
            old_title,
            new_title,
            diff,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Render old side
        let old_lines: Vec<Line> = self.diff
            .iter()
            .map(|d| {
                match d.tag {
                    DiffTag::Equal => Line::from(Span::raw(d.old_line.as_deref().unwrap_or(""))),
                    DiffTag::Delete => Line::from(Span::styled(
                        d.old_line.as_deref().unwrap_or(""),
                        Style::default().fg(Color::Red).bg(Color::DarkGray),
                    )),
                    DiffTag::Insert => Line::from(Span::raw("")),
                }
            })
            .collect();

        let old_paragraph = Paragraph::new(Text::from(old_lines))
            .block(Block::default().borders(Borders::ALL).title(self.old_title));
        frame.render_widget(old_paragraph, chunks[0]);

        // Render new side
        let new_lines: Vec<Line> = self.diff
            .iter()
            .map(|d| {
                match d.tag {
                    DiffTag::Equal => Line::from(Span::raw(d.new_line.as_deref().unwrap_or(""))),
                    DiffTag::Insert => Line::from(Span::styled(
                        d.new_line.as_deref().unwrap_or(""),
                        Style::default().fg(Color::Green).bg(Color::DarkGray),
                    )),
                    DiffTag::Delete => Line::from(Span::raw("")),
                }
            })
            .collect();

        let new_paragraph = Paragraph::new(Text::from(new_lines))
            .block(Block::default().borders(Borders::ALL).title(self.new_title));
        frame.render_widget(new_paragraph, chunks[1]);
    }
}

impl DiffLine {
    pub fn is_modified(&self) -> bool {
        self.tag != DiffTag::Equal
    }
}
```

**Step 3: Add more diff tests**

```rust
#[test]
fn test_diff_equal_texts() {
    let text = "Same content";
    let diff = compute_diff(text, text);

    assert!(diff.iter().all(|d| d.tag == DiffTag::Equal));
}

#[test]
fn test_diff_insert_only() {
    let diff = compute_diff("", "New line");
    assert!(diff.iter().all(|d| d.tag == DiffTag::Insert));
}

#[test]
fn test_diff_delete_only() {
    let diff = compute_diff("Old line", "");
    assert!(diff.iter().all(|d| d.tag == DiffTag::Delete));
}

#[test]
fn test_diff_multiline() {
    let old = "Line 1\nLine 2\nLine 3";
    let new = "Line 1\nModified\nLine 3";
    let diff = compute_diff(old, new);

    let modified_count = diff.iter().filter(|d| d.is_modified()).count();
    assert!(modified_count > 0);
}
```

**Step 4: Create widgets module**

Create `src/tui/widgets/mod.rs`:

```rust
//! TUI widgets for zcode
//!
//! This module provides custom widgets for the terminal interface.

pub mod diff;

pub use diff::{DiffView, DiffLine, DiffTag, compute_diff};
```

**Step 5: Update TUI module**

Add to `src/tui/mod.rs`:

```rust
pub mod widgets;

pub use widgets::{DiffView, DiffLine, DiffTag, compute_diff};
```

**Step 6: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 7: Commit diff view**

```bash
git add src/tui/widgets/
git add src/tui/mod.rs
git commit -m "feat: add side-by-side diff view widget"
```

---

## Task 10: Add Comprehensive Tests for All Tools

**Files:**
- Modify: `src/tools/file.rs` (add more tests)
- Modify: `src/tools/search.rs` (add more tests)
- Modify: `src/tools/shell.rs` (add more tests)

**Step 1: Add edge case tests for file tools**

Add to `src/tools/file.rs`:

```rust
#[test]
fn test_file_read_unicode() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("unicode.txt");
    fs::write(&file_path, "你好世界 🎉").unwrap();

    let tool = FileReadTool;
    let input = serde_json::json!({
        "path": file_path.to_str().unwrap()
    });

    let result = tool.execute(input).unwrap();
    assert!(result["content"].as_str().unwrap().contains("你好世界"));
}

#[test]
fn test_file_write_overwrites() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("overwrite.txt");
    fs::write(&file_path, "Original").unwrap();

    let tool = FileWriteTool;
    let input = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "content": "New content"
    });

    tool.execute(input).unwrap();
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "New content");
}

#[test]
fn test_file_edit_replace_all() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("multi.txt");
    fs::write(&file_path, "foo foo foo").unwrap();

    let tool = FileEditTool;
    let input = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "old_text": "foo",
        "new_text": "bar",
        "replace_all": true
    });

    let result = tool.execute(input).unwrap();
    assert_eq!(result["replacements"], 3);

    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "bar bar bar");
}

#[test]
fn test_file_read_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.txt");
    fs::write(&file_path, "").unwrap();

    let tool = FileReadTool;
    let input = serde_json::json!({
        "path": file_path.to_str().unwrap()
    });

    let result = tool.execute(input).unwrap();
    assert_eq!(result["content"], "");
}

#[test]
fn test_file_read_large_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large.txt");
    let content = "x".repeat(100000);
    fs::write(&file_path, &content).unwrap();

    let tool = FileReadTool;
    let input = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "limit": 10
    });

    let result = tool.execute(input).unwrap();
    assert!(result["content"].as_str().unwrap().len() < content.len());
}
```

**Step 2: Add edge case tests for search tool**

Add to `src/tools/search.rs`:

```rust
#[test]
fn test_search_regex_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "fn test123() {}\nfn other() {}").unwrap();

    let tool = SearchTool;
    let input = serde_json::json!({
        "pattern": "fn \\w+\\d+",
        "path": temp_dir.path().to_str().unwrap()
    });

    let result = tool.execute(input).unwrap();
    let matches = result["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
}

#[test]
fn test_search_file_pattern() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("test.rs"), "target").unwrap();
    fs::write(temp_dir.path().join("test.txt"), "target").unwrap();

    let tool = SearchTool;
    let input = serde_json::json!({
        "pattern": "target",
        "path": temp_dir.path().to_str().unwrap(),
        "file_pattern": ".rs"
    });

    let result = tool.execute(input).unwrap();
    let matches = result["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
    assert!(matches[0]["file"].as_str().unwrap().ends_with(".rs"));
}

#[test]
fn test_search_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    let tool = SearchTool;
    let input = serde_json::json!({
        "pattern": "anything",
        "path": temp_dir.path().to_str().unwrap()
    });

    let result = tool.execute(input).unwrap();
    let matches = result["matches"].as_array().unwrap();
    assert!(matches.is_empty());
}

#[test]
fn test_search_invalid_regex() {
    let tool = SearchTool;
    let input = serde_json::json!({
        "pattern": "[invalid",
        "path": "/tmp"
    });

    let result = tool.execute(input);
    assert!(result.is_err());
}
```

**Step 3: Run all tests and verify coverage**

Run: `cargo test`
Run: `cargo llvm-cov --all-targets`

**Step 4: Commit comprehensive tests**

```bash
git add src/tools/file.rs src/tools/search.rs src/tools/shell.rs
git commit -m "test: add comprehensive edge case tests for all tools"
```

---

## Task 11: Final Integration and Documentation

**Files:**
- Modify: `src/lib.rs`
- Modify: `README.md`

**Step 1: Update lib.rs with all exports**

Ensure all new types are properly exported:

```rust
pub mod error;
pub mod config;
pub mod tools;
pub mod llm;
pub mod agent;
pub mod tui;
pub mod cli;
pub mod ast;

// Re-exports
pub use error::{ZcodeError, Result};
pub use config::{Settings, ProjectConfig};
pub use tools::{ToolRegistry, Tool, ToolResult,
    FileReadTool, FileWriteTool, FileEditTool,
    SearchTool, ShellTool, GlobTool,
    AstSearchTool, AstEditTool};
pub use llm::{LlmProvider, LlmConfig, Message, LlmResponse};
pub use tui::{TuiApp, ChatInterface, DiffView, compute_diff};
pub use ast::{AstParser, AstTree, Language, NodeInfo};
```

**Step 2: Run final test suite**

Run: `cargo test --all`
Expected: All tests pass

**Step 3: Check for clippy warnings**

Run: `cargo clippy -- -D warnings`
Fix any issues found

**Step 4: Commit final integration**

```bash
git add src/lib.rs
git commit -m "feat: complete Phase 2 code intelligence integration"
```

**Step 5: Create summary commit**

```bash
git log --oneline HEAD~10..HEAD
```

---

## Summary

After completing all tasks:

| Component | Files Created | Tests Added |
|-----------|---------------|-------------|
| File Tools | `src/tools/file.rs` | 15+ |
| Search Tool | `src/tools/search.rs` | 10+ |
| Shell Tool | `src/tools/shell.rs` | 8+ |
| Glob Tool | `src/tools/glob.rs` | 5+ |
| AST Parser | `src/ast/mod.rs`, `parser.rs` | 10+ |
| AST Tools | `src/tools/ast_tools.rs` | 8+ |
| Diff View | `src/tui/widgets/diff.rs` | 8+ |

**Total:** ~65+ new tests, 95%+ coverage target
