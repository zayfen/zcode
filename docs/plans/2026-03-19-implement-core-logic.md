# Implement Core Logic

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire up zcode's main entry point to the TUI, convert Tool trait to async, implement 3 basic tools (file_read, file_write, shell_exec), integrate rig-core 0.33.0 for real LLM streaming, and build a basic agent orchestration loop.

**Architecture:** Bottom-up — fix the foundation (async Tool trait, dependencies), then build tools on top, then wire LLM, then connect everything through the agent loop. Each layer depends only on the layer below.

**Tech Stack:** Rust 2021 (rustc 1.91.1), tokio async runtime, rig-core 0.33.0, ratatui 0.27 + crossterm 0.27, serde_json, futures 0.3

---

### Task 1: Enable rig-core dependency

**Files:**
- Modify: `Cargo.toml:44`

**Step 1: Update Cargo.toml**

Replace the commented-out rig-core line with the latest version:

```toml
# LLM integration
rig-core = "0.33"
```

**Step 2: Verify dependency resolution**

Run: `cargo check`
Expected: Compiles successfully (may take a while to download rig-core and its deps)

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: enable rig-core 0.33 dependency"
```

---

### Task 2: Make Tool trait async

**Files:**
- Modify: `src/tools/mod.rs:14-23,50-56,70-116`
- Modify: `tests/registry_test.rs:16-28,30-63`

**Step 1: Write failing test for async tool execution**

Add an async test to `src/tools/mod.rs` tests module:

```rust
#[tokio::test]
async fn test_registry_execute_async() {
    let mut registry = ToolRegistry::new();
    registry.register(TestTool);

    let result = registry.execute_async("test", Value::Null).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Value::String("test result".to_string()));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_registry_execute_async`
Expected: FAIL — `execute_async` method not found

**Step 3: Make Tool trait async and update ToolRegistry**

In `src/tools/mod.rs`, change the Tool trait to use `std::future::Future` and add async execute to ToolRegistry:

```rust
use std::future::Future;
use std::pin::Pin;

/// Result type for tool execution
pub type ToolResult<T> = Result<T>;

/// Trait for implementing tools (async version)
pub trait Tool: Send + Sync {
    /// Get the tool name
    fn name(&self) -> &str;

    /// Get the tool description
    fn description(&self) -> &str;

    /// Execute the tool with the given input (async)
    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = ToolResult<Value>> + Send + '_>>;
}

/// Registry for managing and executing tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool in the registry
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        let name = tool.name().to_string();
        self.tools.insert(name, Arc::new(tool));
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Execute a tool by name (async)
    pub async fn execute(&self, name: &str, input: Value) -> ToolResult<Value> {
        let tool = self.tools.get(name).ok_or_else(|| ZcodeError::ToolNotFound {
            name: name.to_string(),
        })?;

        tool.execute(input).await
    }

    /// List all registered tools
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
}
```

Update the `TestTool` in tests to return a pinned boxed future:

```rust
struct TestTool;

impl Tool for TestTool {
    fn name(&self) -> &str { "test" }
    fn description(&self) -> &str { "A test tool" }
    fn execute(&self, _input: Value) -> Pin<Box<dyn Future<Output = ToolResult<Value>> + Send + '_>> {
        Box::pin(async { Ok(Value::String("test result".to_string())) })
    }
}
```

Update all existing tests to use `.await`:

```rust
#[tokio::test]
async fn test_registry_register() { /* unchanged - sync */ }

#[tokio::test]
async fn test_registry_execute() {
    let mut registry = ToolRegistry::new();
    registry.register(TestTool);

    let result = registry.execute("test", Value::Null).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Value::String("test result".to_string()));
}

#[tokio::test]
async fn test_registry_unknown_tool() {
    let registry = ToolRegistry::new();
    let result = registry.execute("unknown", Value::Null).await;
    assert!(result.is_err());
}
```

**Step 4: Update integration test**

In `tests/registry_test.rs`, update MockTool and tests:

```rust
use std::future::Future;
use std::pin::Pin;
use zcode::tools::{Tool, ToolRegistry, ToolResult};

