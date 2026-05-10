<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# mcp

## Purpose
Model Context Protocol client implementing JSON-RPC 2.0 over stdio. Manages MCP server subprocesses, discovers remote tools via `tools/list`, and wraps them as local `Tool` trait objects via `McpToolAdapter`.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | Module declarations and public re-exports |
| `types.rs` | `McpError`, `McpRequest`, `McpResponse`, `McpServerConfig`, `McpTool`, `McpTransport` |
| `client.rs` | `McpClient` — subprocess management, JSON-RPC communication; `McpToolAdapter` — wraps remote tools as local `Tool` |

## For AI Agents

### Working In This Directory
- MCP servers are launched as stdio subprocesses
- `tools/list` discovers available tools; `tools/call` executes them
- `McpToolAdapter` implements the `Tool` trait so MCP tools work with `ToolRegistry`

### Testing Requirements
- Inline tests

## Dependencies

### Internal
- `crate::tools` — `Tool` trait for `McpToolAdapter`
- `crate::error` — error types

### External
- `serde`, `serde_json`, `tokio`, `reqwest`

<!-- MANUAL: -->
