<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# llm

## Purpose
LLM provider integration with support for Anthropic (Claude), OpenAI (GPT), and Ollama (local models). Provides streaming SSE responses, tool call parsing, and a unified `LlmProvider` trait. Also contains `LlmConfig`, `Message`, `LlmResponse`, and `LlmClient` facade.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | Core types: `LlmConfig`, `Message`, `MessageRole`, `LlmResponse`, `UsageStats`, `LlmClient` facade |
| `provider.rs` | `LlmProvider` trait and `RigProvider` — HTTP client for Anthropic/OpenAI/Ollama APIs |
| `streaming.rs` | `StreamHandler` and `StreamingResponse` — SSE stream parsing for real-time LLM output |
| `tool_call.rs` | `ToolSchema`, `ToolCallRequest`, `ToolCallResponse`, `generate_tool_schemas()` — Anthropic/OpenAI function calling |

## For AI Agents

### Working In This Directory
- Provider is selected via `LlmConfig.provider` string ("anthropic", "openai", "ollama")
- API keys resolved from env vars: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, or `ANTHROPIC_AUTH_TOKEN`
- Tool calls follow Anthropic's format — `tool_call.rs` handles parsing
- `RigProvider::chat()` is the main entry point for conversations with tool support
- Default model: `claude-3-5-sonnet-20241022`

### Testing Requirements
- Extensive inline tests for config, messages, responses, and client
- LLM API calls are not tested in CI (require API keys)

### Common Patterns
- `Arc<dyn LlmProvider>` for shared provider access
- `Message::system()`, `Message::user()`, `Message::assistant()` constructors
- `ToolSchema` converts `Tool` trait objects to LLM-compatible function definitions

## Dependencies

### Internal
- `crate::error` — `ZcodeError::LlmApiError`, `LlmResponseError`, `MissingApiKey`
- `crate::tools` — Tool trait for schema generation

### External
- `reqwest` (json, stream, blocking), `async-trait`, `tokio`, `serde_json`

<!-- MANUAL: -->
