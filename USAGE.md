# zcode Usage Reference

Complete reference for all `zcode` CLI commands and workflows.

---

## Global Flags

```
zcode [global-flags] <command> [command-flags]

Global flags:
  -c, --config <path>    Path to config file (default: .zcode/config.toml)
  -v, --verbose          Enable verbose logging
  -q, --quiet            Suppress non-essential output
      --json             Output results as JSON (for scripting)
  -h, --help             Print help
  -V, --version          Print version
```

---

## Commands

### `zcode init`

Initialize a new zcode project in the current directory.

```bash
zcode init
zcode init --name "my-api" --language rust --language typescript
```

Creates `.zcode/config.toml` and `.zcode/scripts/` directory.

**Flags:**

| Flag | Description |
|---|---|
| `--name <name>` | Project name (defaults to directory name) |
| `--language <lang>` | Add a language (repeatable) |
| `--framework <fw>` | Add a framework (repeatable) |
| `--force` | Overwrite existing config |

---

### `zcode chat`

Launch the interactive TUI chat interface.

```bash
zcode chat
zcode chat --model claude-3-5-sonnet-20241022
zcode chat --system "You are a senior Rust developer"
```

**Keybindings in TUI:**

| Key | Action |
|---|---|
| `Enter` | Send message |
| `Ctrl+C` | Interrupt current generation |
| `Ctrl+Q` | Quit |
| `Ctrl+S` | Save snapshot |
| `Ctrl+R` | Show recent snapshots |
| `↑ / ↓` | Scroll history |
| `PgUp / PgDn` | Scroll by page |

---

### `zcode ask`

Send a single one-shot request and print the response.

```bash
zcode ask "Add error handling to src/main.rs"
zcode ask "What does the ContextAssembler do?" --no-tools
zcode ask "Refactor this function" --file src/lib.rs --line 42
```

**Flags:**

| Flag | Description |
|---|---|
| `--file <path>` | Include this file in context |
| `--line <n>` | Focus on this line number |
| `--no-tools` | Disable tool calling (pure text response) |
| `--model <model>` | Override LLM model |
| `--max-tokens <n>` | Override max tokens |

---

### `zcode review`

Review current git changes using the ReviewerAgent.

```bash
# Review all staged changes
zcode review

# Review a specific file
zcode review --file src/agent/reviewer.rs

# Review a specific diff file
zcode review --diff changes.patch

# Review with only security checks
zcode review --only security

# Output as JSON
zcode review --json
```

**Flags:**

| Flag | Description |
|---|---|
| `--file <path>` | Review a specific file |
| `--diff <path>` | Review a diff patch file |
| `--only <category>` | Limit to: logic, security, performance, style, testing |
| `--max-issues <n>` | Maximum issues to report (default: 20) |

**Review categories:**

| Category | Checks |
|---|---|
| `logic` | `.unwrap()` calls, `panic!()` |
| `security` | Hardcoded credentials, SQL injection |
| `performance` | Unnecessary `.clone()` on collections |
| `style` | Lines > 120 chars |
| `testing` | New functions without tests |

---

### `zcode snapshot`

Manage workspace snapshots backed by SQLite.

#### `zcode snapshot save`

```bash
zcode snapshot save "before-refactor"
zcode snapshot save "v2.0-baseline" --description "Stable state before API redesign"
```

#### `zcode snapshot list`

```bash
zcode snapshot list

# Output:
# ID  NAME              DATE                 DESCRIPTION
# 1   before-refactor   2026-03-25 00:10     -
# 2   v2.0-baseline     2026-03-25 00:15     Stable state before API redesign
```

#### `zcode snapshot restore`

```bash
zcode snapshot restore 1
zcode snapshot restore 1 --dry-run    # Show what would be restored
```

#### `zcode snapshot diff`

```bash
# Show diff between snapshot and current state
zcode snapshot diff 1
zcode snapshot diff 1 2    # Diff between two snapshots
```

#### `zcode snapshot delete`

```bash
zcode snapshot delete 1
zcode snapshot delete --all    # Delete all snapshots
```

---

### `zcode diff`

Show a git-aware diff context (what the agent would see).

