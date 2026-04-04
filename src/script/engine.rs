//! Script engine abstraction — unified trait for all scripting backends.
//!
//! Supports Lua, Python, JavaScript, and Shell.

use crate::error::Result;
use serde_json::Value;
use std::path::Path;

// ─── ScriptContext ─────────────────────────────────────────────────────────────

/// Context provided to scripts: the zcode API surface.
/// Scripts receive this as a set of callable functions.
#[derive(Debug, Clone, Default)]
pub struct ScriptContext {
    /// Current working directory for relative paths
    pub cwd: Option<String>,
    /// Additional key-value bindings accessible in the script
    pub env: std::collections::HashMap<String, String>,
}

// ─── ScriptOutput ──────────────────────────────────────────────────────────────

/// The result of executing a script
#[derive(Debug, Clone)]
pub struct ScriptOutput {
    /// JSON return value (or null)
    pub value: Value,
    /// Any printed stdout from the script
    pub stdout: String,
    /// Whether execution succeeded
    pub success: bool,
}

impl ScriptOutput {
    pub fn success(value: Value) -> Self {
        Self { value, stdout: String::new(), success: true }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            value: Value::Null,
            stdout: msg.into(),
            success: false,
        }
    }
}

// ─── ScriptEngine trait ────────────────────────────────────────────────────────

/// Unified trait for all scripting backends
pub trait ScriptEngine: Send + Sync {
    /// Engine name (e.g. "lua", "python", "javascript", "shell")
    fn name(&self) -> &str;

    /// File extensions handled by this engine (e.g. [".lua"])
    fn extensions(&self) -> &[&str];

    /// Evaluate a code snippet and return its result
    fn eval(&self, code: &str, ctx: &ScriptContext) -> Result<ScriptOutput>;

    /// Load a script file and call a specific function
    fn call_function(
        &self,
        script_path: &Path,
        function_name: &str,
        args: Value,
        ctx: &ScriptContext,
    ) -> Result<ScriptOutput>;

    /// Check if this engine handles a given file extension
    fn handles(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext = format!(".{}", ext.to_string_lossy().to_lowercase());
            self.extensions().contains(&ext.as_str())
        } else {
            false
        }
    }
}

// ─── MockEngine (test helper) ──────────────────────────────────────────────────

/// A simple mock engine for testing the trait interface
pub struct MockEngine {
    pub engine_name: String,
    pub exts: Vec<&'static str>,
}

impl MockEngine {
    pub fn new(name: impl Into<String>, exts: Vec<&'static str>) -> Self {
        Self { engine_name: name.into(), exts }
    }
}

impl ScriptEngine for MockEngine {
    fn name(&self) -> &str { &self.engine_name }
    fn extensions(&self) -> &[&str] { &self.exts }

    fn eval(&self, code: &str, _ctx: &ScriptContext) -> Result<ScriptOutput> {
        Ok(ScriptOutput {
            value: serde_json::json!({ "code": code }),
            stdout: format!("[{}] eval: {}", self.engine_name, code),
            success: true,
        })
    }

    fn call_function(
        &self,
        script_path: &Path,
        function_name: &str,
        args: Value,
        _ctx: &ScriptContext,
    ) -> Result<ScriptOutput> {
        Ok(ScriptOutput {
            value: args,
            stdout: format!(
                "[{}] call {}::{} done",
                self.engine_name,
                script_path.display(),
                function_name
            ),
            success: true,
        })
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_mock_engine_name() {
        let e = MockEngine::new("lua", vec![".lua"]);
        assert_eq!(e.name(), "lua");
    }

    #[test]
    fn test_mock_engine_extensions() {
        let e = MockEngine::new("python", vec![".py", ".pyw"]);
        assert_eq!(e.extensions(), &[".py", ".pyw"]);
    }

    #[test]
    fn test_mock_engine_handles() {
        let e = MockEngine::new("lua", vec![".lua"]);
        assert!(e.handles(&PathBuf::from("script.lua")));
        assert!(!e.handles(&PathBuf::from("script.py")));
        assert!(!e.handles(&PathBuf::from("script"))); // no ext
    }

    #[test]
    fn test_mock_engine_eval() {
        let e = MockEngine::new("lua", vec![".lua"]);
        let ctx = ScriptContext::default();
        let out = e.eval("return 42", &ctx).unwrap();
        assert!(out.success);
        assert!(out.stdout.contains("return 42"));
    }

    #[test]
    fn test_mock_engine_call_function() {
        let e = MockEngine::new("python", vec![".py"]);
        let ctx = ScriptContext::default();
        let args = serde_json::json!({ "x": 1 });
        let out = e.call_function(
            &PathBuf::from("deploy.py"),
            "run",
            args.clone(),
            &ctx,
        ).unwrap();
        assert!(out.success);
        assert_eq!(out.value, args);
        assert!(out.stdout.contains("deploy.py"));
        assert!(out.stdout.contains("run"));
    }

    #[test]
    fn test_script_output_success() {
        let out = ScriptOutput::success(serde_json::json!({"result": 42}));
        assert!(out.success);
        assert_eq!(out.value["result"], 42);
    }

    #[test]
    fn test_script_output_error() {
        let out = ScriptOutput::error("syntax error");
        assert!(!out.success);
        assert!(out.stdout.contains("syntax error"));
    }

    #[test]
    fn test_script_context_default() {
        let ctx = ScriptContext::default();
        assert!(ctx.cwd.is_none());
        assert!(ctx.env.is_empty());
    }

    #[test]
    fn test_handles_case_insensitive() {
        let e = MockEngine::new("lua", vec![".lua"]);
        // .LUA should match .lua
        assert!(e.handles(&PathBuf::from("SCRIPT.LUA")));
    }
}
