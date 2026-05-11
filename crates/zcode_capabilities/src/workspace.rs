//! Local workspace tools for agentic coding tasks.
//!
//! These tools are intentionally scoped to a single project root. They provide
//! the minimum file, search, and shell capabilities needed for coder/reviewer
//! agents to make and verify workspace changes when no MCP filesystem server is
//! configured.

use crate::{Tool, ToolRegistry, ToolResult};
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use zcode_core::{Result, ZcodeError};

const DEFAULT_READ_BYTES: usize = 200_000;
const DEFAULT_GLOB_LIMIT: usize = 200;
const DEFAULT_COMMAND_TIMEOUT_SECS: u64 = 120;
const MAX_COMMAND_TIMEOUT_SECS: u64 = 600;
const MAX_OUTPUT_CHARS: usize = 40_000;

/// Register the default local workspace tools.
pub fn register_workspace_tools(
    registry: &mut ToolRegistry,
    root: impl Into<PathBuf>,
) -> Result<()> {
    let workspace = Workspace::new(root.into())?;
    registry.register(GlobTool::new(workspace.clone()));
    registry.register(ReadFileTool::new("read_file", workspace.clone()));
    registry.register(ReadFileTool::new("file_read", workspace.clone()));
    registry.register(WriteFileTool::new(workspace.clone()));
    registry.register(EditFileTool::new(workspace.clone()));
    registry.register(ShellTool::new(workspace));
    Ok(())
}

#[derive(Clone)]
struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new(root: PathBuf) -> Result<Self> {
        let root = if root.exists() {
            root.canonicalize().map_err(|e| {
                ZcodeError::ConfigError(format!("Cannot canonicalize workspace root: {}", e))
            })?
        } else {
            std::fs::create_dir_all(&root).map_err(|e| {
                ZcodeError::ConfigError(format!("Cannot create workspace root: {}", e))
            })?;
            root.canonicalize().map_err(|e| {
                ZcodeError::ConfigError(format!("Cannot canonicalize workspace root: {}", e))
            })?
        };
        Ok(Self { root })
    }

    fn resolve_existing(&self, raw: &str) -> Result<PathBuf> {
        let candidate = self.resolve_lexical(raw)?;
        let canonical = candidate.canonicalize().map_err(|e| {
            ZcodeError::ConfigError(format!("Path does not exist or cannot be read: {}", e))
        })?;
        if !canonical.starts_with(&self.root) {
            return Err(ZcodeError::ConfigError(format!(
                "Path is outside workspace root: {}",
                raw
            )));
        }
        Ok(canonical)
    }

    fn resolve_for_write(&self, raw: &str) -> Result<PathBuf> {
        let candidate = self.resolve_lexical(raw)?;
        let parent = candidate.parent().unwrap_or(&self.root);
        std::fs::create_dir_all(parent).map_err(|e| {
            ZcodeError::ConfigError(format!("Cannot create parent directory: {}", e))
        })?;
        let canonical_parent = parent.canonicalize().map_err(|e| {
            ZcodeError::ConfigError(format!("Cannot canonicalize parent directory: {}", e))
        })?;
        if !canonical_parent.starts_with(&self.root) {
            return Err(ZcodeError::ConfigError(format!(
                "Path is outside workspace root: {}",
                raw
            )));
        }
        Ok(candidate)
    }

    fn resolve_lexical(&self, raw: &str) -> Result<PathBuf> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Ok(self.root.clone());
        }

        let input = Path::new(raw);
        let candidate = if input.is_absolute() {
            input.to_path_buf()
        } else {
            self.root.join(input)
        };
        let normalized = normalize_path(&candidate);
        if !normalized.starts_with(&self.root) {
            return Err(ZcodeError::ConfigError(format!(
                "Path is outside workspace root: {}",
                raw
            )));
        }
        Ok(normalized)
    }

    fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

struct GlobTool {
    workspace: Workspace,
}

impl GlobTool {
    fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
}

impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "List files in the workspace matching a glob-like pattern such as **/*.rs or src/*.ts."
    }

    fn input_schema(&self) -> Value {
        json!({
            "name": self.name(),
            "description": self.description(),
            "input_schema": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob-like pattern. Supports * and **."
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional directory to search from, relative to the workspace root."
                    },
                    "max_files": {
                        "type": "integer",
                        "description": "Maximum number of files to return."
                    }
                },
                "required": ["pattern"]
            }
        })
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("**/*");
        let base_raw = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let max_files = input
            .get("max_files")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_GLOB_LIMIT as u64) as usize;
        let max_files = max_files.max(1);

        let base = self.workspace.resolve_existing(base_raw)?;
        if !base.is_dir() {
            return Err(ZcodeError::ConfigError(format!(
                "glob path is not a directory: {}",
                base_raw
            )));
        }

        let mut files = Vec::new();
        collect_matching_files(&self.workspace, &base, pattern, max_files, &mut files)?;
        let truncated = files.len() >= max_files;
        Ok(json!({
            "files": files,
            "truncated": truncated,
            "root": self.workspace.root.display().to_string()
        }))
    }
}

struct ReadFileTool {
    name: &'static str,
    workspace: Workspace,
}

impl ReadFileTool {
    fn new(name: &'static str, workspace: Workspace) -> Self {
        Self { name, workspace }
    }
}

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "Read a UTF-8 text file from the workspace."
    }

    fn input_schema(&self) -> Value {
        json!({
            "name": self.name(),
            "description": self.description(),
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path relative to the workspace root."
                    },
                    "max_bytes": {
                        "type": "integer",
                        "description": "Maximum bytes to return."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Optional maximum number of lines to return."
                    }
                },
                "required": ["path"]
            }
        })
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let path_raw = required_str(&input, "path")?;
        let path = self.workspace.resolve_existing(path_raw)?;
        if !path.is_file() {
            return Err(ZcodeError::ConfigError(format!(
                "read_file path is not a file: {}",
                path_raw
            )));
        }

        let max_bytes = input
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_READ_BYTES as u64) as usize;
        let mut content = std::fs::read_to_string(&path).map_err(|e| {
            ZcodeError::ConfigError(format!("Cannot read file {}: {}", path_raw, e))
        })?;
        let mut truncated = false;
        if let Some(limit) = input.get("limit").and_then(|v| v.as_u64()) {
            let lines: Vec<&str> = content.lines().take(limit as usize).collect();
            truncated = content.lines().count() > lines.len();
            content = lines.join("\n");
        }
        if content.len() > max_bytes {
            let end = floor_char_boundary(&content, max_bytes);
            content.truncate(end);
            truncated = true;
        }

        Ok(json!({
            "path": self.workspace.relative(&path),
            "content": content,
            "truncated": truncated
        }))
    }
}

struct WriteFileTool {
    workspace: Workspace,
}

impl WriteFileTool {
    fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
}

impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Create or overwrite a UTF-8 text file inside the workspace."
    }

    fn input_schema(&self) -> Value {
        json!({
            "name": self.name(),
            "description": self.description(),
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path relative to the workspace root."
                    },
                    "content": {
                        "type": "string",
                        "description": "Complete file content to write."
                    }
                },
                "required": ["path", "content"]
            }
        })
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let path_raw = required_str(&input, "path")?;
        let content = required_str(&input, "content")?;
        let path = self.workspace.resolve_for_write(path_raw)?;
        std::fs::write(&path, content).map_err(|e| {
            ZcodeError::ConfigError(format!("Cannot write file {}: {}", path_raw, e))
        })?;
        Ok(json!({
            "path": self.workspace.relative(&path),
            "bytes": content.len()
        }))
    }
}

struct EditFileTool {
    workspace: Workspace,
}

impl EditFileTool {
    fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
}

impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Replace text in a UTF-8 workspace file."
    }

    fn input_schema(&self) -> Value {
        json!({
            "name": self.name(),
            "description": self.description(),
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "old": {
                        "type": "string",
                        "description": "Text to replace. Also accepts old_string or search."
                    },
                    "new": {
                        "type": "string",
                        "description": "Replacement text. Also accepts new_string or replace."
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "Replace all matches instead of the first match."
                    }
                },
                "required": ["path"]
            }
        })
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let path_raw = required_str(&input, "path")?;
        let old = input
            .get("old")
            .or_else(|| input.get("old_string"))
            .or_else(|| input.get("search"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ZcodeError::ConfigError("edit_file requires old text".to_string()))?;
        let new = input
            .get("new")
            .or_else(|| input.get("new_string"))
            .or_else(|| input.get("replace"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ZcodeError::ConfigError("edit_file requires new text".to_string()))?;
        if old.is_empty() {
            return Err(ZcodeError::ConfigError(
                "edit_file old text cannot be empty".to_string(),
            ));
        }

        let path = self.workspace.resolve_existing(path_raw)?;
        let content = std::fs::read_to_string(&path).map_err(|e| {
            ZcodeError::ConfigError(format!("Cannot read file {}: {}", path_raw, e))
        })?;
        if !content.contains(old) {
            return Err(ZcodeError::ConfigError(format!(
                "edit_file could not find requested text in {}",
                path_raw
            )));
        }

        let replace_all = input
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let updated = if replace_all {
            content.replace(old, new)
        } else {
            content.replacen(old, new, 1)
        };
        std::fs::write(&path, &updated).map_err(|e| {
            ZcodeError::ConfigError(format!("Cannot write file {}: {}", path_raw, e))
        })?;

        Ok(json!({
            "path": self.workspace.relative(&path),
            "replacements": if replace_all { content.matches(old).count() } else { 1 }
        }))
    }
}

struct ShellTool {
    workspace: Workspace,
}

impl ShellTool {
    fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
}

impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Run a shell command in the workspace root and return stdout, stderr, and exit status."
    }

    fn input_schema(&self) -> Value {
        json!({
            "name": self.name(),
            "description": self.description(),
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to run."
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Optional working directory relative to the workspace root."
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Timeout in seconds."
                    }
                },
                "required": ["command"]
            }
        })
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let command = required_str(&input, "command")?;
        let cwd_raw = input.get("cwd").and_then(|v| v.as_str()).unwrap_or(".");
        let cwd = self.workspace.resolve_existing(cwd_raw)?;
        if !cwd.is_dir() {
            return Err(ZcodeError::ConfigError(format!(
                "shell cwd is not a directory: {}",
                cwd_raw
            )));
        }

        let timeout_secs = input
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_COMMAND_TIMEOUT_SECS)
            .clamp(1, MAX_COMMAND_TIMEOUT_SECS);

        let mut child = Command::new("sh")
            .arg("-lc")
            .arg(command)
            .current_dir(&cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ZcodeError::ConfigError(format!("Cannot run shell command: {}", e)))?;

        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        let mut timed_out = false;
        loop {
            if child
                .try_wait()
                .map_err(|e| ZcodeError::ConfigError(format!("Cannot poll command: {}", e)))?
                .is_some()
            {
                break;
            }
            if Instant::now() >= deadline {
                timed_out = true;
                let _ = child.kill();
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        let output = child.wait_with_output().map_err(|e| {
            ZcodeError::ConfigError(format!("Cannot collect command output: {}", e))
        })?;

        Ok(json!({
            "command": command,
            "cwd": self.workspace.relative(&cwd),
            "status": output.status.code(),
            "success": output.status.success() && !timed_out,
            "timed_out": timed_out,
            "stdout": truncate_output(&String::from_utf8_lossy(&output.stdout)),
            "stderr": truncate_output(&String::from_utf8_lossy(&output.stderr))
        }))
    }
}

fn required_str<'a>(input: &'a Value, key: &str) -> Result<&'a str> {
    input
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ZcodeError::ConfigError(format!("Tool input requires string `{}`", key)))
}