```bash
# Show which files are changed and their content
zcode diff

# Show diff with token budget estimate
zcode diff --budget 32000

# Show only the file list
zcode diff --files-only

# Show recent commits
zcode diff --commits 10
```

**Flags:**

| Flag | Description |
|---|---|
| `--budget <n>` | Token budget (default: unlimited) |
| `--files-only` | List changed files without content |
| `--commits <n>` | Show last N commits (default: 5) |
| `--staged` | Only staged changes |

---

### `zcode tool`

Manage and run tools.

#### `zcode tool list`

```bash
zcode tool list

# Output:
# NAME              TYPE      SOURCE
# read_file         builtin   -
# write_file        builtin   -
# shell             builtin   -
# my_formatter      script    .zcode/scripts/format.lua
# filesystem        mcp       mcp-server-filesystem
```

#### `zcode tool run`

```bash
# Run a tool directly
zcode tool run read_file '{"path": "src/main.rs"}'
zcode tool run shell '{"command": "cargo clippy"}'
```

---

### `zcode script`

Manage and run scripts.

```bash
# List loaded scripts
zcode script list

# Run a specific script
zcode script run .zcode/scripts/format.lua '{"target": "src/"}'

# Validate all scripts (syntax check)
zcode script validate
```

---

### `zcode lsp`

Interact with configured LSP servers.

```bash
# Show hover information at position
zcode lsp hover src/main.rs:42:10

# Go to definition
zcode lsp definition src/lib.rs:15:5

# Find all references
zcode lsp references src/agent/mod.rs:8:10

# Get completions at position
zcode lsp complete src/main.rs:20:15
```

---

### `zcode config`

View and edit configuration.

```bash
# Print current effective config
zcode config show

# Edit config in $EDITOR
zcode config edit

# Validate config syntax
zcode config validate

# Set a value
zcode config set llm.model "claude-3-5-sonnet-20241022"
zcode config set snapshots.max_snapshots 100
```

---

## Workflows

### Typical Development Session

```bash
# 1. Open an existing project
cd my-rust-project
zcode init    # if not yet initialized

# 2. Take a snapshot before making changes
zcode snapshot save "start-of-session"

# 3. Chat with the agent
zcode chat

# 4. After generating changes, review them
zcode review

# 5. If something went wrong, restore
zcode snapshot restore 1
```

---

### Using Scripts as Tools

Place scripts in the configured `script_dirs` (default: `.zcode/scripts/`).
They are auto-discovered and registered as tools with name `<basename_without_extension>`.

```
.zcode/scripts/
  format.lua          → tool "format"
  run_tests.sh        → tool "run_tests"
  analyze.js          → tool "analyze"
  coverage_report.py  → tool "coverage_report"
```

Each script must expose a `process(args_json)` function / function named after the CLI function.

---

### Connecting MCP Servers

Add to `.zcode/config.toml`:

```toml
[[mcp_servers]]
name    = "github"
command = "mcp-server-github"
args    = []
auto_start = true
```

The server's exposed tools are automatically registered in `ToolRegistry`.

---

### Custom Tree-sitter Grammars

For languages not built in (Zig, Gleam, Haskell, etc.):

1. Build the grammar shared library:
   ```bash
   cd tree-sitter-zig
   tree-sitter generate
   gcc -shared -fPIC -o zig.so src/parser.c
   ```

2. Add to `.zcode/config.toml`:
   ```toml
   [[grammars]]
   language     = "zig"
   library_path = "/usr/local/lib/zig.so"
   extensions   = ["zig"]
   ```

3. The grammar is loaded at startup and used for AST-aware tools.

---

## Environment Variables

| Variable | Description |
|---|---|
| `ZCODE_CONFIG` | Override config file path |
| `ZCODE_API_KEY` | LLM API key (overrides config) |
| `ZCODE_MODEL` | LLM model name (overrides config) |
| `ZCODE_LOG` | Log level: error, warn, info, debug, trace |
| `ZCODE_NO_COLOR` | Disable color output |

---

## Exit Codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | General error |
| `2` | Configuration error |
| `3` | LLM provider error |
| `4` | Tool execution error |
| `5` | Review failed (errors found) |