struct MockTool { name: String }

impl MockTool {
    fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}

impl Tool for MockTool {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str { "A mock tool for testing" }
    fn execute(&self, _input: serde_json::Value) -> Pin<Box<dyn Future<Output = ToolResult<serde_json::Value>> + Send + '_>> {
        Box::pin(async { Ok(serde_json::json!({ "result": "mock executed" })) })
    }
}

#[tokio::test]
async fn test_registry_registers_tool() {
    let mut registry = ToolRegistry::new();
    let tool = MockTool::new("test_tool");
    registry.register(tool);
    assert!(registry.get("test_tool").is_some());
}

#[tokio::test]
async fn test_registry_executes_tool() {
    let mut registry = ToolRegistry::new();
    let tool = MockTool::new("execute_tool");
    registry.register(tool);

    let input = serde_json::json!({ "param": "value" });
    let result = registry.execute("execute_tool", input).await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["result"], "mock executed");
}

#[tokio::test]
async fn test_registry_unknown_tool() {
    let registry = ToolRegistry::new();
    let result = registry.execute("unknown_tool", serde_json::json!({})).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("unknown_tool"));
}
```

**Step 5: Run all tests to verify they pass**

Run: `cargo test`
Expected: All tests PASS

**Step 6: Commit**

```bash
git add src/tools/mod.rs tests/registry_test.rs
git commit -m "feat: make Tool trait async with Pin<Box<dyn Future>> return"
```

---

### Task 3: Implement file_read tool

**Files:**
- Create: `src/tools/file_ops.rs`
- Modify: `src/tools/mod.rs` (add `pub mod file_ops;`)

**Step 1: Write failing tests**

Create `src/tools/file_ops.rs`:

```rust
//! File system tools for zcode

use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use crate::error::{ZcodeError, Result};
use super::{Tool, ToolResult};

/// Tool for reading file contents
pub struct FileReadTool;

impl Tool for FileReadTool {
    fn name(&self) -> &str { "file_read" }
    fn description(&self) -> &str { "Read the contents of a file. Input: {\"path\": \"<file_path>\"}" }

    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = ToolResult<Value>> + Send + '_>> {
        Box::pin(async move {
            let path = input["path"].as_str()
                .ok_or_else(|| ZcodeError::InvalidToolInput("Missing 'path' field".to_string()))?;

            let content = tokio::fs::read_to_string(path).await
                .map_err(|e| ZcodeError::ToolExecutionFailed {
                    name: "file_read".to_string(),
                    message: format!("Failed to read '{}': {}", path, e),
                })?;

            Ok(serde_json::json!({ "content": content }))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_file_read_success() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "Hello, world!").unwrap();

        let tool = FileReadTool;
        let input = serde_json::json!({ "path": tmp.path().to_str().unwrap() });
        let result = tool.execute(input).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["content"].as_str().unwrap().contains("Hello, world!"));
    }

    #[tokio::test]
    async fn test_file_read_missing_path() {
        let tool = FileReadTool;
        let input = serde_json::json!({});
        let result = tool.execute(input).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'path'"));
    }

    #[tokio::test]
    async fn test_file_read_not_found() {
        let tool = FileReadTool;
        let input = serde_json::json!({ "path": "/nonexistent/file.txt" });
        let result = tool.execute(input).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("file_read"));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test file_ops`
Expected: FAIL — module not found (file_ops not declared in mod.rs)

**Step 3: Add module declaration**

In `src/tools/mod.rs`, add after the existing imports:

```rust
pub mod file_ops;

pub use file_ops::FileReadTool;
```

**Step 4: Run tests to verify they pass**

Run: `cargo test file_ops`
Expected: All 3 tests PASS

**Step 5: Commit**

```bash
git add src/tools/file_ops.rs src/tools/mod.rs
git commit -m "feat: implement file_read tool"
```

---

### Task 4: Implement file_write tool

**Files:**
- Modify: `src/tools/file_ops.rs`

**Step 1: Write failing tests**

Add to `src/tools/file_ops.rs`:

```rust
/// Tool for writing file contents
pub struct FileWriteTool;

