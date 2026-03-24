//! Integration tests: Workspace facade
//!
//! Tests the full Workspace workflow across multiple modules:
//! config loading, snapshot save/restore, file context building.

use std::io::Write;
use tempfile::TempDir;
use zcode::workspace::{Workspace, WorkspaceContext};
use zcode::config::ProjectConfig;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn fresh_workspace() -> (TempDir, Workspace) {
    let dir = TempDir::new().unwrap();
    let ws = Workspace::init(dir.path(), "integration-test").unwrap();
    (dir, ws)
}

fn write(dir: &TempDir, name: &str, content: &str) {
    std::fs::write(dir.path().join(name), content).unwrap();
}

// ─── Config tests ────────────────────────────────────────────────────────────

#[test]
fn test_workspace_init_and_reload() {
    let dir = TempDir::new().unwrap();

    // Init
    let ws = Workspace::init(dir.path(), "reload-test").unwrap();
    assert_eq!(ws.config.name, "reload-test");

    // Re-open — should read config from disk
    let ws2 = Workspace::open(dir.path()).unwrap();
    assert_eq!(ws2.config.name, "reload-test");
}

#[test]
fn test_workspace_config_with_languages() {
    let dir = TempDir::new().unwrap();

    // Write config manually
    std::fs::create_dir_all(dir.path().join(".zcode")).unwrap();
    std::fs::write(
        dir.path().join(".zcode/config.toml"),
        r#"
name = "polyglot"
languages = ["rust", "python", "typescript"]
"#,
    ).unwrap();

    let ws = Workspace::open(dir.path()).unwrap();
    assert_eq!(ws.config.name, "polyglot");
    assert_eq!(ws.config.languages.len(), 3);
    assert!(ws.config.languages.contains(&"rust".to_string()));
}

#[test]
fn test_workspace_config_with_mcp_servers() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".zcode")).unwrap();
    std::fs::write(
        dir.path().join(".zcode/config.toml"),
        r#"
name = "mcp-project"

[[mcp_servers]]
name = "filesystem"
command = "mcp-server"
args = ["/workspace"]
auto_start = true
"#,
    ).unwrap();

    let ws = Workspace::open(dir.path()).unwrap();
    assert_eq!(ws.config.mcp_servers.len(), 1);
    assert_eq!(ws.config.mcp_servers[0].name, "filesystem");
    assert_eq!(ws.config.mcp_servers[0].command, "mcp-server");
    assert!(ws.config.mcp_servers[0].auto_start);
}

#[test]
fn test_workspace_config_with_lsp_servers() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".zcode")).unwrap();
    std::fs::write(
        dir.path().join(".zcode/config.toml"),
        r#"
name = "lsp-project"

[[lsp_servers]]
language = "rust"
command = "rust-analyzer"

[[lsp_servers]]
language = "python"
command = "pylsp"
"#,
    ).unwrap();

    let ws = Workspace::open(dir.path()).unwrap();
    assert_eq!(ws.config.lsp_servers.len(), 2);
    assert_eq!(ws.config.lsp_servers[0].language, "rust");
    assert_eq!(ws.config.lsp_servers[1].language, "python");
}

#[test]
fn test_workspace_config_with_custom_grammars() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".zcode")).unwrap();
    std::fs::write(
        dir.path().join(".zcode/config.toml"),
        r#"
name = "grammar-project"

[[grammars]]
language = "zig"
library_path = "/usr/lib/tree-sitter-zig.so"
extensions = ["zig"]
"#,
    ).unwrap();

    let ws = Workspace::open(dir.path()).unwrap();
    assert_eq!(ws.config.grammars.len(), 1);
    assert_eq!(ws.config.grammars[0].language, "zig");
    assert_eq!(ws.config.grammars[0].extensions, vec!["zig"]);
}

#[test]
fn test_workspace_config_with_script_hooks() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".zcode")).unwrap();
    std::fs::write(
        dir.path().join(".zcode/config.toml"),
        r#"
name = "hooks-project"

[scripts]
script_dirs = [".zcode/scripts"]
[scripts.hooks]
before_tool = ".zcode/scripts/before_tool.lua"
on_task_complete = ".zcode/scripts/notify.py"
"#,
    ).unwrap();

    let ws = Workspace::open(dir.path()).unwrap();
    assert_eq!(ws.config.scripts.script_dirs, vec![".zcode/scripts"]);
    assert_eq!(
        ws.config.scripts.hooks.before_tool.as_deref(),
        Some(".zcode/scripts/before_tool.lua")
    );
    assert_eq!(
        ws.config.scripts.hooks.on_task_complete.as_deref(),
        Some(".zcode/scripts/notify.py")
    );
}

