# zcode Architecture

> This document describes the internal design of `zcode`, a modular AI coding agent.

---

## Overview

`zcode` follows a **modular monolith** pattern — all components live in one crate but communicate through well-defined trait boundaries. This makes it easy to swap components (e.g., replace the LLM provider or add a new scripting engine) without restructuring the whole system.

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI / TUI Layer                          │
│              (clap commands  ·  ratatui chat interface)          │
└───────────────────────────────┬─────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────┐
│                        Workspace Facade                          │
│   Workspace::open/init · build_diff_context · snapshot_save     │
└──────┬──────────────┬────────────────────┬───────────────────────┘
       │              │                    │
┌──────▼──────┐ ┌─────▼──────┐  ┌─────────▼───────────────────────┐
│  Agent Bus  │ │  Git Diff  │  │         Session Snapshots        │
│  (tokio     │ │  Context   │  │  (SQLite · save/restore/diff)    │
│  mpsc)      │ │  Builder   │  └─────────────────────────────────-┘
└──────┬──────┘ └────────────┘
       │
┌──────▼───────────────────────────────────────────────────────────┐
│                        Agent System                               │
│                                                                   │
│  ┌─────────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │  Orchestrator   │  │   Planner    │  │       Coder          │ │
│  │  (task routing) │  │  (task split)│  │  (code generation)   │ │
│  └────────┬────────┘  └──────────────┘  └──────────────────────┘ │
│           │                                                        │
│  ┌────────▼────────┐  ┌──────────────────────────────────────────┐│
│  │  ReviewerAgent  │  │            AgentLoop                     ││
│  │  (code review)  │  │  (conversation · tool calls · history)   ││
│  └─────────────────┘  └──────────────────────────────────────────┘│
└─────────────────────────┬────────────────────────────────────────-┘
                          │
       ┌──────────────────┼─────────────────────┐
       │                  │                     │
┌──────▼───────┐  ┌───────▼──────┐  ┌──────────▼──────────────────┐
│ LLM Provider │  │ Tool Registry│  │      Memory System           │
│              │  │              │  │                               │
│ • Anthropic  │  │ • file tools │  │ WorkingMemory (in-proc)       │
│ • OpenAI     │  │ • shell      │  │ ProjectMemory (disk, git)     │
│ • Ollama     │  │ • search     │  │ SemanticIndex (vec embed)     │
│              │  │ • AST tools  │  │ ContextAssembler (budget)     │
└──────────────┘  │ • MCP tools  │  └───────────────────────────────┘
                  │ • Script     │
                  │   tools ─────┼──────────────────────────────────┐
                  └──────────────┘                                  │
                                                         ┌──────────▼──────────┐
                                                         │   Script Engines     │
                                                         │                      │
                                                         │  LuaEngine (mlua)    │
                                                         │  PythonEngine (pyo3) │
                                                         │  JsEngine (quickjs)  │
                                                         │  ShellEngine (sh)    │
                                                         └──────────────────────┘
```

---

## Module Breakdown

### `error` — Unified Error Type
`ZcodeError` covers all failure modes: IO, JSON, config parse, tool not found, LLM errors, script errors. Implements `From<io::Error>` and `From<serde_json::Error>` for ergonomic use of `?`.

---

### `config` — Configuration

| Struct | Purpose |
|---|---|
| `Settings` | Global user settings (~/.config/zcode/settings.toml) |
| `ProjectConfig` | Per-project config (.zcode/config.toml) |
| `McpServerConfig` | MCP server definition |
| `LspServerConfig` | LSP server definition |
| `ScriptConfig` + `HookConfig` | Script directories + lifecycle hooks |
| `SnapshotConfig` | SQLite snapshot parameters |
| `GrammarConfig` | Custom Tree-sitter shared library |

---

### `tools` — Tool System

```
Tool (trait)
  ├── FileTool (read/write/list/delete)
  ├── ShellTool (subprocess execution)
  ├── SearchTool (ripgrep-style search)
  ├── AstTool (Tree-sitter parse queries)
  ├── McpToolAdapter (wraps remote MCP tools)
  └── ScriptTool (wraps a script file as a tool)

ToolRegistry
  └── HashMap<name, Box<dyn Tool>>
```

All tools implement `fn execute(&self, input: Value) -> ToolResult<Value>`.

---

### `llm` — LLM Integration

```
LlmProvider (trait)
  ├── AnthropicProvider (claude-3-5-sonnet, streaming SSE)
  ├── OpenAiProvider (gpt-4o, function calling)
  └── OllamaProvider (local models)

LlmConfig    — model, temperature, max_tokens, system_prompt
Message      — role (system/user/assistant/tool), content
ToolCallSpec — JSON schema definition of a tool for function calling
```

---

### `agent` — Multi-Agent System

```
AgentTrait
  ├── OrchestratorAgent — receives user requests, routes to specialists
  ├── PlannerAgent      — breaks complex tasks into ordered subtasks
  ├── CoderAgent        — writes/edits code via LLM + tools
  └── ReviewerAgent     — static analysis of code diffs (5 categories)

MessageBus (tokio mpsc)
  └── BusHandle — per-agent sender/receiver

