<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# src

## Purpose
All source code for the zcode crate. This is a modular monolith — each subdirectory is a module with its own `mod.rs`, public API, and trait boundaries. The crate root (`lib.rs`) declares all modules and re-exports key types.

## Key Files
| File | Description |
|------|-------------|
| `lib.rs` | Crate root — module declarations, public re-exports, and crate-level tests |
| `main.rs` | CLI entry point — clap arg parsing, tracing init, command dispatch |
| `error.rs` | Unified `ZcodeError` enum with `From` conversions for IO/JSON/TOML errors |

## Subdirectories
| Directory | Purpose |
|-----------|---------|
| `agent/` | Multi-agent orchestration (orchestrator, planner, coder, reviewer, bus) |
| `ast/` | AST parsing via Tree-sitter (language registry, grammar loading) |
| `cli/` | CLI argument parsing (clap) and command handlers |
| `config/` | User settings and project-level configuration (TOML) |
| `docs/` | Harness Engineering docs validator and scaffold generator |
| `git/` | Git integration via subprocess (diff, changed files, context builder) |
| `llm/` | LLM provider trait, Anthropic/OpenAI/Ollama HTTP clients, streaming, tool calls |
| `lsp/` | Language Server Protocol client (stdio transport) |
| `mcp/` | Model Context Protocol client (JSON-RPC 2.0 over stdio) |
| `memory/` | Three-tier memory: working (in-proc), project (disk), semantic (TF-IDF), context assembly |
| `script/` | Multi-language scripting engines (Lua, Python, JS, Shell) + hook registry |
| `session/` | Session snapshot manager (SQLite-backed) |
| `skills/` | Skills system — loads markdown skills from `docs/skills/` into system prompt |
| `task_store/` | Task progress persistence (JSON files in `.zcode/tasks/`) |
| `tools/` | Tool trait, registry, and built-in tools (file, shell, search, glob, AST) |
| `tui/` | Terminal UI (ratatui + crossterm) with chat interface |
| `workspace/` | Top-level facade wiring agents, tools, memory, git, snapshots |

## For AI Agents

### Working In This Directory
- All modules follow the `mod.rs` + sibling files pattern
- Public API is re-exported through `lib.rs`
- Use `crate::error::Result` as the standard result type
- New modules must be declared in `lib.rs` and added to the re-exports
- Build with `cargo build`, test with `cargo test`

### Testing Requirements
- Unit tests are inline (`#[cfg(test)] mod tests`) at the bottom of each file
- Integration tests live in `/tests/`
- Run `cargo test` from project root

### Common Patterns
- `impl Trait` for dependency injection (e.g., `LlmProvider`, `Tool`, `ScriptEngine`)
- `Arc<dyn Trait>` for shared ownership across threads
- `serde + schemars` for serializable config types
- `thiserror` for library errors, `anyhow` for application errors

## Dependencies

### Internal
- `error.rs` is imported by every other module
- `workspace/` depends on nearly all other modules as the integration facade

### External
- See root `Cargo.toml` for full dependency list

<!-- MANUAL: -->
