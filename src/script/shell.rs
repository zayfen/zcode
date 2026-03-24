//! Shell/Bash scripting engine (native subprocess)

use crate::error::{Result, ZcodeError};
use crate::script::engine::{ScriptContext, ScriptEngine, ScriptOutput};
use serde_json::Value;
use std::path::Path;
use std::process::Command;

/// Shell scripting engine using the system shell (sh/bash)
pub struct ShellEngine;

impl ShellEngine {
    pub fn new() -> Self { Self }

    fn run_shell(
        script: &str,
        env_vars: &std::collections::HashMap<String, String>,
        cwd: Option<&str>,
    ) -> Result<ScriptOutput> {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(script);

        // Inject environment variables
        for (k, v) in env_vars {
            cmd.env(k, v);
        }

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        let output = cmd.output()
            .map_err(|e| ZcodeError::InternalError(format!("Shell exec error: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        let success = output.status.success();

        let value = serde_json::json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
        });

        Ok(ScriptOutput {
            value,
            stdout: stdout.clone(),
            success,
        })
    }
}

impl Default for ShellEngine {
    fn default() -> Self { Self::new() }
}

impl ScriptEngine for ShellEngine {
    fn name(&self) -> &str { "shell" }
    fn extensions(&self) -> &[&str] { &[".sh", ".bash"] }

    fn eval(&self, code: &str, ctx: &ScriptContext) -> Result<ScriptOutput> {
        Self::run_shell(code, &ctx.env, ctx.cwd.as_deref())
    }

    fn call_function(
        &self,
        script_path: &Path,
        function_name: &str,
        args: Value,
        ctx: &ScriptContext,
    ) -> Result<ScriptOutput> {
        let script_code = std::fs::read_to_string(script_path)
            .map_err(|e| ZcodeError::IoError(e))?;

        let args_str = serde_json::to_string(&args).unwrap_or("{}".to_string());

        // Source the script, then call the function with args as $1
        let combined = format!(
            "{}\n{} '{}'",
            script_code,
            function_name,
            args_str.replace('\'', "'\\''"), // escape single quotes
        );

        let mut env = ctx.env.clone();
        env.insert("ZCODE_ARGS".to_string(), args_str);

        Self::run_shell(&combined, &env, ctx.cwd.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ScriptContext { ScriptContext::default() }

    #[test]
    fn test_shell_engine_name() {
        assert_eq!(ShellEngine::new().name(), "shell");
    }

    #[test]
    fn test_shell_engine_extensions() {
        let e = ShellEngine::new();
        assert!(e.extensions().contains(&".sh"));
        assert!(e.extensions().contains(&".bash"));
    }

    #[test]
    fn test_shell_eval_echo() {
        let e = ShellEngine::new();
        let out = e.eval("echo hello_shell", &ctx()).unwrap();
        assert!(out.success);
        assert!(out.value["stdout"].as_str().unwrap().contains("hello_shell"));
    }

    #[test]
    fn test_shell_eval_exit_code_zero() {
        let e = ShellEngine::new();
        let out = e.eval("true", &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value["exit_code"], 0);
    }

    #[test]
    fn test_shell_eval_exit_code_nonzero() {
        let e = ShellEngine::new();
        let out = e.eval("false", &ctx()).unwrap();
        assert!(!out.success);
        assert_ne!(out.value["exit_code"].as_i64().unwrap(), 0);
    }

    #[test]
    fn test_shell_eval_arithmetic() {
        let e = ShellEngine::new();
        let out = e.eval("echo $((10 + 32))", &ctx()).unwrap();
        assert!(out.success);
        assert!(out.value["stdout"].as_str().unwrap().contains("42"));
    }

    #[test]
    fn test_shell_eval_env_vars() {
        let mut ctx = ScriptContext::default();
        ctx.env.insert("MY_VAR".to_string(), "hello_env".to_string());
        let e = ShellEngine::new();
        let out = e.eval("echo $MY_VAR", &ctx).unwrap();
        assert!(out.success);
        assert!(out.value["stdout"].as_str().unwrap().contains("hello_env"));
    }

    #[test]
    fn test_shell_eval_stderr_captured() {
        let e = ShellEngine::new();
        let out = e.eval("echo error_msg >&2; true", &ctx()).unwrap();
        assert!(out.success);
        assert!(out.value["stderr"].as_str().unwrap().contains("error_msg"));
    }

    #[test]
    fn test_shell_call_function() {
        use tempfile::NamedTempFile;
        use std::io::Write;
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "greet() {{ echo \"Hello from shell: $1\"; }}").unwrap();

        let e = ShellEngine::new();
        let out = e.call_function(f.path(), "greet", serde_json::json!({"user": "world"}), &ctx()).unwrap();
        assert!(out.success);
        assert!(out.value["stdout"].as_str().unwrap().contains("Hello from shell:"));
    }

    #[test]
    fn test_shell_handles_extensions() {
        let e = ShellEngine::new();
        use std::path::PathBuf;
        assert!(e.handles(&PathBuf::from("deploy.sh")));
        assert!(e.handles(&PathBuf::from("run.bash")));
        assert!(!e.handles(&PathBuf::from("run.py")));
    }
}