AgentLoop — conversation state, tool call dispatch, token counting
```

**ReviewerAgent categories:**

| Category | What's checked |
|---|---|
| `Logic` | `.unwrap()`, `panic!()` |
| `Security` | Hardcoded credentials, SQL injection risks |
| `Performance` | Unnecessary `.clone()` on collections |
| `Style` | Lines > 120 chars |
| `Testing` | New functions without corresponding `#[test]` |

---

### `memory` — Context Management

```
WorkingMemory   — in-process ephemeral state (key/value + conversation)
ProjectMemory   — persisted to .zcode/ directory (markdown files)
SemanticIndex   — vector embedding for semantic search
ContextAssembler — TokenBudget-aware assembly of context for LLM
```

---

### `script` — Multi-Language Scripting

All engines implement `ScriptEngine`:

```rust
trait ScriptEngine: Send + Sync {
    fn name(&self) -> &str;
    fn extensions(&self) -> &[&str];
    fn eval(&self, code: &str, ctx: &ScriptContext) -> Result<ScriptOutput>;
    fn call_function(&self, path, fn_name, args, ctx) -> Result<ScriptOutput>;
    fn handles(&self, path: &Path) -> bool;
}
```

Engines inject a `zcode` global API: `read_file`, `write_file`, `shell`, `log`.

`ScriptManager` scans configured directories, converts each script file into a `ScriptTool`, and registers them in `ToolRegistry`.

---

### `mcp` — MCP Client

Implements the [Model Context Protocol](https://modelcontextprotocol.io/) spec:
- `McpClient` manages a stdio subprocess (Content-Length framing)
- `McpToolAdapter` wraps remote tool definitions as local `Tool` trait objects  
- Supports `tools/list` + `tools/call` JSON-RPC 2.0 methods

---

### `lsp` — LSP Client

`LspClient` speaks [Language Server Protocol](https://microsoft.github.io/language-server-protocol/):
- Stdio transport with `Content-Length` header framing
- Methods: `initialize`, `textDocument/didOpen`, `textDocument/definition`, `textDocument/references`, `textDocument/hover`, `textDocument/completion`
- Language auto-detection from file extension

---

### `git` — Git Integration

`GitDiff` uses `git` subprocess (no libgit2 dependency):

| Method | Description |
|---|---|
| `is_git_repo(path)` | Detect if path is under git |
| `repo_root(path)` | Find repository root |
| `changed_files(path)` | List modified/added/deleted files |
| `full_diff(path)` | Get complete unified diff |
| `recent_commits(path, n)` | Last N commit messages |
| `build_context(path)` | Build `DiffContext` with patch + file list |

`DiffContext::load_changed_contents()` — lazy-loads only the changed files (not entire repo).

---

### `session` — Snapshot Manager

`SnapshotManager` persists workspace snapshots to SQLite:

```
snapshots table  — id, name, description, timestamp
files table      — snapshot_id, relative_path, content
```

Key operations: `save_workspace()`, `restore()`, `list()`, `diff()`.

---

### `ast` — Language & Grammar

```
LanguageRegistry   — registers LanguageProvider per language
LanguageProvider   — parses source into AstTree via tree-sitter

GrammarRegistry    — maps file extensions to language names
  ├── 17 built-in languages (Rust, Python, JS/TS, Go, C/C++, …)
  └── register_from_path() — runtime custom grammar (.so/.dylib)
```

---

### `workspace` — Integration Facade

`Workspace` is the top-level API that wires everything together:

```rust
let mut ws = Workspace::open("./my-project")?;

// Git diff-aware context (only changed files, token-budgeted)
let ctx = ws.build_diff_context(32_000)?;
let prompt_addition = ctx.as_prompt_context();

// Snapshot before a big change
let id = ws.snapshot_save("before-refactor", None)?;

// Restore if something goes wrong
ws.snapshot_restore(id)?;
```

---

## Data Flow: User Request → LLM Response

```
1. User types request in TUI / CLI
2. Workspace::build_diff_context() loads only changed files (token-budget)
3. ContextAssembler assembles: system prompt + memory + diff + file snippets
4. OrchestratorAgent routes to Planner or Coder
5. CoderAgent calls LLM with assembled context
6. LLM responds with text + optional tool_calls JSON
7. AgentLoop dispatches tool calls → ToolRegistry::execute()
8. Results injected back as tool messages → loop continues
9. Final response displayed in TUI / printed to stdout
10. ReviewerAgent optionally reviews any new diff
11. SnapshotManager auto-saves if config.snapshots.auto_snapshot = true
```

---

## Dependency Philosophy

| Concern | Choice | Rationale |
|---|---|---|
| Async runtime | tokio | Industry standard, great ecosystem |
| Serialization | serde + serde_json | Ubiquitous, zero-cost |
| TUI | ratatui | Maintained fork of tui-rs |
| Lua scripting | mlua (vendored) | No external lua runtime needed |
| JS scripting | rquickjs (vendored) | No V8/Node dependency |
| Python scripting | pyo3 | Leverages system Python |
| SQLite | rusqlite | Single-file, no server |
| LSP/MCP | pure stdio | No extra lib, max portability |
| Git | subprocess | No libgit2 linking complexity |
| Tree-sitter | runtime dlopen | Custom grammars without recompiling |
