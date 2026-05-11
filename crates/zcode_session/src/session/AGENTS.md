<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-05-11 -->

# session

## Purpose
Session snapshot management backed by SQLite. Persists workspace state (file contents) to enable save/restore/diff of project snapshots. Used by the `Workspace` facade for checkpointing before major operations.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | Module declarations and public re-exports |
| `snapshot.rs` | `SnapshotManager` — SQLite-backed snapshot CRUD; `Snapshot`, `SnapshotDetail`, `FileSnapshot` types. Methods: `save_workspace()`, `restore()`, `list()`, `diff()` |

## For AI Agents

### Working In This Directory
- Database schema: `snapshots` table (id, name, description, timestamp) + `files` table (snapshot_id, relative_path, content)
- `SnapshotManager::new()` creates the SQLite DB if it doesn't exist
- `restore()` writes files back to disk, returns a map of (path → success)

### Testing Requirements
- Inline tests using `tempfile::TempDir`

## Dependencies

### Internal
- `zcode_core` — wraps IO, serialization, and SQLite errors

### External
- `rusqlite` (bundled), `serde`, `chrono`

<!-- MANUAL: -->
