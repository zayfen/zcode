<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-05-11 -->

# src

## Purpose
Application shell and compatibility modules for the `zcode` package. Core behavior now lives in workspace crates under `crates/`; `src/lib.rs` re-exports those crates for older imports while `src/main.rs` and `src/cli/` keep the binary entry point.

## Key Files
| File | Description |
|------|-------------|
| `lib.rs` | Compatibility/export shell over `zcode_*` workspace crates |
| `main.rs` | CLI entry point: clap arg parsing, tracing init, command dispatch |
| `cli/args.rs` | Clap argument definitions |
| `cli/commands.rs` | Command handlers and integration wiring between layers |

## Retained Subdirectories
| Directory | Purpose |
|-----------|---------|
| `ast/` | Tree-sitter language and grammar registry compatibility code |
| `cli/` | CLI argument parsing and command handlers |
| `git/` | Git integration via subprocess |
| `lsp/` | Language Server Protocol client |
| `memory/` | Retained memory/context helpers |
| `script/` | Retained multi-language scripting engines and hook registry |
| `workspace/` | Facade for project config, snapshots, and compatibility workflows |

## Moved Layers
| Old Area | New Crate |
|----------|-----------|
| UI/TUI | `crates/zcode_ui` |
| Docs/task store | `crates/zcode_requirements` |
| Agent orchestration | `crates/zcode_orchestration` |
| LLM provider | `crates/zcode_llm_provider` |
| Tools/MCP/skills/shared prompt | `crates/zcode_capabilities` |
| Session snapshots/messages | `crates/zcode_session` |
| Errors/config/shared DTOs | `crates/zcode_core` |

## For AI Agents

### Working In This Directory
- Prefer editing the owning workspace crate in `crates/` instead of adding new logic to `src/`.
- Keep `src/lib.rs` focused on re-exports and compatibility.
- Do not recreate removed old modules such as `src/agent`, `src/llm`, `src/tools`, `src/tui`, `src/docs`, `src/session`, or `src/config`.
- Build with `cargo check --workspace`; test with `cargo test --workspace` or targeted package tests.

### Common Patterns
- Use `zcode_core::Result` / `ZcodeError` for shared errors.
- Use `zcode_capabilities::ToolRegistry` for MCP/capability tools.
- Use `zcode_llm_provider::RigProvider` for OpenAI-compatible chat completions.
- Coder execution should flow through `zcode_orchestration::AgentLoop` ReAct behavior.

<!-- MANUAL: -->