impl Tool for FileWriteTool {
    fn name(&self) -> &str { "file_write" }
    fn description(&self) -> &str { "Write content to a file. Input: {\"path\": \"<file_path>\", \"content\": \"<text>\"}" }

    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = ToolResult<Value>> + Send + '_>> {
        Box::pin(async move {
            let path = input["path"].as_str()
                .ok_or_else(|| ZcodeError::InvalidToolInput("Missing 'path' field".to_string()))?;

            let content = input["content"].as_str()
                .ok_or_else(|| ZcodeError::InvalidToolInput("Missing 'content' field".to_string()))?;

            // Create parent directories if they don't exist
            if let Some(parent) = std::path::Path::new(path).parent() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| ZcodeError::ToolExecutionFailed {
                        name: "file_write".to_string(),
                        message: format!("Failed to create directories: {}", e),
                    })?;
            }

            tokio::fs::write(path, content).await
                .map_err(|e| ZcodeError::ToolExecutionFailed {
                    name: "file_write".to_string(),
                    message: format!("Failed to write '{}': {}", path, e),
                })?;

            Ok(serde_json::json!({ "success": true, "path": path }))
        })
    }
}
```

Add tests:

```rust
#[tokio::test]
async fn test_file_write_success() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("output.txt");

    let tool = FileWriteTool;
    let input = serde_json::json!({
        "path": path.to_str().unwrap(),
        "content": "Written by zcode!"
    });
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test file_write`
Expected: FAIL — `FileWriteTool` not found

**Step 3: Run tests to verify they pass**

(Implementation is in Step 1 already, so tests should pass immediately after adding the code.)

Run: `cargo test file_write`
Expected: All tests PASS

**Step 4: Update module exports**

In `src/tools/mod.rs`, update the re-export:

```rust
pub use file_ops::{FileReadTool, FileWriteTool};
```

**Step 5: Commit**

```bash
git add src/tools/file_ops.rs src/tools/mod.rs
git commit -m "feat: implement file_write tool"
```

---

### Task 5: Implement shell_exec tool

**Files:**
- Modify: `src/tools/file_ops.rs` (or create `src/tools/shell.rs`)

**Step 1: Write failing tests**

Add to `src/tools/file_ops.rs`:

```rust
/// Tool for executing shell commands
pub struct ShellExecTool;

impl Tool for ShellExecTool {
    fn name(&self) -> &str { "shell_exec" }
    fn description(&self) -> &str { "Execute a shell command. Input: {\"command\": \"<cmd>\", \"cwd\": \"<dir>\" (optional)}" }

    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = ToolResult<Value>> + Send + '_>> {
        Box::pin(async move {
            let command = input["command"].as_str()
                .ok_or_else(|| ZcodeError::InvalidToolInput("Missing 'command' field".to_string()))?;

            let cwd = input["cwd"].as_str();

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

            let output = cmd.output().await
                .map_err(|e| ZcodeError::ToolExecutionFailed {
                    name: "shell_exec".to_string(),
                    message: format!("Failed to execute command: {}", e),
                })?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);

            Ok(serde_json::json!({
                "stdout": stdout,
                "stderr": stderr,
                "exit_code": exit_code,
                "success": output.status.success()
            }))
        })
    }
}
```

Add tests:

```rust
#[tokio::test]
async fn test_shell_exec_echo() {
    let tool = ShellExecTool;
    let input = serde_json::json!({ "command": "echo hello" });
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
    let input = serde_json::json!({ "command": "exit 1" });
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
    let input = serde_json::json!({
        "command": "pwd",
        "cwd": tmp.path().to_str().unwrap()
    });
    let result = tool.execute(input).await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output["stdout"].as_str().unwrap().contains(tmp.path().to_str().unwrap()));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test shell_exec`
Expected: FAIL — `ShellExecTool` not found

**Step 3: Run tests to verify they pass**

Run: `cargo test shell_exec`
Expected: All tests PASS

**Step 4: Update module exports**

In `src/tools/mod.rs`, update:

```rust
pub use file_ops::{FileReadTool, FileWriteTool, ShellExecTool};
```

**Step 5: Commit**

```bash
git add src/tools/file_ops.rs src/tools/mod.rs
git commit -m "feat: implement shell_exec tool"
```

---

### Task 6: Implement real LLM integration with rig-core

**Files:**
- Modify: `Cargo.toml` (already done in Task 1)
- Modify: `src/llm/mod.rs`
- Modify: `src/llm/provider.rs`

**Step 1: Update LlmProvider trait to be async**

In `src/llm/provider.rs`:

```rust
use std::future::Future;
use std::pin::Pin;
use crate::error::{Result, ZcodeError};
use crate::llm::{LlmConfig, LlmResponse, Message};

