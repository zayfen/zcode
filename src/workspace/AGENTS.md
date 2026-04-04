<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# workspace

## Purpose
Top-level integration facade that wires together all zcode capabilities: project config, git diff, session snapshots, and LLM context assembly. This is the primary API for external consumers of the zcode library.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | `Workspace` — open/init, `build_diff_context()`, `build_file_context()`, snapshot helpers; `WorkspaceInfo` — status info; `WorkspaceContext` — LLM-ready context with files, diff, commits |

## For AI Agents

### Working In This Directory
- `Workspace::open()` loads config from `.zcode/config.toml` (falls back to defaults)
- `Workspace::init()` creates the `.zcode/` directory and config file
- `build_diff_context()` loads only git-changed files within a character budget
- `build_file_context()` loads specific files within a budget
- `WorkspaceContext::as_prompt_context()` formats everything as markdown for LLM injection

### Testing Requirements
- Extensive inline tests using `tempfile::TempDir`

### Common Patterns
- `Workspace` owns a `SnapshotManager` (Option, lazily initialized from config)
- `WorkspaceContext` is a plain data struct suitable for serialization

## Dependencies

### Internal
- `crate::config` — `ProjectConfig`
- `crate::git` — `GitDiff` for diff operations
- `crate::session` — `SnapshotManager` for checkpointing

### External
- `serde`, `serde_json`

<!-- MANUAL: -->
