<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# config

## Purpose
Configuration management for both user-level settings and project-level configs. User settings live at `~/.config/zcode/settings.toml`; project config lives at `.zcode/config.toml`. All config types derive `Serialize`, `Deserialize`, and `JsonSchema`.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | `ProjectConfig` and all config sub-types: `McpServerConfig`, `LspServerConfig`, `ScriptConfig`, `HookConfig`, `SnapshotConfig`, `GrammarConfig`, `LlmConfigOverride`, `ToolConfigs` |
| `settings.rs` | `Settings` — global user-level settings (LLM provider, model, API key paths) |

## For AI Agents

### Working In This Directory
- All config types use `serde + schemars` for TOML serialization and JSON schema generation
- `ProjectConfig::load()` reads from `.zcode/config.toml`
- `ProjectConfig::save()` creates `.zcode/` directory if needed and writes atomically
- Default values are provided via `Default` implementations

### Testing Requirements
- Extensive inline tests covering roundtrip serialization, edge cases, defaults
- Uses `tempfile::TempDir` for file I/O tests

### Common Patterns
- `#[serde(default)]` for optional fields
- `fn bool_true()` and `fn default_*()` for serde default functions
- `schemars::JsonSchema` derive on all config structs

## Dependencies

### Internal
- `crate::error` — `ZcodeError::FileNotFound`, `ConfigLoadError`, etc.

### External
- `serde`, `serde_json`, `schemars`, `toml`, `config`, `directories`

<!-- MANUAL: -->