/// Trait for LLM providers (async)
pub trait LlmProvider: Send + Sync {
    /// Generate a completion from a prompt
    fn complete(&self, prompt: &str) -> Pin<Box<dyn Future<Output = Result<String>> + Send + '_>>;

    /// Generate a completion from a conversation
    fn chat(&self, messages: &[Message]) -> Pin<Box<dyn Future<Output = Result<LlmResponse>> + Send + '_>>;

    /// Stream a completion (returns a stream of text chunks)
    fn stream_chat(&self, messages: &[Message]) -> Pin<Box<dyn Future<Output = Result<StreamingResponse>> + Send + '_>>;
}
```

**Step 2: Update MockLlmProvider for async trait**

```rust
pub struct MockLlmProvider {
    response: String,
}

impl MockLlmProvider {
    pub fn new(response: impl Into<String>) -> Self {
        Self { response: response.into() }
    }
}

impl LlmProvider for MockLlmProvider {
    fn complete(&self, _prompt: &str) -> Pin<Box<dyn Future<Output = Result<String>> + Send + '_>> {
        let resp = self.response.clone();
        Box::pin(async move { Ok(resp) })
    }

    fn chat(&self, _messages: &[Message]) -> Pin<Box<dyn Future<Output = Result<LlmResponse>> + Send + '_>> {
        let resp = self.response.clone();
        Box::pin(async move {
            Ok(LlmResponse {
                content: resp,
                model: "mock-model".to_string(),
                usage: Some(crate::llm::UsageStats {
                    input_tokens: 10,
                    output_tokens: 5,
                }),
            })
        })
    }

    fn stream_chat(&self, _messages: &[Message]) -> Pin<Box<dyn Future<Output = Result<StreamingResponse>> + Send + '_>> {
        let resp = self.response.clone();
        Box::pin(async move {
            let chunks = vec![Ok(resp)];
            Ok(Box::pin(futures::stream::iter(chunks)) as StreamingResponse)
        })
    }
}
```

**Step 3: Implement RigProvider with real API calls**

```rust
use rig::client::CompletionClient;
use rig::providers::anthropic;

pub struct RigProvider {
    config: LlmConfig,
}

impl RigProvider {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    fn get_api_key(&self) -> Result<String> {
        if let Some(ref key) = self.config.api_key {
            return Ok(key.clone());
        }
        let env_var = match self.config.provider.as_str() {
            "anthropic" => "ANTHROPIC_API_KEY",
            "openai" => "OPENAI_API_KEY",
            _ => "API_KEY",
        };
        std::env::var(env_var).map_err(|_| ZcodeError::MissingApiKey(self.config.provider.clone()))
    }
}

