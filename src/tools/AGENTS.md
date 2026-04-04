<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# tools

## Purpose
Tool system providing the `Tool` trait, `ToolRegistry`, and built-in tools for file I/O, shell execution, code search, glob patterns, and AST queries. All tools implement `fn execute(&self, input: Value) -> ToolResult<Value>` and can be registered dynamically.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | `Tool` trait, `ToolRegistry`, `register_default_tools()`, `ToolResult` type alias |
| `file.rs` | `FileReadTool`, `FileWriteTool`, `FileEditTool` — file system operations |
| `shell.rs` | `ShellTool` — subprocess execution |
| `search.rs` | `SearchTool` — ripgrep-style code search |
| `glob.rs` | `GlobTool` — file pattern matching |
| `ast_tools.rs` | `AstSearchTool`, `AstEditTool` — Tree-sitter based code analysis (requires `LanguageRegistry`) |

## For AI Agents

### Working In This Directory
- To add a new tool: implement `Tool` trait, then register via `registry.register(MyTool)`
- Each tool can override `anthropic_schema()` to provide a custom parameter schema for LLM function calling
- Built-in tools registered by `register_default_tools()`: `file_read`, `file_write`, `file_edit`, `search`, `shell`, `glob`
- AST tools require a `LanguageRegistry` and are registered separately

### Testing Requirements
- Inline tests with mock tools (`TestTool`, `EchoTool`, `FailingTool`, `JsonTool`)
- Thread safety tests with `Arc<ToolRegistry>`

### Common Patterns
- `Arc<dyn Tool>` for shared ownership
- `HashMap<String, Arc<dyn Tool>>` for the registry
- `serde_json::Value` as the universal input/output type

## Dependencies

### Internal
- `crate::error` — `ZcodeError::ToolNotFound`, `ToolExecutionFailed`, `InvalidToolInput`
- `crate::ast` — `LanguageRegistry` for AST tools

### External
- `serde_json`, `glob`, `regex`, `walkdir`, `tree-sitter`

<!-- MANUAL: -->
