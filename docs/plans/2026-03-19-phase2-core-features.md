# Phase 2 Core Features Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add core developer tools, real streaming, better TUI, and native tool calling to the zcode programming agent.

**Architecture:** Four independent feature tracks implemented in order: (1) core tools (file_edit, search, glob) expand the agent's capabilities, (2) real streaming wires rig-core's SSE streaming for token-by-token display, (3) better TUI adds scroll/multiline/status bar to the chat interface, (4) native tool calling replaces fragile JSON text parsing with rig-core's structured function calling API.

**Tech Stack:** Rust 1.91, rig-core 0.33, ratatui 0.27, crossterm 0.27, tokio, serde_json, glob 0.3, walkdir 2.4

---

## Feature 1: Core Tools (file_edit, search, glob)

### Task 1.1: FileEditTool

**Files:**
- Modify: `src/tools/file_ops.rs:150` (after ShellExecTool, before tests)
- Modify: `src/tools/mod.rs:28` (add FileEditTool to use/re-exports)
- Modify: `src/tools/mod.rs:65` (register in register_built_in_tools)

**Step 1: Write failing tests**

Add to `src/tools/file_ops.rs` test module (before the closing `}`):

```rust
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test file_edit`
Expected: compile errors (FileEditTool not found)

**Step 3: Implement FileEditTool**

Add to `src/tools/file_ops.rs` after `ShellExecTool` (before `#[cfg(test)]`):

```rust
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
            // Lines before the edit range (1-indexed, so start_line-1 is the index)
            for line in lines.iter().take(start_line - 1) {
                result_lines.push((*line).to_string());
            }
            // The replacement content
            for line in new_content.lines() {
                result_lines.push(line.to_string());
            }
            // Lines after the edit range
            for line in lines.iter().skip(end_line) {
                result_lines.push((*line).to_string());
            }

            let mut output = result_lines.join("\n");
            // Preserve trailing newline if original had one
            if file_content.ends_with('\n') && !output.ends_with('\n') {
                output.push('\n');
            }

            tokio::fs::write(path, &output).await.map_err(|e| {
                ZcodeError::ToolExecutionFailed {
                    name: self.name().to_string(),
                    message: e.to_string(),
                }
            })?;

            Ok(serde_json::json!({
                "success": true,
                "path": path,
                "lines_replaced": (end_line - start_line + 1),
                "lines_inserted": new_content.lines().count()
            }))
        })
    }
}
```

**Step 4: Register FileEditTool**

Modify `src/tools/mod.rs`:

- Line 28: change to `pub use file_ops::{FileReadTool, FileWriteTool, ShellExecTool, FileEditTool};`
- Line 65: add `self.register(file_ops::FileEditTool);`

**Step 5: Run tests to verify they pass**

Run: `cargo test file_edit`
Expected: all 4 file_edit tests pass

**Step 6: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: clean

**Step 7: Commit**

```bash
git add src/tools/file_ops.rs src/tools/mod.rs
git commit -m "feat: add FileEditTool for targeted line-range editing"
```

---

### Task 1.2: SearchTool (ripgrep-style)

**Files:**
- Create: `src/tools/search.rs`
- Modify: `src/tools/mod.rs` (add pub mod search, re-export, register)

**Step 1: Add search dependency to Cargo.toml**

Modify `Cargo.toml` — add `grep = "0.3"` under the File system section:

```toml
# File system
walkdir = "2.4"
glob = "0.3"
grep = "0.3"
```