impl LlmProvider for RigProvider {
    fn complete(&self, prompt: &str) -> Pin<Box<dyn Future<Output = Result<String>> + Send + '_>> {
        let config = self.config.clone();
        let prompt = prompt.to_string();
        Box::pin(async move {
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .map_err(|_| ZcodeError::MissingApiKey("anthropic".to_string()))?;

            let client = anthropic::ClientBuilder::new(&api_key)
                .build()
                .map_err(|e| ZcodeError::LlmApiError(format!("Failed to create client: {}", e)))?;

            let agent = client
                .agent(&config.model)
                .preamble("You are a helpful programming assistant.")
                .build();

            let response = agent
                .prompt(&prompt)
                .await
                .map_err(|e| ZcodeError::LlmApiError(format!("Completion failed: {}", e)))?;

            Ok(response)
        })
    }

    fn chat(&self, messages: &[Message]) -> Pin<Box<dyn Future<Output = Result<LlmResponse>> + Send + '_>> {
        let config = self.config.clone();
        let messages = messages.to_vec();
        Box::pin(async move {
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .map_err(|_| ZcodeError::MissingApiKey("anthropic".to_string()))?;

            let client = anthropic::ClientBuilder::new(&api_key)
                .build()
                .map_err(|e| ZcodeError::LlmApiError(format!("Failed to create client: {}", e)))?;

            let mut builder = client.agent(&config.model);

            // Add system message if present
            for msg in &messages {
                if msg.role == crate::llm::MessageRole::System {
                    builder = builder.preamble(&msg.content);
                    break;
                }
            }

            let agent = builder.build();

            // Build conversation prompt from non-system messages
            let prompt = messages
                .iter()
                .filter(|m| m.role != crate::llm::MessageRole::System)
                .map(|m| match m.role {
                    crate::llm::MessageRole::User => format!("User: {}", m.content),
                    crate::llm::MessageRole::Assistant => format!("Assistant: {}", m.content),
                    _ => m.content.clone(),
                })
                .collect::<Vec<_>>()
                .join("\n");

            let response = agent
                .prompt(&prompt)
                .await
                .map_err(|e| ZcodeError::LlmApiError(format!("Chat failed: {}", e)))?;

            Ok(LlmResponse {
                content: response,
                model: config.model.clone(),
                usage: None,
            })
        })
    }

    fn stream_chat(&self, messages: &[Message]) -> Pin<Box<dyn Future<Output = Result<StreamingResponse>> + Send + '_>> {
        // For now, use non-streaming and wrap in a stream
        // Full streaming support can be added later
        let config = self.config.clone();
        let messages = messages.to_vec();
        Box::pin(async move {
            let response = RigProvider { config }.chat(&messages).await?;
            let content = response.content;
            let chunks = vec![Ok(content)];
            Ok(Box::pin(futures::stream::iter(chunks)) as StreamingResponse)
        })
    }
}
```

**Step 4: Update tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider() {
        let provider = MockLlmProvider::new("Hello, world!");
        let result = provider.complete("test").await.unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[tokio::test]
    async fn test_mock_provider_chat() {
        let provider = MockLlmProvider::new("Response");
        let messages = vec![Message::user("Hello")];
        let response = provider.chat(&messages).await.unwrap();
        assert_eq!(response.content, "Response");
    }

    #[test]
    fn test_rig_provider_creation() {
        let config = LlmConfig::default();
        let provider = RigProvider::new(config);
        assert_eq!(provider.config().provider, "anthropic");
    }
}
```

**Step 5: Run tests**

Run: `cargo test`
Expected: All tests PASS (rig API tests only run with API key)

**Step 6: Commit**

```bash
git add src/llm/provider.rs src/llm/mod.rs Cargo.toml
git commit -m "feat: integrate rig-core for real LLM API calls"
```

---

### Task 7: Connect main.rs to TUI

**Files:**
- Modify: `src/main.rs`
- Modify: `src/tools/mod.rs` (add register_built_in_tools)

**Step 1: Add register_built_in_tools helper**

In `src/tools/mod.rs`:

