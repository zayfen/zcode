<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# lsp

## Purpose
Language Server Protocol client communicating over stdio transport with Content-Length header framing. Supports initialize, textDocument/didOpen, definition, references, hover, and completion.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | Module declarations and public re-exports |
| `client.rs` | `LspClient` — stdio transport, request/response handling, and `HoverResult` type |

## For AI Agents

### Working In This Directory
- Language is auto-detected from file extension
- Client launches LSP server as a subprocess via the command in `LspServerConfig`
- Uses Content-Length framing (JSON-RPC 2.0)

### Testing Requirements
- Inline tests in `client.rs`

## Dependencies

### Internal
- `crate::config` — `LspServerConfig`
- `crate::error` — wraps IO errors

### External
- `lsp-types`, `serde_json`, `tokio`

<!-- MANUAL: -->
