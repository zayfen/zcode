<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-05-11 -->

# tui

## Purpose
Terminal user interface built with ratatui + crossterm. Provides a chat interface for interactive conversation with the LLM agent, including multi-line input (Shift+Enter, Alt+Enter, Ctrl+J), cursor movement, message history display, streaming responses, loading/thinking state, and pending prompt queueing.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | `TuiApp` — main application state, event handling, LLM integration; `init_terminal()`, `restore_terminal()` — terminal setup/teardown; keyboard enhancement (kitty protocol) |
| `chat.rs` | `ChatInterface`, `ChatMessage` — chat UI rendering, input buffer management, message display |

## For AI Agents

### Working In This Directory
- Terminal lifecycle: `init_terminal()` → `TuiApp::run()` → `restore_terminal()`
- Event handling: Ctrl+C/Esc to quit, Enter to queue/send, Ctrl+O toggles full thinking, Shift/Alt/Ctrl+Enter for newline, characters for typing
- LLM calls stream through a worker thread; the UI keeps rendering loading animation, collapsed thinking text, assistant deltas, and queued prompt count
- Supports kitty keyboard enhancement protocol for better key disambiguation

### Testing Requirements
- Extensive inline tests for event handling (keyboard events, edge cases)

## Dependencies

### Internal
- `zcode_llm_provider` — `LlmProvider`, `Message`, `MessageRole`
- `zcode_core` — wraps crossterm/ratatui errors

### External
- `ratatui`, `crossterm`, `futures`, `textwrap`

<!-- MANUAL: -->
