<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# cli

## Purpose
Command-line interface using clap (derive macro). Defines argument parsing (`Args`, `Command`) and command execution handlers.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | Module declarations and public re-exports |
| `args.rs` | `Args` struct with clap derive — defines all CLI flags and subcommands |
| `commands.rs` | `execute_command()` and `execute_default()` — dispatch CLI commands to the appropriate handler |

## For AI Agents

### Working In This Directory
- Add new CLI flags in `args.rs` using clap derive macros
- Add new command handlers in `commands.rs`
- Default behavior (no subcommand) launches the TUI chat interface

### Testing Requirements
- Tests in `/tests/cli_test.rs`

## Dependencies

### Internal
- `crate::tui` — for launching the chat interface
- `crate::llm` — for LLM client initialization
- `crate::workspace` — for workspace operations

### External
- `clap` (derive, env features)

<!-- MANUAL: -->