// ─── Snapshot workflow ────────────────────────────────────────────────────────

#[test]
fn test_workspace_snapshot_full_workflow() {
    let (dir, mut ws) = fresh_workspace();

    // Write some files
    write(&dir, "main.rs", "fn main() { println!(\"v1\"); }");
    write(&dir, "lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }");

    // Save snapshot
    let id = ws.snapshot_save("v1", Some("Initial state")).unwrap();
    assert!(id > 0);

    // Modify files
    write(&dir, "main.rs", "fn main() { println!(\"v2 - modified\"); }");

    // Verify modification
    let content = std::fs::read_to_string(dir.path().join("main.rs")).unwrap();
    assert!(content.contains("v2 - modified"));

    // Restore snapshot
    let restored = ws.snapshot_restore(id).unwrap();
    assert!(restored > 0);

    // Verify restoration
    let content = std::fs::read_to_string(dir.path().join("main.rs")).unwrap();
    assert!(content.contains("v1"));
}

#[test]
fn test_workspace_multiple_snapshots() {
    let (dir, mut ws) = fresh_workspace();
    write(&dir, "data.txt", "version 1");

    let id1 = ws.snapshot_save("snap1", None).unwrap();
    write(&dir, "data.txt", "version 2");
    let id2 = ws.snapshot_save("snap2", None).unwrap();

    let list = ws.snapshot_list().unwrap();
    assert_eq!(list.len(), 2);

    // Restore id1 — should go back to version 1
    ws.snapshot_restore(id1).unwrap();
    let content = std::fs::read_to_string(dir.path().join("data.txt")).unwrap();
    assert_eq!(content, "version 1");

    // Restore id2 — should go back to version 2
    ws.snapshot_restore(id2).unwrap();
    let content = std::fs::read_to_string(dir.path().join("data.txt")).unwrap();
    assert_eq!(content, "version 2");
}

// ─── File context building ────────────────────────────────────────────────────

#[test]
fn test_workspace_build_file_context_basic() {
    let (dir, ws) = fresh_workspace();
    write(&dir, "README.md", "# My Project\n\nA great project.");

    let ctx = ws.build_file_context(&["README.md"], 100_000);
    assert_eq!(ctx.files.len(), 1);
    assert_eq!(ctx.files[0].0, "README.md");
    assert!(ctx.files[0].1.contains("My Project"));
}

#[test]
fn test_workspace_build_file_context_multiple_files() {
    let (dir, ws) = fresh_workspace();
    write(&dir, "a.rs", "fn alpha() {}");
    write(&dir, "b.rs", "fn beta() {}");

    let ctx = ws.build_file_context(&["a.rs", "b.rs"], 100_000);
    assert_eq!(ctx.files.len(), 2);
}

#[test]
fn test_workspace_build_file_context_skips_missing() {
    let (_, ws) = fresh_workspace();
    // "ghost.rs" does not exist
    let ctx = ws.build_file_context(&["ghost.rs"], 100_000);
    assert!(ctx.files.is_empty());
}

#[test]
fn test_workspace_context_prompt_contains_file_content() {
    let (dir, ws) = fresh_workspace();
    write(&dir, "tool.rs", "pub fn process() -> bool { true }");

    let ctx = ws.build_file_context(&["tool.rs"], 100_000);
    let prompt = ctx.as_prompt_context();
    assert!(prompt.contains("tool.rs"));
    assert!(prompt.contains("pub fn process"));
}

// ─── WorkspaceInfo ───────────────────────────────────────────────────────────

#[test]
fn test_workspace_info_structure() {
    let (_, ws) = fresh_workspace();
    let info = ws.info();
    assert_eq!(info.project_name, "integration-test");
    assert_eq!(info.root, ws.root());
}

#[test]
fn test_workspace_open_nonexistent_path_uses_dir_name() {
    let dir = TempDir::new().unwrap();
    // No config exists — should fall back to dir name
    let ws = Workspace::open(dir.path()).unwrap();
    let info = ws.info();
    assert!(!info.project_name.is_empty());
}