```rust
impl ToolRegistry {
    /// Register all built-in tools
    pub fn register_built_in_tools(&mut self) {
        self.register(file_ops::FileReadTool);
        self.register(file_ops::FileWriteTool);
        self.register(file_ops::ShellExecTool);
    }
}
```

**Step 2: Wire up main.rs to start TUI**

In `src/main.rs`:

```rust
use clap::Parser;
use tracing_subscriber::EnvFilter;

/// Zcode programming agent
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the project directory
    #[arg(short, long, default_value = ".")]
    path: String,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let filter = if args.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    tracing::info!("Starting zcode in directory: {}", args.path);

    // Load settings
    let settings = zcode::Settings::load().unwrap_or_default();
    tracing::debug!("Loaded settings: {:?}", settings);

    // Initialize tool registry with built-in tools
    let mut registry = zcode::ToolRegistry::new();
    registry.register_built_in_tools();
    tracing::debug!("Initialized tool registry with {} tools", registry.list().len());

    // Initialize LLM provider
    let llm_config = zcode::LlmConfig {
        provider: settings.llm.provider.clone(),
        model: settings.llm.model.clone(),
        api_key: settings.llm.api_key.clone(),
        temperature: settings.llm.temperature,
        max_tokens: settings.llm.max_tokens,
    };
    tracing::debug!("LLM config: provider={}, model={}", llm_config.provider, llm_config.model);

    // Initialize and run TUI
    let mut terminal = zcode::tui::init_terminal()?;
    let mut app = zcode::TuiApp::new();

    // Add welcome message
    app.chat.add_message(zcode::tui::chat::ChatMessage::system(
        "Welcome to zcode! Type a message and press Enter to chat. Esc to quit."
    ));

    let result = app.run(&mut terminal);

    // Always restore terminal
    zcode::tui::restore_terminal(&mut terminal)?;

    result?;
    Ok(())
}
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src/main.rs src/tools/mod.rs
git commit -m "feat: connect main to TUI with built-in tools"
```

---

### Task 8: Implement Agent orchestration loop

**Files:**
- Modify: `src/agent.rs`
- Modify: `src/tui/chat.rs` (integrate agent with chat)
- Modify: `src/tui/mod.rs` (add agent to TuiApp)

**Step 1: Define Agent with LLM and tool registry**

In `src/agent.rs`:

```rust
//! Agent module for zcode
//!
//! This module implements the main agent loop and orchestration.

use crate::error::Result;
use crate::llm::{LlmProvider, Message, LlmResponse};
use crate::tools::ToolRegistry;
use std::sync::Arc;

/// System prompt for the agent
const SYSTEM_PROMPT: &str = r#"You are zcode, a programming assistant. You can help with:
- Reading and writing files
- Executing shell commands
- Answering programming questions

When you need to use a tool, respond with a JSON block like:
```json
{"tool": "file_read", "input": {"path": "src/main.rs"}}
```

Otherwise, respond with helpful text."#;

/// Agent state
pub struct Agent {
    /// Agent name
    name: String,
    /// LLM provider
    llm: Arc<dyn LlmProvider>,
    /// Tool registry
    tools: Arc<ToolRegistry>,
    /// Conversation history
    history: Vec<Message>,
}

impl Agent {
    /// Create a new agent
    pub fn new(
        name: impl Into<String>,
        llm: Arc<dyn LlmProvider>,
        tools: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            name: name.into(),
            llm,
            tools,
            history: vec![Message::system(SYSTEM_PROMPT)],
        }
    }

    /// Get the agent name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Process a user message and return a response
    pub async fn process_message(&mut self, user_input: &str) -> Result<String> {
        // Add user message to history
        self.history.push(Message::user(user_input));

        // Get LLM response
        let response = self.llm.chat(&self.history).await?;
        let mut content = response.content.clone();

        // Check if response contains a tool call
        if let Some(tool_call) = self.parse_tool_call(&content) {
            // Execute the tool
            match self.tools.execute(&tool_call.tool, tool_call.input).await {
                Ok(output) => {
                    // Add tool result to history and get follow-up
                    self.history.push(Message::assistant(&content));
                    self.history.push(Message::user(format!(
                        "Tool result:\n```json\n{}\n```",
                        serde_json::to_string_pretty(&output).unwrap_or_default()
                    )));

                    let follow_up = self.llm.chat(&self.history).await?;
                    content = follow_up.content.clone();
                }
                Err(e) => {
                    content = format!("Tool execution failed: {}", e);
                }
            }
        }

        // Add assistant response to history
        self.history.push(Message::assistant(&content));
        Ok(content)
    }

    /// Parse a tool call from LLM response
    fn parse_tool_call(&self, response: &str) -> Option<ToolCall> {
        // Look for JSON blocks in the response
        let json_start = response.find("```json")?;
        let json_content_start = json_start + 7; // len("```json")
        let json_end = response[json_content_start..].find("```")?;
        let json_str = &response[json_content_start..json_content_start + json_end];

        let parsed: serde_json::Value = serde_json::from_str(json_str.trim()).ok()?;

        let tool = parsed["tool"].as_str()?.to_string();
        let input = parsed["input"].clone();

        Some(ToolCall { tool, input })
    }
}

