# zcode Architecture Design

**Date:** 2026-03-19
**Status:** Approved
**Type:** CLI Coding Agent

## Executive Summary

zcode is a Rust-based programming agent inspired by Claude Code and Cursor. It follows Unix philosophy ("do one thing well") while providing advanced features like multi-agent parallelism, persistent semantic memory, and built-in code intelligence.

## Design Goals

| Goal | Priority |
|------|----------|
| CLI Companion for terminal-based coding | High |
| Provider-agnostic LLM support (20+ providers) | High |
| Rich TUI with Ratatui | High |
| MCP Client for extensibility | High |
| Full code intelligence (AST + LSP + Semantic) | High |
| Multi-agent parallel execution | High |
| Tiered memory system | High |
| Multi-language scripting (Python/JS/Lua/Bash) | High |
| Trust-based permissions (Unix philosophy) | High |

---

## 1. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              zcode                                       │
│                     "A Rust-powered coding agent"                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                         CLI Entry (clap)                           │ │
│  │   zcode [task]           - Run a task                              │ │
│  │   zcode chat             - Interactive chat mode                   │ │
│  │   zcode --mcp <server>   - Add MCP server for session              │ │
│  └───────────────────────────┬────────────────────────────────────────┘ │
│                              │                                           │
│  ┌───────────────────────────┴────────────────────────────────────────┐ │
│  │                      TUI Layer (Ratatui + Crossterm)               │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │ │
│  │  │ Chat Panel  │  │  Code Panel │  │ Status Bar  │                 │ │
│  │  │ (streaming) │  │ (diff view) │  │ (todos)     │                 │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                 │ │
│  └───────────────────────────┬────────────────────────────────────────┘ │
│                              │                                           │
│  ┌───────────────────────────┴────────────────────────────────────────┐ │
│  │                      Agent Layer (Multi-Agent)                     │ │
│  │  ┌──────────────┐   ┌─────────┐ ┌─────────┐ ┌─────────┐           │ │
│  │  │ Orchestrator │──►│ Planner │ │  Coder  │ │Reviewer │           │ │
│  │  │   Agent      │   │ Agent   │ │ Agent   │ │ Agent   │           │ │
│  │  └──────────────┘   └─────────┘ └─────────┘ └─────────┘           │ │
│  └───────────────────────────┬────────────────────────────────────────┘ │
│                              │                                           │
│  ┌───────────────────────────┴────────────────────────────────────────┐ │
│  │                      Tool Layer (Extensible)                       │ │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐           │ │
│  │  │ File   │ │ Shell  │ │ Search │ │  LSP   │ │  AST   │           │ │
│  │  │ I/O    │ │ Exec   │ │ (grep) │ │ Client │ │(tree-sitter)│       │ │
│  │  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘           │ │
│  └───────────────────────────┬────────────────────────────────────────┘ │
│                              │                                           │
│  ┌───────────────────────────┴────────────────────────────────────────┐ │
│  │                      Infrastructure Layer                          │ │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐  │ │
│  │  │ Rig (LLM)   │ │ MCP Client  │ │ Memory      │ │ Scripting   │  │ │
│  │  │ 20+ models  │ │ (rmcp)      │ │ (SQLite)    │ │ (multi)     │  │ │
│  │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘  │ │
│  │  ┌─────────────┐ ┌─────────────┐                                   │ │
│  │  │ Embeddings  │ │ Semantic    │                                   │ │
│  │  │ (FastEmbed) │ │ Search      │                                   │ │
│  │  └─────────────┘ └─────────────┘                                   │ │
│  └────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

**Key Principles:**
- **Single binary** with clean module boundaries
- **Streaming-first** - All LLM calls stream to UI in real-time
- **Async throughout** - Tokio runtime, non-blocking operations
- **Plugin tools** - Tools are dynamically registered, scriptable

---

## 2. Multi-Agent Architecture

### Orchestrator Agent

**Responsibilities:**
- Parse user intent and decompose complex tasks
- Route to appropriate subagent
- Track progress and manage todos
- Aggregate results and report to user

