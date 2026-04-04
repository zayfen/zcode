<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# zcode

## Purpose
A modular AI coding agent CLI tool built in Rust. Zcode provides multi-agent orchestration (orchestrator, planner, coder, reviewer), LLM integration (Anthropic, OpenAI, Ollama), a multi-language scripting engine (Lua, Python, JS, Shell), code intelligence via Tree-sitter, LSP/MCP client support, and git-aware context management — all in a single crate with well-defined trait boundaries.

## Key Files
| File | Description |
|------|-------------|
| `Cargo.toml` | Project manifest — Rust 2021, MSRV 1.75, all dependencies |
| `src/lib.rs` | Crate root — module declarations and public re-exports |
| `src/main.rs` | CLI entry point — clap arg parsing, tracing init, command dispatch |
| `src/error.rs` | Unified `ZcodeError` enum covering all failure modes |
| `ARCHITECTURE.md` | Detailed architecture doc with diagrams and data flow |
| `README.md` | User-facing project overview |
| `USAGE.md` | Usage instructions |

## Subdirectories
| Directory | Purpose |
|-----------|---------|
| `src/` | All source code — 17 submodules (see `src/AGENTS.md`) |
| `tests/` | Integration tests (see `tests/AGENTS.md`) |
| `docs/` | Design docs, PRDs, specs, plans (see `docs/AGENTS.md`) |
| `templates/` | Document templates for PRD, spec, tasks, etc. (see `templates/AGENTS.md`) |
| `example/` | Example project showing zcode config and docs structure (see `example/AGENTS.md`) |
| `.worktrees/` | Git worktrees for isolated development |

## For AI Agents

### Working In This Directory
- Build: `cargo build`
- Test: `cargo test`
- Run: `cargo run -- <args>`
- This is a single-crate Rust project (not a workspace)
- All modules live in `src/` with `mod.rs` as the module root
- Dependencies are pinned — check `Cargo.toml` before adding new ones
- The project uses `anyhow` for top-level errors and `thiserror` for library errors

### Testing Requirements
- Run `cargo test` before committing changes
- Integration tests live in `tests/` — unit tests are inline in each module
- MSRV is 1.75 — avoid features requiring newer Rust

### Common Patterns
- All traits use `async_trait` for async methods
- Tool system: implement `Tool` trait, register via `ToolRegistry`
- LLM providers: implement `LlmProvider` trait
- Agent types: implement `AgentTrait` via the agent bus system
- Configuration: `Settings` (global) and `ProjectConfig` (per-project)

## Dependencies

### External
- **tokio** — async runtime (full features)
- **clap** — CLI argument parsing (derive macro)
- **serde / serde_json** — serialization
- **reqwest** — HTTP client for LLM APIs
- **ratatui + crossterm** — TUI framework
- **tree-sitter** — code parsing and AST queries
- **rusqlite** — SQLite for session snapshots
- **mlua** — Lua scripting engine (vendored)
- **pyo3** — Python scripting engine
- **rquickjs** — JavaScript scripting engine (vendored)
- **lsp-types** — LSP protocol types

<!-- MANUAL: Custom project notes can be added below -->
