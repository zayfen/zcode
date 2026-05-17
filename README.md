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
| Session | One JSONL file per session, LanceDB related-history retrieval, load/list/delete, and deterministic compression |
| TUI | Ratatui chat interface |

## Build

```bash
cargo build --workspace
cargo test --workspace
```

For day-to-day startup checks, run the compiled binary directly:

```bash
cargo build --workspace
target/debug/zcode chat
```

`cargo run -- chat` is useful while developing, but it includes Cargo's check,
compile, link, and run overhead. Do not use it as the measure for zcode's TUI
startup latency.

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
target/debug/zcode chat

# Initialize/validate standardized requirement docs
target/debug/zcode docs init
target/debug/zcode docs check

# Run a task through the agent workflow
target/debug/zcode run "Implement the next task from docs"

# Manage persisted task records
target/debug/zcode task list
target/debug/zcode task sync
target/debug/zcode task run <task-id-or-description>
target/debug/zcode task run-all -j 2
```

Use `--skip-docs-check` if you need to run before the `docs/` scaffold is valid.

## Session Context

Interactive chat sessions are stored as JSONL files under `.zcode/sessions/`.
Before each new prompt, zcode queries a derived LanceDB index in
`.zcode/session-index/` and injects only related prior turns into the agent
context. Unrelated prompts start fresh, which avoids mixing answers across
topics in the same session.

## Workspace Crates

| Crate | Responsibility |
|---|---|
| `zcode_ui` | CLI/TUI screen rendering |
| `zcode_requirements` | Requirement docs and task store |
| `zcode_orchestration` | Agent graph and ReAct execution |
| `zcode_llm_provider` | OpenAI-compatible provider |
| `zcode_capabilities` | Skills, MCP, tool calls, shared context |
| `zcode_session` | JSONL sessions, LanceDB related-history retrieval, and compression |
| `zcode_core` | Shared errors, config, and DTOs |

See [ARCHITECTURE.md](ARCHITECTURE.md), [中文架构说明](docs/architecture.zh-CN.md), and [USAGE.md](USAGE.md) for details.