**State Machine:**
```
Idle ──analyze──► Planning ──delegate──► Executing
   ▲                                           │
   └──────────── completed ◄───────────────────┘
```

### Specialized Subagents

| Agent | Model Preference | Tools | Output |
|-------|-----------------|-------|--------|
| **Planner** | Claude Sonnet | read_file, search, glob, lsp_* | Plan, Todo list |
| **Coder** | Claude Sonnet/Haiku | read_file, write_file, edit_file, execute, ast_edit | Code changes, Diff |
| **Reviewer** | Claude Opus | read_file, diff, test_run, lint, lsp_* | Review, Suggestions |

### Agent Communication

```rust
enum AgentMessage {
    TaskAssigned { task: Task, context: Context },
    ProgressUpdate { agent: AgentId, progress: f32 },
    ToolRequest { tool: ToolCall },
    ToolResult { result: ToolResult },
    TaskCompleted { result: TaskResult },
    SubTaskSpawned { subagent: AgentType, task: Task },
}
```

### Parallel Execution

- Multiple Coders can work on independent files simultaneously
- Reviewer runs after Coder completes
- Planner can explore while Coder works

---

## 3. Tool Layer & Scripting System

### Tool Trait

```rust
trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> JsonSchema;
    async fn execute(&self, args: Value) -> ToolResult;
}
```

### Built-in Tools

| Tool | Parameters | Description |
|------|------------|-------------|
| `file_read` | path, limit?, offset? | Read file contents |
| `file_write` | path, content | Write/create file |
| `file_edit` | path, old, new | Replace text in file |
| `shell_exec` | cmd, args, cwd? | Execute shell command |
| `search` | pattern, path | Grep-style search |
| `glob` | pattern, path | File pattern matching |
| `ast_edit` | path, pattern, replacement | Structural code edit |
| `ast_search` | pattern, lang | Semantic code search |
| `lsp_goto_def` | file, line, col | Go to definition |
| `lsp_refs` | file, line, col | Find references |
| `lsp_hover` | file, line, col | Get type info |
| `lsp_complete` | file, line, col | Get completions |

### Scripting Engines

| Language | Crate | Use Case |
|----------|-------|----------|
| **Lua** | mlua | Fast, embedded, configuration |
| **Python** | pyo3 | Rich ecosystem, ML tools |
| **JavaScript** | deno_runtime | NPM packages, async |
| **Bash** | native subprocess | Shell integration, quick scripts |

### Shared Scripting API

```python
# Python example
zcode.read_file(path) -> str
zcode.write_file(path, content)
zcode.edit_file(path, old, new)
zcode.shell(cmd, args) -> {stdout, stderr, code}
zcode.search(pattern, path) -> [Match]
zcode.glob(pattern) -> [Path]
zcode.lsp_goto_def(file, line, col) -> Location
zcode.ast_edit(path, pattern, replacement)
zcode.register_tool(name, handler)  # Create custom tool
zcode.log(level, message)
```

### Script Locations

```
~/.config/zcode/scripts/     # User global scripts
.zcode/scripts/              # Project-specific scripts
~/.config/zcode/hooks/       # Lifecycle hooks
```

---

## 4. Advanced Features (Differentiators)

### 4.1 True Multi-Agent Parallelism

Unlike Claude Code's sequential execution, zcode runs multiple agents in parallel:

```
Orchestrator
    ├──► Coder-1: refactor handlers/auth.rs    [35%]
    ├──► Coder-2: refactor handlers/user.rs    [60%]
    ├──► Coder-3: refactor handlers/post.rs    [20%]
    └──► Planner:   analyze handlers/admin.rs  [analyzing]
```

### 4.2 Persistent Semantic Memory

Three-tier memory system:

1. **Working Memory** (session): Recent files, edits, commands, current task state
2. **Project Memory** (SQLite): Architecture decisions, patterns discovered, conventions, mistakes to avoid
3. **Semantic Index** (embeddings): Code similarity search, "find code like this", cross-file relationships

### 4.3 Native Code Intelligence

