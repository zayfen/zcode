# zcode

`zcode` is a Rust AI coding agent CLI organized as a layered Cargo workspace.

## Features

| Category | Capability |
|---|---|
| Workspace layers | UI, requirements, orchestration, LLM provider, capabilities, session, core |
| Agent workflow | Planner, ReAct coder, reviewer/test gate, self-learning summaries |
| LLM provider | OpenAI-compatible chat completions |
| Models | `ZCODE_MODEL` plus optional `ZCODE_FAST_MODEL` for simple tasks |
| Capabilities | MCP tools, skills, and global shared context |
| Requirements | `docs/` scaffold, validation, task parsing, and task persistence |
| Session | Message history, load/list/delete, and deterministic compression |
| TUI | Ratatui chat interface |

## Build

```bash
cargo build --workspace
cargo test --workspace
```

## LLM Configuration

Set these environment variables before running real LLM workflows:

```bash
export ZCODE_BASE_URL="https://api.openai.com/v1"
export ZCODE_API_KEY="sk-..."
export ZCODE_MODEL="gpt-4o"
export ZCODE_FAST_MODEL="gpt-4o-mini"
```

`ZCODE_BASE_URL` can be a service root, a `/v1` root, or a full `/chat/completions` endpoint.

## Basic Usage

```bash
# Start the TUI chat
cargo run -- chat

# Initialize/validate standardized requirement docs
cargo run -- docs init
cargo run -- docs check

# Run a task through the agent workflow
cargo run -- run "Implement the next task from docs"

# Manage persisted task records
cargo run -- task list
cargo run -- task sync
cargo run -- task run <task-id-or-description>
cargo run -- task run-all -j 2
```

Use `--skip-docs-check` if you need to run before the `docs/` scaffold is valid.

## Workspace Crates

| Crate | Responsibility |
|---|---|
| `zcode_ui` | CLI/TUI screen rendering |
| `zcode_requirements` | Requirement docs and task store |
| `zcode_orchestration` | Agent graph and ReAct execution |
| `zcode_llm_provider` | OpenAI-compatible provider |
| `zcode_capabilities` | Skills, MCP, tool calls, shared context |
| `zcode_session` | Session messages and compression |
| `zcode_core` | Shared errors, config, and DTOs |

See [ARCHITECTURE.md](ARCHITECTURE.md) and [USAGE.md](USAGE.md) for details.
