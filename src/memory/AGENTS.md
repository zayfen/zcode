<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# memory

## Purpose
Three-tier memory architecture for managing LLM context: Working Memory (in-process, session-scoped), Project Memory (SQLite persistent), Semantic Index (TF-IDF vector search), and Context Assembler (token-budget-aware context builder).

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | Module declarations and public re-exports |
| `working.rs` | `WorkingMemory` — ephemeral in-process state with LRU file tracking, tool execution history, and `TokenUsage` tracking |
| `project.rs` | `ProjectMemory` — persisted to `.zcode/` directory as markdown files; `MemoryEntry`, `CodeChunk` types |
| `semantic.rs` | `SemanticIndex` — TF-IDF vector search for code similarity; `SearchResult` |
| `context.rs` | `ContextAssembler`, `AssembledContext`, `TokenBudget`, `estimate_tokens()` — assembles context within token budget limits |

## For AI Agents

### Working In This Directory
- `estimate_tokens()` uses a simple heuristic (~4 chars per token) — no external tokenizer
- `TokenBudget` defines how many tokens each context source gets
- `ContextAssembler` prioritizes: system prompt > recent files > diff > semantic results
- `ProjectMemory` stores architecture decisions and code chunks as markdown

### Testing Requirements
- Inline tests in each file
- Uses `tempfile::TempDir` for persistence tests

## Dependencies

### Internal
- `crate::error` — error types

### External
- `rusqlite` (bundled SQLite), `serde`, `walkdir`

<!-- MANUAL: -->