fn collect_matching_files(
    workspace: &Workspace,
    dir: &Path,
    pattern: &str,
    max_files: usize,
    files: &mut Vec<String>,
) -> Result<()> {
    if files.len() >= max_files {
        return Ok(());
    }

    let entries = std::fs::read_dir(dir)
        .map_err(|e| ZcodeError::ConfigError(format!("Cannot read directory: {}", e)))?;
    for entry in entries.flatten() {
        if files.len() >= max_files {
            break;
        }
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if path.is_dir() {
            if should_skip_dir(&file_name) {
                continue;
            }
            collect_matching_files(workspace, &path, pattern, max_files, files)?;
        } else if path.is_file() {
            let rel = workspace.relative(&path);
            if matches_pattern(pattern, &rel) {
                files.push(rel);
            }
        }
    }
    Ok(())
}

fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git" | "target" | "node_modules" | ".zcode" | "dist" | "build" | ".next"
    )
}

fn matches_pattern(pattern: &str, text: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() || pattern == "**/*" || pattern == "*" {
        return true;
    }
    if let Some(rest) = pattern.strip_prefix("**/") {
        return matches_pattern(rest, text) || wildcard_match(pattern, text);
    }
    wildcard_match(pattern, text)
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == text || text.ends_with(pattern);
    }

    let mut remainder = text;
    let mut first = true;
    for part in pattern.split('*').filter(|part| !part.is_empty()) {
        if first && !pattern.starts_with('*') {
            let Some(stripped) = remainder.strip_prefix(part) else {
                return false;
            };
            remainder = stripped;
        } else {
            let Some(index) = remainder.find(part) else {
                return false;
            };
            remainder = &remainder[index + part.len()..];
        }
        first = false;
    }

    if !pattern.ends_with('*') {
        if let Some(last) = pattern.rsplit('*').find(|part| !part.is_empty()) {
            return text.ends_with(last);
        }
    }
    true
}

fn truncate_output(text: &str) -> String {
    if text.len() <= MAX_OUTPUT_CHARS {
        return text.to_string();
    }
    let end = floor_char_boundary(text, MAX_OUTPUT_CHARS);
    format!(
        "{}... (truncated, total {} chars)",
        &text[..end],
        text.len()
    )
}

fn floor_char_boundary(text: &str, index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    let mut end = index;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn registry(root: &Path) -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        register_workspace_tools(&mut registry, root).unwrap();
        registry
    }

    #[test]
    fn test_workspace_read_write_edit() {
        let dir = TempDir::new().unwrap();
        let registry = registry(dir.path());

        registry
            .execute(
                "write_file",
                json!({"path": "src/main.ts", "content": "const value = 1;\n"}),
            )
            .unwrap();
        let read = registry
            .execute("read_file", json!({"path": "src/main.ts"}))
            .unwrap();
        assert_eq!(read["content"], "const value = 1;\n");

        registry
            .execute(
                "edit_file",
                json!({"path": "src/main.ts", "old": "1", "new": "2"}),
            )
            .unwrap();
        let read = registry
            .execute("file_read", json!({"path": "src/main.ts"}))
            .unwrap();
        assert_eq!(read["content"], "const value = 2;\n");
    }

    #[test]
    fn test_workspace_rejects_parent_escape() {
        let dir = TempDir::new().unwrap();
        let registry = registry(dir.path());
        let result = registry.execute(
            "write_file",
            json!({"path": "../outside.txt", "content": "no"}),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_glob_matches_files() {
        let dir = TempDir::new().unwrap();
        let registry = registry(dir.path());
        registry
            .execute("write_file", json!({"path": "src/main.rs", "content": ""}))
            .unwrap();
        registry
            .execute("write_file", json!({"path": "README.md", "content": ""}))
            .unwrap();

        let result = registry
            .execute("glob", json!({"pattern": "**/*.rs"}))
            .unwrap();
        let files = result["files"].as_array().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], "src/main.rs");
    }

    #[test]
    fn test_shell_runs_in_workspace() {
        let dir = TempDir::new().unwrap();
        let registry = registry(dir.path());
        let result = registry
            .execute("shell", json!({"command": "pwd", "timeout_secs": 5}))
            .unwrap();
        assert!(result["success"].as_bool().unwrap());
        assert!(result["stdout"]
            .as_str()
            .unwrap()
            .contains(dir.path().to_str().unwrap()));
    }
}
