<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# tests

## Purpose
Integration tests for the zcode crate. These test cross-module interactions that go beyond what unit tests (inline in each source file) can cover.

## Key Files
| File | Description |
|------|-------------|
| `cli_test.rs` | CLI argument parsing and command dispatch tests |
| `grammar_integration.rs` | Tree-sitter grammar loading and parsing across languages |
| `registry_test.rs` | Tool registry creation, registration, and execution |
| `reviewer_integration.rs` | ReviewerAgent static analysis across all 5 categories |
| `scripting_integration.rs` | Multi-language scripting engine tests (Lua, Python, JS, Shell) |
| `workspace_integration.rs` | Workspace facade tests (open, init, context building, snapshots) |

## For AI Agents

### Working In This Directory
- Run all: `cargo test` from project root
- Run single file: `cargo test --test cli_test`
- All tests use `tempfile::TempDir` for isolated filesystem operations
- Some tests require system Python (pyo3) and may be skipped if unavailable

### Testing Requirements
- Integration tests must not depend on external APIs (no real LLM calls)
- Use mock providers for agent tests
- All tests should pass on macOS and Linux

<!-- MANUAL: -->
