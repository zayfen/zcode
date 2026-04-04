<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# script

## Purpose
Multi-language scripting engine supporting Lua, Python, JavaScript, and Shell. All engines implement a unified `ScriptEngine` trait and inject a `zcode` global API (`read_file`, `write_file`, `shell`, `log`). The `ScriptManager` scans directories, converts scripts to `ScriptTool` instances, and registers them in the `ToolRegistry`.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | Module declarations, re-exports, and `default_script_manager()` factory |
| `engine.rs` | `ScriptEngine` trait, `ScriptContext`, `ScriptOutput` — the interface all engines implement |
| `hooks.rs` | `HookRegistry`, `HookType`, `HookContext` — lifecycle hooks (before_tool, after_tool, on_task_start, on_task_complete) |
| `lua.rs` | `LuaEngine` — mlua (Lua 5.4, vendored) |
| `python.rs` | `PythonEngine` — pyo3 (uses system Python) |
| `javascript.rs` | `JsEngine` — rquickjs (QuickJS, vendored) |
| `shell.rs` | `ShellEngine` — plain shell script execution via subprocess |
| `manager.rs` | `ScriptManager` — scans directories, registers scripts as tools; `ScriptTool` wraps a script file as a `Tool` |

## For AI Agents

### Working In This Directory
- To add a new engine, implement `ScriptEngine` and register in `default_script_manager()`
- Each engine injects `zcode.read_file()`, `zcode.write_file()`, `zcode.shell()`, `zcode.log()` into the script's global scope
- `ScriptManager::scan_directory()` recursively finds script files and registers them as tools

### Testing Requirements
- Tests in `/tests/scripting_integration.rs`
- Inline tests in each engine file

## Dependencies

### Internal
- `crate::tools` — `Tool` trait, `ToolRegistry`
- `crate::error` — `ZcodeError::ScriptError`

### External
- `mlua` (lua54, vendored), `pyo3` (auto-initialize), `rquickjs` (full features), `tokio`

<!-- MANUAL: -->
