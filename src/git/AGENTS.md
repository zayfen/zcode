<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# git

## Purpose
Git integration via subprocess (no libgit2). Provides diff analysis, changed file detection, repository root resolution, and a `DiffContext` builder for assembling git-aware LLM context.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | Module declarations and public re-exports |
| `diff.rs` | `GitDiff` struct with methods: `is_git_repo()`, `repo_root()`, `changed_files()`, `full_diff()`, `recent_commits()`, `build_context()`. Also `ChangedFile`, `FileStatus`, `DiffContext` |

## For AI Agents

### Working In This Directory
- Uses `git` CLI via `std::process::Command` — no libgit2 dependency
- `DiffContext::load_changed_contents()` lazily loads only changed files
- `FileStatus` enum: `Added`, `Modified`, `Deleted`, `Renamed`, `Untracked`

### Testing Requirements
- Inline tests (requires a git repo for full coverage)

## Dependencies

### Internal
- `crate::error` — wraps IO errors

### External
- `serde`, `regex`, `similar` (diff algorithm)

<!-- MANUAL: -->