Built-in Tree-sitter + LSP for 40+ languages:
- Structural edits: "Rename all usages of Foo to Bar"
- Smart navigation without LSP server
- Pattern matching with ast-grep
- Refactoring: Extract function, inline variable, etc.

### 4.4 Model Picker

Auto-select optimal model per task:

| Task Type | Auto-Selected Model |
|-----------|-------------------|
| Complex reasoning | Claude Opus 4 / GPT-4 |
| Code generation | Claude Sonnet 4 / DeepSeek V3 |
| Quick edits | Claude Haiku / GPT-4o-mini |
| Code review | Claude Opus 4 |
| Local/privacy | Ollama (Llama 4 / Qwen 3) |
| Long context | Gemini 2.0 (1M tokens) |
| Cheap bulk work | DeepSeek V3 |

### 4.5 Spec-Driven Development

Write spec → Generate tests → Implement → Verify loop:

```bash
$ zcode spec "User authentication with JWT"  # Generates spec.md
$ zcode test --generate                       # Creates tests from spec
$ zcode implement                             # Implements until tests pass
$ zcode verify                                # Runs full verification
```

### 4.6 Continuous Code Review

Watch mode with auto-review:
```bash
$ zcode watch --review
```

### 4.7 Diff-Aware Context

Only loads relevant files based on git diff, saving tokens.

### 4.8 Session Snapshots & Rollback

```bash
$ zcode session save "before-refactor"
$ zcode session list
$ zcode session rollback before-refactor
```

---

## 5. Memory & Context Management

### Token Budget Manager

```
Total Budget: 200K tokens
├── System: 10K
├── Conversation: 80K
├── File Context: 60K
├── Tool Results: 40K
└── Available: 10K
```

### Context Assembly Algorithm

1. Start with system prompt
2. Add project memory (conventions, patterns)
3. Semantic search for relevant code
4. Add conversation history
5. Fill remaining with file contents

---

## 6. Terminal UI (Ratatui)

### Main Views

| View | Shortcut | Description |
|------|----------|-------------|
| Chat | Default | Conversation with streaming responses |
| Diff | Ctrl+D | Side-by-side code changes |
| Logs | Ctrl+L | Debug and trace logs |
| Todos | Ctrl+T | Task progress checklist |
| Multi-Agent | Auto | Parallel agent status cards |

### Key Bindings

| Key | Action |
|-----|--------|
| Ctrl+T | Toggle todos |
| Ctrl+D | Toggle diff view |
| Ctrl+L | Toggle logs |
| Ctrl+P | Command palette |
| Ctrl+S | Save snapshot |
| Ctrl+Z | Undo last change |
| Ctrl+C | Interrupt agent |
| Ctrl+Q | Quit |

### UI Components

- `StreamingText` - Real-time LLM response rendering
- `ToolCallBlock` - Collapsible tool execution display
- `TodoList` - Interactive checklist with progress
- `DiffView` - Side-by-side code comparison
- `AgentCard` - Mini status card for parallel agents
- `CommandPalette` - Fuzzy search for commands/files

---

## 7. Technology Stack

### Core Dependencies

```toml
# Runtime
tokio = { version = "1", features = ["full"] }
futures = "0.3"
async-trait = "0.1"

# LLM
rig-core = "0.5"
rig-provider-anthropic = "0.2"
rig-provider-openai = "0.2"

# MCP
rmcp = { version = "0.16", features = ["client"] }

# TUI
ratatui = "0.28"
crossterm = "0.28"
tui-textarea = "0.6"

# Code Intelligence
tree-sitter = "0.24"
ast-grep-core = "0.26"
tower-lsp = "0.20"

# Memory
rusqlite = { version = "0.32", features = ["bundled"] }
sqlite-vec = "0.1"
fastembed = "4"

# Scripting
mlua = { version = "0.10", features = ["luau", "async"] }
pyo3 = { version = "0.22", features = ["auto-initialize"] }
deno_runtime = "0.200"

# CLI & Config
clap = { version = "4", features = ["derive", "env"] }
directories = "5"
config = "0.14"
toml = "0.8"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "0.8"

# Error & Logging
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"

# File System & Git
walkdir = "2"
glob = "0.3"
ignore = "0.4"
gix = "0.67"
similar = "2"
```

