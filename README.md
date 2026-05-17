# zcode &middot; [![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE) [![Rust](https://img.shields.io/badge/rust-1.91%2B-orange)](https://www.rust-lang.org)

**zcode** is a modular AI coding agent CLI built in Rust. It orchestrates a multi-agent workflow — planner, ReAct coder, reviewer, and self-learning — across a layered Cargo workspace, delivering deterministic, auditable, and high-throughput software engineering at the terminal.

---

## Why zcode

Most AI coding tools treat the agent as a black box: prompt in, code out. zcode decomposes the agent lifecycle into explicit, inspectable stages. Every plan, every tool call, every review decision is persisted as structured session data, indexed for retrieval, and auditable after the fact.

- **Agentic rigor** — a four-stage workflow (Plan &rarr; Code &rarr; Review &rarr; Learn) with bounded retry loops, not a single opaque inference pass.
- **Fresh-by-default context** — each prompt is matched against prior turns via a local LanceDB vector index. Unrelated questions start clean; related ones receive only relevant history.
- **Layered architecture** — seven focused crates with strict dependency direction. Swap the LLM provider, add capabilities, or replace the TUI without touching the orchestration core.
- **Offline-capable retrieval** — session context selection and compression run entirely without LLM calls. The vector index is derived from JSONL logs and can be rebuilt at any time.
- **OpenAI-compatible** — works with any provider that speaks the OpenAI chat completions protocol.

---

## Architecture at a Glance

```
src/main.rs          Binary shell
crates/zcode_cli     CLI dispatch (clap)
crates/zcode_ui      TUI rendering (Ratatui)
crates/zcode_requirements   Docs scaffold, validation, task store
crates/zcode_orchestration  Agent graph: Planner → Coder → Reviewer → Learning
crates/zcode_llm_provider   OpenAI-compatible chat completions
crates/zcode_capabilities   Skills, MCP tools, shared context
crates/zcode_session        JSONL sessions + LanceDB vector index
crates/zcode_core           Shared DTOs, errors, config
```

Dependencies flow strictly downward. `zcode_core` is the leaf; nothing depends on it circularly.

The agent graph runs four specialized roles coordinated by a root orchestrator:

| Agent | Role |
|---|---|
| **Orchestrator** | Root coordinator. Routes work, manages retry gates. |
| **Planner** | Reads standardized requirement docs and produces an executable plan. |
| **Coder** | ReAct loop: reason, call tools, observe, repeat, report. |
| **Reviewer** | Red/green test gate. Failures loop back to the coder with findings. |
| **Self-Learning** | Summarizes recurring errors into persistent correction notes. |

---

## Quick Start

### Prerequisites

- Rust 1.91+ (LanceDB MSRV)
- An OpenAI-compatible API endpoint

### Build

```bash
cargo build --workspace
cargo test --workspace
```

### Configure

```bash
export ZCODE_BASE_URL="https://api.openai.com/v1"
export ZCODE_API_KEY="sk-..."
export ZCODE_MODEL="gpt-4o"
export ZCODE_FAST_MODEL="gpt-4o-mini"   # optional, for simple tasks
```

### Launch

```bash
# Interactive TUI chat
target/debug/zcode chat

# Initialize requirement docs scaffold
target/debug/zcode docs init

# Run a task through the full agent workflow
target/debug/zcode run "Implement the next task from docs"

# Run all pending tasks with parallelism
target/debug/zcode task run-all -j 2
```

> **Note on startup performance:** measure with the compiled binary (`target/debug/zcode chat`). `cargo run -- chat` includes Cargo graph checks, incremental compilation, and linking overhead — it is not representative of zcode's runtime latency.

---

## Session Model

Every interactive chat session is stored as one append-only JSONL file under `.zcode/sessions/`. A derived LanceDB index at `.zcode/session-index/` powers related-turn retrieval.

| Design Property | Benefit |
|---|---|
| JSONL as source of truth | Human-readable, append-friendly, trivially recoverable |
| LanceDB as derived index | Real vector nearest-neighbor search; disposable and rebuildable |
| Fresh-by-default | Unrelated prompts start clean — no cross-topic contamination |
| Matched-turn injection | Only relevant user/assistant turns reach the LLM |
| LLM-free retrieval | Context selection works offline; LLM calls reserved for reasoning |
| Optional-context guard | History is framed as optional background; current prompt is authoritative |

---

## Capabilities

### MCP Tools

Tools are discovered and executed through the Model Context Protocol. Configure servers in `.zcode/config.toml`:

```toml
[[mcp_servers]]
name = "filesystem"
command = "mcp-server-filesystem"
args = ["/workspace"]
auto_start = true
```

Attach ad-hoc servers at runtime with `-M`:

```bash
zcode -M "mcp-server-filesystem /workspace" run "Inspect the project"
```

### Skills

Project skills live at `docs/skills/<name>/SKILL.md`. At runtime, zcode selects skills per prompt using generic relevance scoring over name, description, triggers, and body text. Unrelated skills are never injected into the LLM context.

```markdown
---
name: rust-conventions
description: Rust coding conventions for zcode
priority: high
triggers: rust, cargo, clippy, test
---

Use `ZcodeError` for production errors.
```

---

## Commands

| Command | Purpose |
|---|---|
| `zcode chat` | Launch interactive TUI |
| `zcode run <desc>` | Execute a task through the full agent workflow |
| `zcode docs init` | Scaffold standardized requirement docs |
| `zcode docs check` | Validate docs against the zcode convention |
| `zcode task list` | List persisted task records |
| `zcode task run-all -j N` | Execute all pending tasks with N-way parallelism |
| `zcode feed <path>` | Ingest raw requirements into the docs structure |

---

## Project Structure

```
zcode/
├── crates/                  # Workspace crates (layered architecture)
├── src/                     # Binary shell (main.rs) + re-exports (lib.rs)
├── docs/                    # Requirement docs, PRDs, specs
├── templates/               # Document templates
├── tests/                   # Integration tests
├── ARCHITECTURE.md          # Detailed architecture reference
├── USAGE.md                 # Usage reference
└── README.zh-CN.md          # 中文说明
```

---

## Development

```bash
cargo check --workspace
cargo test --workspace --lib
cargo test --test cli_test
cargo test --test registry_test
cargo test --test reviewer_integration
```

MSRV is Rust 1.91.0. All async traits use `async_trait`. Configuration flows through `Settings` and `ProjectConfig`.

---

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) — full architecture with data-flow diagrams
- [USAGE.md](USAGE.md) — detailed CLI usage and configuration
- [docs/architecture.zh-CN.md](docs/architecture.zh-CN.md) — 中文架构说明
- [README.zh-CN.md](README.zh-CN.md) — 中文 README

---

## License

MIT &copy; zcode contributors
