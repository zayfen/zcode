<!-- Parent: ../../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-05-11 -->

# zcode_cli

## Purpose
Command-line interface layer using clap derive macros. Defines argument parsing (`Args`, `Command`) and command execution handlers that wire the binary into requirements, capabilities, orchestration, LLM provider, and UI layers.

## Key Files
| File | Description |
|------|-------------|
| `src/lib.rs` | Module declarations and public re-exports |
| `src/args.rs` | `Args` struct with clap derive; defines all CLI flags and subcommands |
| `src/commands.rs` | `execute_command()` and `execute_default()`; dispatch CLI commands to handlers |

## For AI Agents

### Working In This Directory
- Add new CLI flags in `args.rs` using clap derive macros
- Add new command handlers in `commands.rs`
- Default behavior (no subcommand) launches the TUI chat interface

### Testing Requirements
- Tests in `/tests/cli_test.rs`

## Dependencies

### Internal
- `zcode_ui` — for launching the chat interface
- `zcode_llm_provider` — for LLM provider initialization
- `zcode_core` — for settings and project config
- `zcode_requirements` — for docs validation and task storage
- `zcode_orchestration` — for agent graph execution
- `zcode_capabilities` — for skills and MCP tools

### External
- `clap` (derive, env features)

<!-- MANUAL: -->