/// A parsed tool call from LLM response
struct ToolCall {
    tool: String,
    input: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::MockLlmProvider;

    #[tokio::test]
    async fn test_agent_creation() {
        let llm = Arc::new(MockLlmProvider::new("Hello!"));
        let tools = Arc::new(ToolRegistry::new());
        let agent = Agent::new("zcode", llm, tools);

        assert_eq!(agent.name(), "zcode");
        assert_eq!(agent.history.len(), 1); // system prompt
    }

    #[tokio::test]
    async fn test_agent_process_message() {
        let llm = Arc::new(MockLlmProvider::new("I can help with that!"));
        let tools = Arc::new(ToolRegistry::new());
        let mut agent = Agent::new("zcode", llm, tools);

        let response = agent.process_message("Hello").await.unwrap();
        assert_eq!(response, "I can help with that!");
        assert_eq!(agent.history.len(), 3); // system + user + assistant
    }

    #[test]
    fn test_parse_tool_call() {
        let llm = Arc::new(MockLlmProvider::new(""));
        let tools = Arc::new(ToolRegistry::new());
        let agent = Agent::new("zcode", llm, tools);

        let response = r#"Here is the file:
```json
{"tool": "file_read", "input": {"path": "test.rs"}}
```
"#;
        let tool_call = agent.parse_tool_call(response).unwrap();
        assert_eq!(tool_call.tool, "file_read");
        assert_eq!(tool_call.input["path"], "test.rs");
    }

    #[test]
    fn test_parse_tool_call_no_json() {
        let llm = Arc::new(MockLlmProvider::new(""));
        let tools = Arc::new(ToolRegistry::new());
        let agent = Agent::new("zcode", llm, tools);

        let response = "Just a regular response with no tool call.";
        assert!(agent.parse_tool_call(response).is_none());
    }
}
```

**Step 2: Integrate agent with TUI**

In `src/tui/mod.rs`, update TuiApp to include the agent and make `run` async:

```rust
use crate::agent::Agent;
use crate::llm::LlmProvider;
use crate::tools::ToolRegistry;
use std::sync::Arc;

