<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-05-11 -->

# src

## Purpose
Root binary/library shell for the `zcode` package. Core behavior lives in workspace crates under `crates/`; `src/lib.rs` re-exports those crates while `src/main.rs` delegates CLI parsing and command execution to `zcode_cli`.

## Key Files
| File | Description |
|------|-------------|
| `lib.rs` | Export shell over `zcode_*` workspace crates |
| `main.rs` | Binary entry point: clap parse call, tracing init, command dispatch through `zcode_cli` |

## Moved Layers
| Old Area | New Crate |
|----------|-----------|
| CLI args/commands | `crates/zcode_cli` |
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
- Keep `src/lib.rs` focused on crate re-exports.
- Do not recreate removed old modules such as `src/cli`, `src/agent`, `src/llm`, `src/tools`, `src/tui`, `src/docs`, `src/session`, `src/config`, `src/ast`, `src/git`, `src/lsp`, `src/memory`, `src/script`, or `src/workspace`.
- Build with `cargo check --workspace`; test with `cargo test --workspace` or targeted package tests.

### Common Patterns
- Use `zcode_core::Result` / `ZcodeError` for shared errors.
- Use `zcode_capabilities::ToolRegistry` for MCP/capability tools.
- Use `zcode_llm_provider::RigProvider` for OpenAI-compatible chat completions.
- Coder execution should flow through `zcode_orchestration::AgentLoop` ReAct behavior.

<!-- MANUAL: -->
