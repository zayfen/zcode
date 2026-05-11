# zcode Usage Reference

## Global Flags

```text
zcode [global-flags] <command>

Global flags:
  --skip-docs-check       Skip docs/ validation
  -m, --model <model>     Override ZCODE_MODEL for this run
  -M, --mcp <command>     Start an extra MCP stdio server
  -v, --verbose           Enable verbose tracing
  -h, --help              Print help
  -V, --version           Print version
```

## Commands

### `zcode chat`

Launch the interactive TUI chat interface.

```bash
zcode chat
zcode --model gpt-4o-mini chat
```

### `zcode run`

Run a task through the agent workflow.

```bash
zcode run "Refactor the LLM provider"
zcode run "Fix the failing task" --resume <TASK_ID>
zcode run "Implement docs parser" --max-iterations 80
```

The task workflow uses planner -> ReAct coder -> reviewer/test gate with bounded coder retries. Simple tasks can use `ZCODE_FAST_MODEL`.

### `zcode feed`

Feed raw requirement documents into the standardized `docs/` structure.

```bash
zcode feed requirements.md
zcode feed raw-requirements/ --investigate
```

### `zcode docs`

Manage requirement docs.

```bash
zcode docs init
zcode docs check
```

`docs init` creates the required scaffold. `docs check` validates the project against the zcode requirements convention.

### `zcode task`

Manage persisted task records in `.zcode/tasks/`.

```bash
zcode task list
zcode task show <TASK_ID>
zcode task sync
zcode task run <TASK_ID>
zcode task run "Direct task description"
zcode task run-all -j 2
zcode task clean
```

## LLM Environment

| Variable | Description |
|---|---|
| `ZCODE_BASE_URL` | OpenAI-compatible API base URL or full chat completions endpoint |
| `ZCODE_API_KEY` | Bearer token for LLM requests |
| `ZCODE_MODEL` | Main model |
| `ZCODE_FAST_MODEL` | Optional fast model for simple tasks |

Example:

```bash
export ZCODE_BASE_URL="https://api.openai.com/v1"
export ZCODE_API_KEY="sk-..."
export ZCODE_MODEL="gpt-4o"
export ZCODE_FAST_MODEL="gpt-4o-mini"
```

## MCP Tools

Tools are provided by MCP/capability providers. Built-in local file, shell, search, glob, and AST tools are not registered by default.

Configure MCP servers in `.zcode/config.toml`:

```toml
[[mcp_servers]]
name = "filesystem"
command = "mcp-server-filesystem"
args = ["/workspace"]
auto_start = true
```

Or attach an extra stdio MCP server from the CLI:

```bash
zcode -M "mcp-server-filesystem /workspace" run "Inspect the project"
```

## Docs Structure

`zcode_requirements` expects:

```text
docs/
  prd/
    *.md
  specs/
    coding.spec.md
  tasks/
    *.tasks.md
  validation.md
  review-checklist.md
```

Run `zcode docs init` to create the scaffold.

## Development

```bash
cargo check --workspace
cargo test --workspace --lib
cargo test --test cli_test
cargo test --test registry_test
cargo test --test reviewer_integration
```

Integration tests cover CLI parsing, capability registry behavior, and reviewer heuristics.
