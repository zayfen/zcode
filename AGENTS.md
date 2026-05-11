<!-- Generated: 2026-03-29 | Updated: 2026-05-11 -->

# zcode

## Purpose
A modular AI coding agent CLI tool built in Rust. Zcode is now a Cargo workspace with explicit layers for UI, requirement documentation, orchestration, OpenAI-compatible LLM access, capabilities, session management, and shared core types.

## Key Files
| File | Description |
|------|-------------|
| `Cargo.toml` | Root package plus workspace member declarations |
| `src/lib.rs` | Export shell over the layered crates |
| `src/main.rs` | Binary entry point: tracing init and dispatch into `zcode_cli` |
| `crates/zcode_cli/src/commands.rs` | CLI command handlers and workspace wiring |
| `crates/zcode_core/src/error.rs` | Unified `ZcodeError` enum covering shared failure modes |
| `ARCHITECTURE.md` | Detailed architecture doc with data flow |
| `README.md` | User-facing project overview |
| `USAGE.md` | Usage instructions |

## Workspace Layers
| Crate | Purpose |
|-------|---------|
| `crates/zcode_ui` | CLI screen/TUI rendering for current session conversation plus agent, skills, and MCP status |
| `crates/zcode_cli` | Clap argument parsing and command dispatch for the binary |
| `crates/zcode_requirements` | Requirement docs scaffold, validation, parsing, and task store; standardizes LLM prompt inputs |
| `crates/zcode_orchestration` | Agent graph workflows. Root orchestration coordinates planner, ReAct coder, reviewer, and self-learning behavior |
| `crates/zcode_llm_provider` | OpenAI-compatible chat completions provider using `ZCODE_BASE_URL`, `ZCODE_API_KEY`, `ZCODE_MODEL`, and `ZCODE_FAST_MODEL` |
| `crates/zcode_capabilities` | Skills, MCP, global shared context, and OpenAI-compatible tool-call abstractions |
| `crates/zcode_session` | Session message storage, history loading/deletion, and deterministic compression |
| `crates/zcode_core` | Shared config, errors, LLM DTOs, and agent/session DTOs |

## Other Directories
| Directory | Purpose |
|-----------|---------|
| `src/` | Root binary/library shell: `main.rs` plus workspace-crate re-exports |
| `tests/` | Integration tests |
| `docs/` | Design docs, PRDs, specs, plans |
| `templates/` | Document templates for PRD, spec, tasks, etc. |
| `examples/` | Example projects and generated app demos |

## For AI Agents

### Working In This Directory
- Build: `cargo build --workspace`
- Check: `cargo check --workspace`
- Test: `cargo test --workspace`
- Run: `cargo run -- <args>`
- This is a Cargo workspace; add new layer code to the crate that owns that responsibility.
- Keep `src/lib.rs` as an export shell; do not rebuild old root modules such as `src/agent`, `src/llm`, `src/tools`, `src/tui`, `src/docs`, `src/session`, `src/ast`, `src/git`, `src/lsp`, `src/memory`, `src/script`, or `src/workspace`.

### Testing Requirements
- Run `cargo test --workspace` before committing broad changes when feasible.
- Integration tests live in `tests/`; unit tests are inline in each crate/module.
- MSRV is 1.75; avoid features requiring newer Rust.

### Common Patterns
- All async traits use `async_trait`.
- Coder work uses the shared ReAct `AgentLoop`: reason, call available MCP/capability tools, observe, repeat, then report.
- Tool access is MCP/capability based. Built-in local file/shell/search/glob/AST tools were intentionally removed from runtime registration.
- LLM requests use OpenAI-compatible chat completions. Configure with `ZCODE_BASE_URL`, `ZCODE_API_KEY`, `ZCODE_MODEL`, and optional `ZCODE_FAST_MODEL`.
- Simple tasks may use the fast model through `ZCODE_FAST_MODEL` / `fast_model`.
- Configuration is shared through `Settings` and `ProjectConfig`.

## Dependencies

### External
- **tokio** — async runtime
- **clap** — CLI argument parsing
- **serde / serde_json** — serialization
- **reqwest** — HTTP client for OpenAI-compatible LLM APIs
- **ratatui + crossterm** — TUI framework
- **rusqlite** — SQLite for session snapshots

<!-- MANUAL: Custom project notes can be added below -->
