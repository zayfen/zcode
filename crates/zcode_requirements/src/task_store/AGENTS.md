<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-05-11 -->

# task_store

## Purpose
Task progress persistence for `zcode run` invocations. Stores each task as a JSON file in `.zcode/tasks/` with full conversation history so execution can be resumed exactly where it left off.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | `TaskStore` — CRUD operations on task records; `TaskRecord` — id, task description, status, iteration, conversation history, result/error; `TaskStatus` enum (Running, Completed, Failed, Interrupted) |

## For AI Agents

### Working In This Directory
- Task IDs are 8-character hex strings derived from timestamp + atomic counter
- `save()` is atomic: writes to `.tmp` then renames
- `clean()` removes all completed/failed/interrupted tasks
- `list()` returns tasks sorted newest-first by `created_at`

### Testing Requirements
- Extensive inline tests using `tempfile::TempDir`

## Dependencies

### Internal
- `zcode_core::agent` — shared task, state, and conversation DTOs
- `zcode_core` — wraps IO and serialization errors

### External
- `serde`, `serde_json`, `uuid`

<!-- MANUAL: -->