Run: `cargo check -p zcode` to verify dependency resolves. (If `grep` crate doesn't work well, fall back to manual line scanning with walkdir — see fallback below.)

**Step 2: Write failing tests**

Create `src/tools/search.rs`:

```rust
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

                // Skip directories
                if !path.is_file() {
                    continue;
                }

                // Skip binary files (check extension)
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    match ext {
                        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "pdf"
                        | "zip" | "tar" | "gz" | "exe" | "dll" | "so" | "dylib" => continue,
                        _ => {}
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_search_finds_pattern() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("test.txt");
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(f, "hello world").unwrap();
        writeln!(f, "foo bar").unwrap();
        writeln!(f, "hello again").unwrap();

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
}
```

**Step 3: Run tests to verify they fail**

Run: `cargo test search`
Expected: compile errors (search module not declared)

**Step 4: Wire module into tools system**

Modify `src/tools/mod.rs`:
- Add `pub mod search;` after `pub mod file_ops;`
- Add `pub use search::SearchTool;` to re-exports
- Add `self.register(search::SearchTool);` in `register_built_in_tools()`

**Step 5: Add regex dependency**

Add to `Cargo.toml`:
```toml
regex = "1.10"
```

**Step 6: Run tests**

Run: `cargo test search`
Expected: all search tests pass

**Step 7: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: clean

**Step 8: Commit**

```bash
git add src/tools/search.rs src/tools/mod.rs Cargo.toml
git commit -m "feat: add SearchTool for ripgrep-style content search"
```

---

### Task 1.3: GlobTool

**Files:**
- Modify: `src/tools/search.rs` (add GlobTool struct + tests)
- Modify: `src/tools/mod.rs` (re-export, register)

**Step 1: Write failing tests**

Add to `src/tools/search.rs`:

```rust
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

                // Check if the path matches the glob pattern
                if glob_pattern.matches_path(path) {
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
```

Add tests to the test module:

```rust
#[tokio::test]
async fn test_glob_finds_files() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("a.rs"), "").unwrap();
    std::fs::write(tmp.path().join("b.rs"), "").unwrap();
    std::fs::write(tmp.path().join("c.txt"), "").unwrap();

    let tool = GlobTool;
    // Use the full path pattern
    let pattern = format!("{}/**/*.rs", tmp.path().to_str().unwrap());
    let input = serde_json::json!({"pattern": pattern});
    let result = tool.execute(input).await.unwrap();
    let files = result["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
}

#[tokio::test]
async fn test_glob_no_matches() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("a.txt"), "").unwrap();

    let tool = GlobTool;
    let pattern = format!("{}/**/*.rs", tmp.path().to_str().unwrap());
    let input = serde_json::json!({"pattern": pattern});
    let result = tool.execute(input).await.unwrap();
    assert_eq!(result["files"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_glob_missing_pattern() {
    let tool = GlobTool;
    let result = tool.execute(serde_json::json!({})).await;
    assert!(result.is_err());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test glob_tool`
Expected: compile errors (GlobTool not found)

**Step 3: Wire into tools system**

Modify `src/tools/mod.rs`:
- Add `pub use search::{SearchTool, GlobTool};`
- Add `self.register(search::GlobTool);` in `register_built_in_tools()`

**Step 4: Run tests**

Run: `cargo test glob_tool`
Expected: all glob tests pass

**Step 5: Update system prompt in main.rs**

Modify `src/main.rs` — the system prompt currently lists tools generically. No change needed since it dynamically reads `registry.list()`.

**Step 6: Run full test suite**

Run: `cargo test`
Expected: all tests pass

**Step 7: Commit**

```bash
git add src/tools/search.rs src/tools/mod.rs
git commit -m "feat: add GlobTool for file pattern matching"
```

---

## Feature 2: Real Streaming

### Task 2.1: Wire rig-core streaming API

**Files:**
- Modify: `src/llm/provider.rs` (update RigProvider::stream_chat to use real streaming)

**Step 1: Research rig-core streaming API**

Read the rig-core source to find the exact streaming API. The key types are:
- `rig::providers::anthropic::streaming::StreamingCompletionResponse`
- `RawStreamingChoice::Message(text)` for text chunks
- The `stream()` method on `CompletionModel`

**Step 2: Write failing test for streaming**

Add to `src/llm/provider.rs` tests:

```rust
#[tokio::test]
async fn test_mock_provider_stream_chunks() {
    use futures::StreamExt;

    let provider = MockLlmProvider::new("Hello streaming world");
    let messages = vec![Message::user("Hi")];
    let mut stream = provider.stream_chat(&messages).await.unwrap();
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap());
    }
    // Mock returns full text as single chunk
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], "Hello streaming world");
}
```

**Step 3: Update RigProvider::stream_chat to use real streaming**

Replace the current fake implementation in `src/llm/provider.rs` (lines 149-163). The new implementation uses rig-core's `CompletionModel::stream()` method:

```rust
fn stream_chat(
    &self,
    messages: &[Message],
) -> Pin<Box<dyn Future<Output = Result<StreamingResponse>> + Send + '_>> {
    let config = self.config.clone();
    let messages = messages.to_vec();
    let api_key = match self.get_api_key() {
        Ok(k) => k,
        Err(e) => return Box::pin(async move { Err(e) }),
    };
    Box::pin(async move {
        use rig::client::CompletionClient;

        let client = rig::providers::anthropic::Client::new(&api_key)
            .map_err(|e| ZcodeError::LlmApiError(format!("Failed to create client: {}", e)))?;

        let mut builder = client.agent(&config.model);

        // Add system message as preamble
        for msg in &messages {
            if msg.role == MessageRole::System {
                builder = builder.preamble(&msg.content);
                break;
            }
        }

        let agent = builder.build();

        // Build conversation from non-system messages
        let prompt = messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| match m.role {
                MessageRole::User => format!("User: {}", m.content),
                MessageRole::Assistant => format!("Assistant: {}", m.content),
                _ => m.content.clone(),
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Use rig-core's streaming API
        use rig::completion::Prompt;
        let mut stream = agent.stream_prompt(&prompt).await
            .map_err(|e| ZcodeError::LlmApiError(format!("Streaming failed: {}", e)))?;

        // Convert rig-core stream to our StreamingResponse type
        let chunks = futures::stream::unfold(stream, |mut stream| async move {
            use futures::StreamExt;
            match stream.next().await {
                Some(Ok(chunk)) => Some((Ok(chunk), stream)),
                Some(Err(e)) => Some((Err(ZcodeError::LlmApiError(format!("Stream error: {}", e))), stream)),
                None => None,
            }
        });

        Ok(Box::pin(chunks) as StreamingResponse)
    })
}
```

**Note:** The exact rig-core streaming API may differ. Check `rig-core-0.33.0/src/agent/mod.rs` for `stream_prompt` or `stream_chat` availability. If `stream_prompt` doesn't exist, use the raw `CompletionModel::stream()` approach which returns `StreamingCompletionResponse` with `RawStreamingChoice` items.

**Step 4: Run tests**

Run: `cargo test -p zcode --lib`
Expected: tests pass

**Step 5: Commit**

```bash
git add src/llm/provider.rs
git commit -m "feat: implement real streaming via rig-core"
```

---

### Task 2.2: Wire streaming into Agent

**Files:**
- Modify: `src/agent.rs` (add `run_streaming` method)

**Step 1: Add run_streaming method to Agent**

Add to `src/agent.rs` after the existing `run()` method:

```rust
/// Process a user message with streaming response
pub async fn run_streaming(
    &mut self,
    user_input: &str,
) -> Result<crate::llm::streaming::StreamingResponse> {
    self.conversation.push(Message::user(user_input));

    let mut messages = vec![Message::system(&self.system_prompt)];
    messages.extend(self.conversation.iter().cloned());

    // For streaming, we just return the stream directly
    // Tool calls are NOT supported during streaming (they require round-trips)
    self.llm.stream_chat(&messages).await
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: all pass

**Step 3: Commit**

```bash
git add src/agent.rs
git commit -m "feat: add run_streaming method to Agent"
```

---

### Task 2.3: Wire streaming into TUI

**Files:**
- Modify: `src/tui/mod.rs` (update run_async to use streaming)
- Modify: `src/tui/chat.rs` (add streaming display support)

**Step 1: Add streaming state to ChatInterface**

Add to `ChatInterface` struct in `src/tui/chat.rs`:

```rust
/// Whether currently streaming a response
pub is_streaming: bool,
/// Accumulated streaming text
pub streaming_text: String,
```

Update `ChatInterface::new()` to initialize these:

```rust
is_streaming: false,
streaming_text: String::new(),
```

Update `ChatInterface` `Default` derive (already uses `#[derive(Debug, Default)]` but `is_streaming: false` and `streaming_text: String::new()` match defaults, so the derive still works).

**Step 2: Update render to show streaming text**

Modify `render_messages` in `src/tui/chat.rs` — after rendering all messages, add streaming text if active:

```rust
// After the message loop, before the empty check:
if self.is_streaming && !self.streaming_text.is_empty() {
    let style = Style::default().fg(Color::Green);
    let prefix = "Assistant: ";
    let max_width = area.width.saturating_sub(2) as usize;
    let wrapped = textwrap::wrap(&self.streaming_text, max_width);

    for (i, line) in wrapped.iter().enumerate() {
        if i == 0 {
            lines.push(Line::from(vec![
                Span::styled(prefix, style.add_modifier(Modifier::BOLD)),
                Span::styled(line.to_string(), style),
            ]));
        } else {
            lines.push(Line::from(Span::styled(line.to_string(), style)));
        }
    }
    // Blinking cursor indicator
    lines.push(Line::from(Span::styled(
        "▊",
        Style::default().fg(Color::Green).add_modifier(Modifier::SLOW_BLINK),
    )));
}
```

**Step 3: Update run_async to use streaming**

Replace the agent call block in `run_async` (`src/tui/mod.rs` lines 134-155) with streaming:

```rust
// Process pending agent message
if self.chat.send_to_agent {
    self.chat.send_to_agent = false;
    let user_input = self.chat.pending_input.take().unwrap_or_default();
    self.chat.add_message(ChatMessage::user(&user_input));

    if let Some(ref mut agent) = self.agent {
        // Use streaming
        self.chat.is_streaming = true;
        self.chat.streaming_text.clear();

        match agent.run_streaming(&user_input).await {
            Ok(stream) => {
                use futures::StreamExt;
                let mut stream = stream;
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            self.chat.streaming_text.push_str(&chunk);
                            // Re-render after each chunk
                            terminal
                                .draw(|f| self.chat.render(f))
                                .map_err(|e| ZcodeError::InternalError(format!("Failed to draw: {}", e)))?;
                        }
                        Err(e) => {
                            self.chat
                                .add_message(ChatMessage::system(format!("Stream error: {}", e)));
                            break;
                        }
                    }
                }
                // Finalize streaming
                let final_text = self.chat.streaming_text.clone();
                self.chat.is_streaming = false;
                self.chat.streaming_text.clear();
                self.chat.add_assistant_response(&final_text);
            }
            Err(e) => {
                self.chat.is_streaming = false;
                self.chat.streaming_text.clear();
                self.chat
                    .add_message(ChatMessage::system(format!("Error: {}", e)));
            }
        }
    } else {
        self.chat.add_assistant_response(
            "Agent not configured. Please set ANTHROPIC_API_KEY.",
        );
    }
}
```

**Step 4: Add futures::StreamExt import**

At the top of `src/tui/mod.rs`, add `use futures::StreamExt;` if not already imported.

**Step 5: Run tests and clippy**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: all pass

**Step 6: Commit**

```bash
git add src/tui/mod.rs src/tui/chat.rs
git commit -m "feat: wire streaming into TUI for token-by-token display"
```

---

## Feature 3: Better TUI

### Task 3.1: Scroll support

**Files:**
- Modify: `src/tui/chat.rs` (use `scroll` field in rendering, add scroll methods)
- Modify: `src/tui/mod.rs` (add PageUp/PageDown key bindings)

**Step 1: Add scroll methods to ChatInterface**

Add to `impl ChatInterface`:

```rust
/// Scroll up by N lines
pub fn scroll_up(&mut self, lines: u16) {
    self.scroll = self.scroll.saturating_sub(lines);
}

/// Scroll down by N lines
pub fn scroll_down(&mut self, lines: u16) {
    self.scroll = self.scroll.saturating_add(lines);
}

/// Auto-scroll to bottom (called after new messages)
pub fn scroll_to_bottom(&mut self) {
    // Will be set by render based on content height
    self.scroll = u16::MAX;
}
```

**Step 2: Update render_messages to use scroll**

Modify `render_messages` to accept scroll offset. The Paragraph widget in ratatui supports `.scroll((row, col))`:

```rust
fn render_messages(&self, area: Rect) -> Paragraph<'_> {
    // ... existing line-building code ...

    let total_lines = lines.len() as u16;
    let view_height = area.height.saturating_sub(2); // borders

    // Clamp scroll position
    let max_scroll = total_lines.saturating_sub(view_height);
    let scroll = if self.scroll == u16::MAX {
        max_scroll
    } else {
        self.scroll.min(max_scroll)
    };

    Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title("Chat"))
        .scroll((scroll, 0))
}
```

**Step 3: Auto-scroll on new messages**

Modify `add_message` and `add_assistant_response` to call `self.scroll_to_bottom()`:

```rust
pub fn add_message(&mut self, message: ChatMessage) {
    self.messages.push(message);
    self.scroll_to_bottom();
}

pub fn add_assistant_response(&mut self, content: &str) {
    self.messages.push(ChatMessage::assistant(content));
    self.scroll_to_bottom();
}
```

**Step 4: Add key bindings for scroll**

Modify `handle_event` in `src/tui/mod.rs` — add PageUp/PageDown:

```rust
(KeyModifiers::NONE, KeyCode::PageUp) => {
    self.chat.scroll_up(10);
}
(KeyModifiers::NONE, KeyCode::PageDown) => {
    self.chat.scroll_down(10);
}
(KeyModifiers::CONTROL, KeyCode::Char('u')) => {
    self.chat.scroll_up(5);
}
(KeyModifiers::CONTROL, KeyCode::Char('d')) => {
    self.chat.scroll_down(5);
}
```

**Step 5: Run tests**

Run: `cargo test`
Expected: all pass

**Step 6: Commit**

```bash
git add src/tui/chat.rs src/tui/mod.rs
git commit -m "feat: add scroll support with PageUp/Down and Ctrl+U/D"
```

---

### Task 3.2: Status bar

**Files:**
- Modify: `src/tui/chat.rs` (add status bar rendering)
- Modify: `src/tui/mod.rs` (update layout, set status)

**Step 1: Add status field to ChatInterface**

Add to `ChatInterface` struct:

```rust
/// Status bar text
pub status: String,
```

Initialize in `new()`:
```rust
status: "Ready".to_string(),
```

**Step 2: Add set_status method**

```rust
/// Set the status bar text
pub fn set_status(&mut self, status: impl Into<String>) {
    self.status = status.into();
}
```

**Step 3: Update layout to include status bar**

Modify `render` method — change constraints from `[Min(3), Length(3)]` to `[Min(3), Length(3), Length(1)]`:

```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Min(3),      // Messages
        Constraint::Length(3),   // Input
        Constraint::Length(1),   // Status bar
    ])
    .split(area);

// Render messages area
let messages_widget = self.render_messages(chunks[0]);
frame.render_widget(messages_widget, chunks[0]);

// Render input area
let input_widget = self.render_input(chunks[1]);
frame.render_widget(input_widget, chunks[1]);

// Render status bar
let status_widget = self.render_status(chunks[2]);
frame.render_widget(status_widget, chunks[2]);
```

**Step 4: Add render_status method**

```rust
/// Render the status bar
fn render_status(&self, area: Rect) -> Paragraph<'_> {
    let scroll_info = if self.scroll > 0 {
        format!(" | Scroll: {}", self.scroll)
    } else {
        String::new()
    };
    let msg_count = format!("Messages: {}", self.messages.len());
    let status_text = format!("{}{} | {}", self.status, scroll_info, msg_count);

    Paragraph::new(status_text).style(
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray),
    )
}
```

**Step 5: Update status during streaming and agent calls**

In `run_async`, set status messages:
- Before agent call: `self.chat.set_status("Thinking...")`
- During streaming: `self.chat.set_status("Streaming...")`
- After complete: `self.chat.set_status("Ready")`

**Step 6: Run tests**

Run: `cargo test`
Expected: all pass

**Step 7: Commit**

```bash
git add src/tui/chat.rs src/tui/mod.rs
git commit -m "feat: add status bar with message count and scroll indicator"
```

---

### Task 3.3: Multiline input

**Files:**
- Modify: `src/tui/chat.rs` (support multiline input)
- Modify: `src/tui/mod.rs` (Shift+Enter for newline)

**Step 1: Add multiline support to ChatInterface**

Add a `cursor_pos` field:

```rust
/// Cursor position in input (byte offset)
pub cursor_pos: usize,
```

Initialize to `0`.

**Step 2: Change Enter to send, Shift+Enter to newline**

Modify `handle_event` in `src/tui/mod.rs`:

```rust
(KeyModifiers::NONE, KeyCode::Enter) => {
    self.chat.send_current_input();
}
(KeyModifiers::SHIFT, KeyCode::Enter) => {
    self.chat.input_char('\n');
}
```

**Step 3: Update render_input for multiline display**

Change the input block height constraint from `Length(3)` to `Length(5)` to give more room:

```rust
Constraint::Length(5),   // Input (multiline)
```

**Step 4: Run tests**

Run: `cargo test`
Expected: all pass

**Step 5: Commit**

```bash
git add src/tui/chat.rs src/tui/mod.rs
git commit -m "feat: add multiline input with Shift+Enter for newlines"
```

---

## Feature 4: Native Tool Calling

### Task 4.1: Define ToolDefinition adapter

**Files:**
- Modify: `src/tools/mod.rs` (add to_tool_definition method to Tool trait)

**Step 1: Add rig-core tool definition support**

Add to `src/tools/mod.rs` — extend the `Tool` trait with a method to convert to rig-core's tool definition format. Also add a schema method:

```rust
/// Get the tool's JSON schema for input parameters
fn input_schema(&self) -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    })
}
```

Override in each tool implementation with proper schemas.

**Step 2: Add input_schema to FileReadTool**

```rust
fn input_schema(&self) -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Path to the file to read"}
        },
        "required": ["path"]
    })
}
```

Similarly for FileWriteTool, ShellExecTool, FileEditTool, SearchTool, GlobTool.

**Step 3: Add conversion helper**

Add to `src/tools/mod.rs`:

```rust
use rig::completion::ToolDefinition;

impl ToolRegistry {
    /// Get all tool definitions for rig-core
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|tool| {
            ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: tool.input_schema(),
            }
        }).collect()
    }
}
```

**Step 4: Run tests**

Run: `cargo test`
Expected: all pass

**Step 5: Commit**

```bash
git add src/tools/mod.rs src/tools/file_ops.rs src/tools/search.rs
git commit -m "feat: add tool definitions for native rig-core tool calling"
```

---

### Task 4.2: Update Agent to use native tool calling

**Files:**
- Modify: `src/agent.rs` (update run() to use native tool calling when available)

**Step 1: Add native tool calling path to Agent::run**

The key change: instead of parsing ` ```json ` blocks, use rig-core's agent with tools registered. rig-core handles the tool call/response cycle internally.

Update `src/agent.rs` — add a new method `run_native` that uses rig-core's tool system:

```rust
/// Process a user message using native rig-core tool calling
pub async fn run_native(&mut self, user_input: &str) -> Result<String> {
    self.conversation.push(Message::user(user_input));

    // Build messages
    let mut messages = vec![Message::system(&self.system_prompt)];
    messages.extend(self.conversation.iter().cloned());

    // Use LLM with native tool support
    let response = self.llm.chat(&messages).await?;
    let content = response.content.clone();

    self.conversation.push(Message::assistant(&content));
    Ok(content)
}
```

**Note:** Full native tool calling requires rig-core's `AgentBuilder::tool()` registration with typed tool structs. This is a bigger change. For now, keep the JSON text parsing approach but add the adapter layer from Task 4.1. The actual switch to native can happen once rig-core's tool registration API is fully wired.

**Step 2: Update parse_tool_call to be more robust**

Improve the existing parser to handle edge cases:

```rust
pub fn parse_tool_call(response: &str) -> Option<(String, Value)> {
    // Try ```json blocks first
    if let Some((tool, input)) = Self::parse_json_block(response) {
        return Some((tool, input));
    }

    // Try bare JSON objects on their own line
    Self::parse_bare_json(response)
}

/// Parse tool call from ```json fenced block
fn parse_json_block(response: &str) -> Option<(String, Value)> {
    let json_start = response.find("```json")?;
    let json_content_start = json_start + 7;
    let json_end = response[json_content_start..].find("```")?;
    let json_str = response[json_content_start..json_content_start + json_end].trim();

    let parsed: Value = serde_json::from_str(json_str).ok()?;

    let tool = parsed.get("tool")?.as_str()?.to_string();
    let input = parsed.get("input")?.clone();

    Some((tool, input))
}

/// Parse tool call from bare JSON (no fence)
fn parse_bare_json(response: &str) -> Option<(String, Value)> {
    let trimmed = response.trim();

    // Must start with { and end with }
    if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
        return None;
    }

    let parsed: Value = serde_json::from_str(trimmed).ok()?;

    let tool = parsed.get("tool")?.as_str()?.to_string();
    let input = parsed.get("input")?.clone();

    Some((tool, input))
}
```

**Step 3: Update tests for new parser**

Add tests:

```rust
#[test]
fn test_parse_tool_call_bare_json() {
    let response = r#"{"tool": "file_read", "input": {"path": "test.rs"}}"#;
    let (tool, input) = Agent::parse_tool_call(response).unwrap();
    assert_eq!(tool, "file_read");
    assert_eq!(input["path"], "test.rs");
}
```

**Step 4: Run tests**

Run: `cargo test`
Expected: all pass

**Step 5: Commit**

```bash
git add src/agent.rs
git commit -m "feat: improve tool call parsing with bare JSON fallback"
```

---

### Task 4.3: Add tool schema to system prompt

**Files:**
- Modify: `src/main.rs` (include tool schemas in system prompt)

**Step 1: Update system prompt generation**

In `src/main.rs`, update the system prompt to include JSON schemas for each tool:

```rust
let tools_desc = registry
    .list()
    .iter()
    .filter_map(|name| {
        registry.get(name).map(|tool| {
            format!(
                "- {}\n  Description: {}\n  Schema: {}",
                tool.name(),
                tool.description(),
                tool.input_schema()
            )
        })
    })
    .collect::<Vec<_>>()
    .join("\n\n");

let system_prompt = format!(
    r#"You are zcode, a programming assistant.

Available tools:
{}

When you need to use a tool, respond with EXACTLY this JSON format and nothing else in the code block:
{{"tool": "tool_name", "input": {{"arg": "value"}}}}

Otherwise, respond with helpful text."#,
    tools_desc
);
```

**Step 2: Run tests**

Run: `cargo test`
Expected: all pass

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: include tool schemas in system prompt for better tool calling"
```

---

## Final Verification

### Task F.1: Full test suite and clippy

**Step 1: Run full test suite**

Run: `cargo test`
Expected: all tests pass

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: clean

**Step 3: Run with API key (manual test)**

Run: `ANTHROPIC_API_KEY=sk-... cargo run`
- Type a message, verify streaming works
- Ask to read a file, verify tool calling works
- Test scroll with PageUp/PageDown
- Test multiline with Shift+Enter

**Step 4: Final commit (if any fixes needed)**

```bash
git add -A
git commit -m "fix: final clippy and test fixes for phase 2"
```
