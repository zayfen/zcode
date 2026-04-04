<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# ast

## Purpose
Extensible AST parsing via Tree-sitter. Provides language-aware code analysis through a `LanguageRegistry` of `LanguageProvider` instances and a `GrammarRegistry` for file extension mapping. No grammars are bundled — register them at runtime.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | Module declarations and public re-exports |
| `language.rs` | `LanguageProvider` trait and `LanguageRegistry` — parses source into AST via Tree-sitter |
| `parser.rs` | `AstParser`, `AstTree`, `NodeInfo` — AST query and traversal types |
| `grammar.rs` | `GrammarRegistry` — maps file extensions to language names; supports runtime custom grammar loading (.so/.dylib) |

## For AI Agents

### Working In This Directory
- 17 built-in languages supported (Rust, Python, JS/TS, Go, C/C++, etc.)
- Custom grammars loaded via `GrammarConfig` in project config
- `LanguageProvider::parse()` returns an `AstTree` that can be queried

### Testing Requirements
- Tests in `grammar_integration.rs` in `/tests/`
- Inline tests in each file

## Dependencies

### Internal
- `crate::config` — GrammarConfig for custom grammars

### External
- `tree-sitter`, `libloading` (for dynamic grammar loading), `regex`

<!-- MANUAL: -->
