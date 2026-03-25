# Coding Spec — zcode

## Tech Stack
- Language: Rust 2021
- Async runtime: tokio (full)
- CLI: clap 4 (derive macros)
- LLM: rig-core + provider plugins (Anthropic, OpenAI)
- TUI: ratatui + crossterm
- Error handling: thiserror + anyhow
- Serialisation: serde + serde_json
- Logging: tracing + tracing-subscriber

## File Structure
```
src/
├── main.rs                 # Entry point: parse args, run execute_command
├── lib.rs                  # Crate root, re-exports
├── error.rs                # ZcodeError enum, Result alias
├── config.rs               # Settings, ProjectConfig (TOML-based)
├── tools/                  # Tool registry + built-in tools
├── llm/                    # LlmProvider trait + RigProvider
├── agent/                  # Agent FSM, orchestrator, loop
├── cli/                    # CLI args + command handlers
│   ├── args.rs             # Args, Command, DocsAction
│   └── commands.rs         # execute_command, execute_docs, …
├── docs/                   # DocsValidator + generate_docs_scaffold
├── tui/                    # Terminal UI (ratatui)
├── memory/                 # Working, project, semantic memory
└── mcp/                    # MCP client
```

## Conventions
- All errors use `ZcodeError`; no raw `unwrap()` in production code
- Async code uses `tokio`; blocking I/O in tools uses `std::fs`
- Public API items carry `///` doc comments
- Tests live in `#[cfg(test)]` blocks alongside source
- New tools implement `ToolExecutor` and register in `ToolRegistry::new()`
