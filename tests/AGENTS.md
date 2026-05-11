<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# tests

## Purpose
Integration tests for the zcode crate. These test cross-module interactions that go beyond what unit tests (inline in each source file) can cover.

## Key Files
| File | Description |
|------|-------------|
| `cli_test.rs` | CLI argument parsing and command dispatch tests |
| `registry_test.rs` | Tool registry creation, registration, and execution |
| `reviewer_integration.rs` | ReviewerAgent static analysis across all 5 categories |

## For AI Agents

### Working In This Directory
- Run all: `cargo test` from project root
- Run single file: `cargo test --test cli_test`

### Testing Requirements
- Integration tests must not depend on external APIs (no real LLM calls)
- Use mock providers for agent tests
- All tests should pass on macOS and Linux

<!-- MANUAL: -->
