# zcode

> 🤖 A modular, extensible AI coding agent — built in Rust.

[![Rust](https://img.shields.io/badge/rust-2021_edition-orange.svg)](https://www.rust-lang.org)
[![Tests](https://img.shields.io/badge/tests-867_passing-brightgreen.svg)](#)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**zcode** is a terminal-based programming assistant that combines a multi-agent orchestration system, multi-language scripting, LSP integration, and session management into a single, dependency-light binary.

---

## ✨ Features

| Category | Capability |
|---|---|
| **Multi-Agent** | Orchestrator / Planner / Coder / Reviewer agents with async message bus |
| **LLM Providers** | Anthropic Claude, OpenAI GPT, local Ollama — pluggable via `LlmProvider` trait |
| **Tool System** | Built-in tools (file, shell, search, AST) + custom tools via scripting or MCP |
| **Scripting** | Lua 5.4, Python, JavaScript (QuickJS), Shell — with lifecycle hooks |
| **MCP Client** | JSON-RPC 2.0 over stdio — connect any MCP-compatible tool server |
| **LSP Client** | goto-definition, find-references, hover, completion for any LSP server |
| **Session Snapshots** | SQLite-backed workspace snapshots with save / restore / diff |
| **Git-Aware Context** | Only loads changed files into LLM context (token-budget-aware) |
| **Grammar Registry** | 17 built-in languages + runtime custom Tree-sitter grammar loading |
| **TUI** | Ratatui-based chat interface with syntax highlighting |

---

## 🚀 Quick Start

### Installation

```bash
# Clone and build
git clone https://github.com/zayfen/zcode.git
cd zcode
cargo build --release

# Run
./target/release/zcode
```

### Initialize a project

```bash
# Create .zcode/config.toml in current directory
zcode init
```

### Chat with the agent

```bash
# Start TUI chat
zcode chat

# One-shot query
zcode ask "Refactor the error handling in src/main.rs to use ? operator"

# Review current git changes
zcode review

# Save a snapshot before a big change
zcode snapshot save "before-refactor"

# Restore a snapshot
zcode snapshot restore <id>
```

---

## ⚙️ Configuration

`zcode` is configured via `.zcode/config.toml` in your project root:

```toml
name = "my-project"
languages = ["rust", "typescript"]
frameworks = ["tokio", "react"]

# LLM provider override
[llm]
provider = "anthropic"          # anthropic | openai | ollama
model    = "claude-3-5-sonnet-20241022"
temperature = 0.7

# Tool access controls
[tools]
disabled = ["delete_file"]

# MCP servers (auto-started on project open)
[[mcp_servers]]
name    = "filesystem"
command = "mcp-server-filesystem"
args    = ["/workspace"]

# LSP servers
[[lsp_servers]]
language = "rust"
command  = "rust-analyzer"

[[lsp_servers]]
language = "python"
command  = "pylsp"

# Custom scripting hooks
[scripts]
script_dirs = [".zcode/scripts"]
[scripts.hooks]
before_tool       = ".zcode/scripts/before_tool.lua"
on_task_complete  = ".zcode/scripts/notify.py"

# Session snapshots
[snapshots]
db_path       = ".zcode/snapshots.db"
max_snapshots = 50
auto_snapshot = true

# Custom Tree-sitter grammars
[[grammars]]
language     = "zig"
library_path = "/usr/lib/tree-sitter-zig.so"
extensions   = ["zig"]
```

Global settings live in `~/.config/zcode/settings.toml`:

```toml
[llm]
provider   = "anthropic"
api_key    = "sk-ant-..."
model      = "claude-3-5-sonnet-20241022"
max_tokens = 8192

[tui]
theme = "dark"
```

---

## 📝 Writing Scripts

Scripts in `.zcode/scripts/` are automatically loaded as tools. Each script exposes a `process(args_json)` function:

**Lua** (`.lua`):
```lua
function process(args_json)
    local content = zcode.read_file("src/main.rs")
    zcode.log("Processing: " .. content:len() .. " bytes")
    return content:upper()
end
```

**JavaScript** (`.js`):
```js
function process(args) {
    const content = zcode.read_file("package.json");
    const pkg = JSON.parse(content);
    return `Project: ${pkg.name} v${pkg.version}`;
}
```

**Python** (`.py`):
```python
def process(args):
    result = zcode.shell("pytest --tb=short")
    return result["stdout"]
```

**Shell** (`.sh`):
```sh
function process() {
    echo "Build output:"
    cargo build --release 2>&1
}
```

### Available Script API

| Function | Description |
|---|---|
| `zcode.read_file(path)` | Read file contents as string |
| `zcode.write_file(path, content)` | Write string to file, returns bool |
| `zcode.shell(cmd)` | Run shell command, returns `{stdout, stderr, exit_code}` |
| `zcode.log(message)` | Log to agent output |

---

## 🧪 Testing

```bash
# Unit tests
cargo test

# Integration tests only
cargo test --tests

# Specific test file
cargo test --test workspace_integration
cargo test --test scripting_integration
cargo test --test reviewer_integration
cargo test --test grammar_integration
```

---

## 🏗️ Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full system design.

---

## 📖 Usage Reference

See [USAGE.md](USAGE.md) for the complete CLI reference.

---

## 📄 License

MIT — see [LICENSE](LICENSE).