pub struct TuiApp {
    pub should_quit: bool,
    pub chat: ChatInterface,
    pub agent: Option<Agent>,
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            chat: ChatInterface::new(),
            agent: None,
        }
    }

    /// Set the agent for the TUI
    pub fn set_agent(&mut self, agent: Agent) {
        self.agent = Some(agent);
    }

    pub fn handle_event(&mut self, event: Event) -> crate::error::Result<()> {
        if let Event::Key(key) = event {
            match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                    self.should_quit = true;
                }
                (KeyModifiers::NONE, KeyCode::Esc) => {
                    self.should_quit = true;
                }
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    if !self.chat.input.is_empty() {
                        // Mark for async processing
                        self.chat.send_to_agent = true;
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char(c)) => {
                    self.chat.input_char(c);
                }
                (KeyModifiers::NONE, KeyCode::Backspace) => {
                    self.chat.backspace();
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Run the main event loop (async version)
    pub async fn run_async(&mut self, terminal: &mut TuiTerminal) -> crate::error::Result<()> {
        while !self.should_quit {
            terminal
                .draw(|f| self.chat.render(f))
                .map_err(|e| ZcodeError::InternalError(format!("Failed to draw: {}", e)))?;

            // Process pending agent message
            if self.chat.send_to_agent {
                self.chat.send_to_agent = false;
                let user_input = self.chat.pending_input.take().unwrap_or_default();
                self.chat.add_message(ChatMessage::user(&user_input));

                if let Some(ref mut agent) = self.agent {
                    match agent.process_message(&user_input).await {
                        Ok(response) => {
                            self.chat.add_message(ChatMessage::assistant(response));
                        }
                        Err(e) => {
                            self.chat.add_message(ChatMessage::system(format!("Error: {}", e)));
                        }
                    }
                } else {
                    self.chat.add_message(ChatMessage::assistant(
                        "Agent not configured. Response will come in Task 8."
                    ));
                }
            }

            if event::poll(std::time::Duration::from_millis(100))
                .map_err(|e| ZcodeError::InternalError(format!("Poll error: {}", e)))?
            {
                let event = event::read()
                    .map_err(|e| ZcodeError::InternalError(format!("Read error: {}", e)))?;
                self.handle_event(event)?;
            }
        }
        Ok(())
    }
}
```

Update `src/tui/chat.rs` ChatInterface to support async agent:

```rust
pub struct ChatInterface {
    pub input: String,
    pub messages: Vec<ChatMessage>,
    pub scroll: u16,
    pub send_to_agent: bool,
    pub pending_input: Option<String>,
}

impl ChatInterface {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
            scroll: 0,
            send_to_agent: false,
            pending_input: None,
        }
    }

    pub fn send_current_input(&mut self) {
        if !self.input.is_empty() {
            self.pending_input = Some(self.input.clone());
            self.send_to_agent = true;
            self.input.clear();
        }
    }
}
```

**Step 3: Update main.rs to create agent**

```rust
// After initializing registry and llm_config:

// Initialize LLM provider
let llm: Arc<dyn zcode::llm::LlmProvider> = if std::env::var("ANTHROPIC_API_KEY").is_ok() {
    Arc::new(zcode::llm::RigProvider::new(llm_config))
} else {
    tracing::warn!("No ANTHROPIC_API_KEY found, using mock LLM provider");
    Arc::new(zcode::llm::provider::MockLlmProvider::new(
        "I'm a mock response. Set ANTHROPIC_API_KEY for real responses."
    ))
};

let registry = Arc::new(registry);
let agent = zcode::agent::Agent::new("zcode", llm, registry);

let mut app = zcode::TuiApp::new();
app.set_agent(agent);
```

**Step 4: Run all tests**

Run: `cargo test`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/agent.rs src/tui/mod.rs src/tui/chat.rs src/main.rs
git commit -m "feat: implement agent orchestration loop with tool execution"
```

---

### Task 9: Final verification and cleanup

**Files:**
- Modify: `src/lib.rs` (add re-exports for new types)

**Step 1: Update lib.rs re-exports**

```rust
pub use tools::{ToolRegistry, Tool, ToolResult, FileReadTool, FileWriteTool, ShellExecTool};
pub use llm::{LlmProvider, LlmConfig, Message, LlmResponse, RigProvider};
pub use agent::Agent;
```

**Step 2: Run full test suite**

Run: `cargo test`
Expected: All tests PASS

**Step 3: Run clippy**

Run: `cargo clippy`
Expected: No warnings (fix any that appear)

**Step 4: Final commit**

```bash
git add src/lib.rs
git commit -m "chore: update re-exports for new types"
```
