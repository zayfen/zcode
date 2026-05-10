//! Shell execution tool for zcode
//!
//! This module provides safe shell command execution using std::process::Command.

use crate::error::ZcodeError;
use crate::tools::{Tool, ToolResult};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Command;

// ─── ShellTool ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ShellInput {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    /// Timeout in milliseconds (currently stored but not enforced via thread — kept for API compat)
    #[serde(default = "default_timeout_ms")]
    timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    30_000
}

/// Execute shell commands safely
pub struct ShellTool;

impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command with optional working directory, environment variables, and timeout"
    }

    fn anthropic_schema(&self) -> Value {
        serde_json::json!({
            "name": self.name(),
            "description": self.description(),
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command executable to run (e.g., 'python3', 'ls')"
                    },
                    "args": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of arguments to pass to the command"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Optional working directory to execute the command in"
                    },
                    "env": {
                        "type": "object",
                        "additionalProperties": {"type": "string"},
                        "description": "Optional environment variables key-value pairs"
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "description": "Command execution timeout in milliseconds (default: 30000)"
                    }
                },
                "required": ["command"]
            }
        })
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let params: ShellInput = serde_json::from_value(input)
            .map_err(|e| ZcodeError::InvalidToolInput(e.to_string()))?;

        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let mut cmd = tokio::process::Command::new("sh");
                let mut full_cmd = params.command.clone();
                for arg in &params.args {
                    full_cmd.push(' ');
                    // Shell escape: wrap in single quotes, replacing ' with '\''
                    let escaped = arg.replace('\'', "'\\''");
                    full_cmd.push('\'');
                    full_cmd.push_str(&escaped);
                    full_cmd.push('\'');
                }
                cmd.arg("-c").arg(&full_cmd);

                if let Some(ref cwd) = params.cwd {
                    cmd.current_dir(cwd);
                }

                for (key, value) in &params.env {
                    cmd.env(key, value);
                }

                cmd.kill_on_drop(true);

                let duration = std::time::Duration::from_millis(params.timeout_ms);
                match tokio::time::timeout(duration, cmd.output()).await {
                    Ok(Ok(output)) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        let exit_code = output.status.code().unwrap_or(-1);

                        Ok(serde_json::json!({
                            "stdout": stdout,
                            "stderr": stderr,
                            "exit_code": exit_code,
                            "success": output.status.success(),
                            "command": params.command,
                            "timeout_ms": params.timeout_ms
                        }))
                    }
                    Ok(Err(e)) => {
                        Err(ZcodeError::ToolExecutionFailed {
                            name: "shell".to_string(),
                            message: format!("Failed to execute '{}': {}", params.command, e),
                        })
                    }
                    Err(_) => {
                        Ok(serde_json::json!({
                            "stdout": "",
                            "stderr": format!("Execution timed out after {} ms", params.timeout_ms),
                            "exit_code": -1,
                            "success": false,
                            "command": params.command,
                            "timeout_ms": params.timeout_ms
                        }))
                    }
                }
            })
        })
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_echo() {
        let result = ShellTool.execute(serde_json::json!({
            "command": "echo",
            "args": ["Hello", "World"]
        })).unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["exit_code"], 0);
        let stdout = result["stdout"].as_str().unwrap();
        assert!(stdout.contains("Hello") && stdout.contains("World"));
    }

    #[test]
    fn test_shell_with_cwd() {
        let result = ShellTool.execute(serde_json::json!({
            "command": "pwd",
            "cwd": "/tmp"
        })).unwrap();

        assert!(result["success"].as_bool().unwrap());
        let stdout = result["stdout"].as_str().unwrap();
        // macOS maps /tmp -> /private/tmp
        assert!(stdout.contains("tmp"));
    }

    #[test]
    fn test_shell_with_env() {
        let result = ShellTool.execute(serde_json::json!({
            "command": "sh",
            "args": ["-c", "echo $MY_TEST_VAR"],
            "env": { "MY_TEST_VAR": "zcode_test_value" }
        })).unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["stdout"].as_str().unwrap().contains("zcode_test_value"));
    }

    #[test]
    fn test_shell_failure_exit_code() {
        let result = ShellTool.execute(serde_json::json!({
            "command": "ls",
            "args": ["/nonexistent_directory_zcode_test_12345"]
        })).unwrap();

        assert!(!result["success"].as_bool().unwrap());
        assert_ne!(result["exit_code"], 0);
        assert!(!result["stderr"].as_str().unwrap().is_empty());
    }

    #[test]
    fn test_shell_nonexistent_command() {
        let result = ShellTool.execute(serde_json::json!({
            "command": "zcode_nonexistent_command_xyz_12345"
        })).unwrap();
        
        assert!(!result["success"].as_bool().unwrap());
        assert_ne!(result["exit_code"], 0);
    }

    #[test]
    fn test_shell_no_args() {
        let result = ShellTool.execute(serde_json::json!({
            "command": "true"
        })).unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["exit_code"], 0);
    }

    #[test]
    fn test_shell_stderr_captured() {
        let result = ShellTool.execute(serde_json::json!({
            "command": "sh",
            "args": ["-c", "echo error >&2; exit 1"]
        })).unwrap();

        assert!(!result["success"].as_bool().unwrap());
        assert!(result["stderr"].as_str().unwrap().contains("error"));
    }

    #[test]
    fn test_shell_stdout_and_stderr() {
        let result = ShellTool.execute(serde_json::json!({
            "command": "sh",
            "args": ["-c", "echo out; echo err >&2"]
        })).unwrap();

        assert!(result["stdout"].as_str().unwrap().contains("out"));
        assert!(result["stderr"].as_str().unwrap().contains("err"));
    }

    #[test]
    fn test_shell_invalid_input() {
        let result = ShellTool.execute(serde_json::json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_shell_returns_command_name() {
        let result = ShellTool.execute(serde_json::json!({
            "command": "echo",
            "args": ["test"]
        })).unwrap();

        assert_eq!(result["command"], "echo");
    }

    #[test]
    fn test_shell_timeout_ms_stored() {
        let result = ShellTool.execute(serde_json::json!({
            "command": "echo",
            "args": ["ok"],
            "timeout_ms": 5000
        })).unwrap();

        assert_eq!(result["timeout_ms"], 5000);
    }

    #[test]
    fn test_shell_multiline_output() {
        let result = ShellTool.execute(serde_json::json!({
            "command": "printf",
            "args": ["line1\\nline2\\nline3"]
        })).unwrap();

        let stdout = result["stdout"].as_str().unwrap();
        assert!(stdout.contains("line1"));
        assert!(stdout.contains("line2"));
        assert!(stdout.contains("line3"));
    }
}