---

## 8. Project Structure

```
zcode/
├── Cargo.toml
├── src/
│   ├── main.rs                 # Entry point
│   ├── lib.rs                  # Library root
│   ├── cli/                    # CLI layer
│   ├── tui/                    # Terminal UI
│   │   ├── app.rs
│   │   └── widgets/
│   ├── agent/                  # Multi-agent system
│   │   ├── orchestrator.rs
│   │   ├── planner.rs
│   │   ├── coder.rs
│   │   └── reviewer.rs
│   ├── llm/                    # LLM integration
│   ├── tools/                  # Built-in tools
│   ├── mcp/                    # MCP client
│   ├── memory/                 # Memory system
│   ├── script/                 # Scripting engines
│   ├── context/                # Context management
│   └── config/                 # Configuration
├── tests/
├── benches/
├── examples/
└── docs/
```

---

## 9. Implementation Roadmap

### Phase 1: Foundation (Week 1-2)
- [ ] Project structure setup
- [ ] CLI with clap
- [ ] Rig integration for LLM calls
- [ ] Simple TUI with chat interface
- [ ] Basic tools: read/write/edit/execute
- [ ] Streaming response rendering

### Phase 2: Code Intelligence (Week 3-4)
- [ ] Tree-sitter integration
- [ ] Language grammar loading
- [ ] AST search and edit tools
- [ ] Diff view in TUI
- [ ] Search tool with ripgrep-style output

### Phase 3: Multi-Agent System (Week 5-6)
- [ ] Agent trait and state machine
- [ ] Orchestrator agent
- [ ] Specialized agents
- [ ] Agent message bus
- [ ] Parallel agent coordination
- [ ] Model picker logic

### Phase 4: Memory System (Week 7-8)
- [ ] SQLite storage for project memory
- [ ] Working memory implementation
- [ ] FastEmbed integration
- [ ] Semantic search
- [ ] Context assembler with token budget

### Phase 5: Extensibility (Week 9-10)
- [ ] MCP client integration
- [ ] Lua/Python/JS scripting
- [ ] Script tool registration
- [ ] Hook system
- [ ] Configuration files

### Phase 6: Advanced Features (Week 11-12)
- [ ] LSP client integration
- [ ] Spec-driven development
- [ ] Continuous review mode
- [ ] Session snapshots
- [ ] Diff-aware context

### Phase 7: Polish & Release (Week 13-14)
- [ ] Test suite
- [ ] Performance optimization
- [ ] Documentation
- [ ] CI/CD pipeline
- [ ] Release binaries

---

## 10. Success Metrics

### Performance Targets
- Startup time: < 100ms
- First token latency: < 500ms
- File read (1MB): < 50ms
- Semantic search: < 100ms for 10K chunks
- Memory usage: < 200MB base
- Binary size: < 30MB

### Quality Targets
- Test coverage: > 80%
- Zero clippy warnings
- Documentation coverage: > 90% public API

---

## 11. Feature Comparison

| Feature | Claude Code | zcode |
|---------|-------------|-------|
| Multi-agent parallel | ❌ Sequential | ✅ True parallel |
| Persistent memory | ❌ Session only | ✅ Tiered + semantic |
| Code intelligence | ⚠️ External tools | ✅ Built-in AST + LSP |
| Model flexibility | ❌ Claude only | ✅ 20+ providers |
| Model picker | ❌ Manual | ✅ Auto-select |
| Scripting | ⚠️ Hooks only | ✅ Python/JS/Lua/Bash |
| Spec-driven dev | ❌ | ✅ Built-in |
| Continuous review | ❌ | ✅ Watch mode |
| Session rollback | ❌ Git only | ✅ Full snapshots |
| Local models | ❌ | ✅ Ollama/LMStudio |
| Open source | ❌ | ✅ MIT/Apache |
