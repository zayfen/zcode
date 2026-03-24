//! Script Manager — routes scripts to the correct engine by file extension

use crate::error::{Result, ZcodeError};
use crate::script::engine::{ScriptContext, ScriptEngine, ScriptOutput};
use crate::script::hooks::HookRegistry;
use crate::tools::{Tool, ToolResult};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ─── ScriptManager ─────────────────────────────────────────────────────────────

/// Manages multiple scripting engines and routes script execution by extension
pub struct ScriptManager {
    engines: Vec<Box<dyn ScriptEngine>>,
    pub hooks: HookRegistry,
}

impl ScriptManager {
    /// Create an empty manager (no engines registered)
    pub fn new() -> Self {
        Self {
            engines: Vec::new(),
            hooks: HookRegistry::new(),
        }
    }

    /// Register a scripting engine
    pub fn add_engine(&mut self, engine: Box<dyn ScriptEngine>) {
        self.engines.push(engine);
    }

    /// Find the engine for a given file extension (returns None if unsupported)
    pub fn engine_for(&self, path: &Path) -> Option<&dyn ScriptEngine> {
        self.engines.iter().find(|e| e.handles(path)).map(|e| e.as_ref())
    }

    /// Get all registered engine names
    pub fn engine_names(&self) -> Vec<&str> {
        self.engines.iter().map(|e| e.name()).collect()
    }

    /// Eval a code snippet using the given language name
    pub fn eval_with_language(
        &self,
        language: &str,
        code: &str,
        ctx: &ScriptContext,
    ) -> Result<ScriptOutput> {
        let engine = self
            .engines
            .iter()
            .find(|e| e.name() == language)
            .ok_or_else(|| {
                ZcodeError::InternalError(format!("No script engine for language: {}", language))
            })?;
        engine.eval(code, ctx)
    }

    /// Execute a script file, calling the given function
    pub fn call_script_function(
        &self,
        script_path: &Path,
        function_name: &str,
        args: Value,
        ctx: &ScriptContext,
    ) -> Result<ScriptOutput> {
        let engine = self.engine_for(script_path).ok_or_else(|| {
            ZcodeError::InternalError(format!(
                "No script engine for file: {}",
                script_path.display()
            ))
        })?;
        engine.call_function(script_path, function_name, args, ctx)
    }

    /// Load all scripts from a directory, returning ScriptTool objects
    pub fn load_scripts_from_dir(
        self: &Arc<Self>,
        dir: &Path,
    ) -> Vec<ScriptTool> {
        let mut tools = Vec::new();

        if !dir.exists() || !dir.is_dir() {
            return tools;
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && self.engine_for(&path).is_some() {
                    let tool_name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unnamed")
                        .to_string();

                    tools.push(ScriptTool {
                        manager: Arc::clone(self),
                        script_path: path,
                        tool_name,
                        tool_description: "Custom script tool".to_string(),
                        function_name: "call".to_string(),
                    });
                }
            }
        }

        tools
    }
}

impl Default for ScriptManager {
    fn default() -> Self { Self::new() }
}

// ─── ScriptTool ────────────────────────────────────────────────────────────────

/// A script file wrapped as a zcode Tool trait object
pub struct ScriptTool {
    manager: Arc<ScriptManager>,
    pub script_path: PathBuf,
    tool_name: String,
    tool_description: String,
    function_name: String,
}

impl ScriptTool {
    pub fn new(
        manager: Arc<ScriptManager>,
        script_path: PathBuf,
        tool_name: impl Into<String>,
        description: impl Into<String>,
        function_name: impl Into<String>,
    ) -> Self {
        Self {
            manager,
            script_path,
            tool_name: tool_name.into(),
            tool_description: description.into(),
            function_name: function_name.into(),
        }
    }
}

impl Tool for ScriptTool {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.tool_description
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let ctx = ScriptContext::default();
        self.manager
            .call_script_function(&self.script_path, &self.function_name, input, &ctx)
            .map(|out| out.value)
            .map_err(|e| ZcodeError::ToolExecutionFailed {
                name: self.tool_name.clone(),
                message: e.to_string(),
            })
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::engine::MockEngine;
    use std::path::PathBuf;

    fn make_manager() -> Arc<ScriptManager> {
        let mut mgr = ScriptManager::new();
        mgr.add_engine(Box::new(MockEngine::new("lua", vec![".lua"])));
        mgr.add_engine(Box::new(MockEngine::new("python", vec![".py"])));
        mgr.add_engine(Box::new(MockEngine::new("javascript", vec![".js"])));
        mgr.add_engine(Box::new(MockEngine::new("shell", vec![".sh"])));
        Arc::new(mgr)
    }

    #[test]
    fn test_script_manager_engine_names() {
        let mgr = make_manager();
        let names = mgr.engine_names();
        assert!(names.contains(&"lua"));
        assert!(names.contains(&"python"));
        assert!(names.contains(&"javascript"));
        assert!(names.contains(&"shell"));
    }

    #[test]
    fn test_engine_for_by_extension() {
        let mgr = make_manager();
        assert!(mgr.engine_for(&PathBuf::from("test.lua")).is_some());
        assert!(mgr.engine_for(&PathBuf::from("test.py")).is_some());
        assert!(mgr.engine_for(&PathBuf::from("test.js")).is_some());
        assert!(mgr.engine_for(&PathBuf::from("test.sh")).is_some());
        assert!(mgr.engine_for(&PathBuf::from("test.rs")).is_none());
    }

    #[test]
    fn test_eval_with_language() {
        let mgr = make_manager();
        let ctx = ScriptContext::default();
        let out = mgr.eval_with_language("lua", "return 42", &ctx).unwrap();
        assert!(out.success);
    }

    #[test]
    fn test_eval_with_unknown_language() {
        let mgr = make_manager();
        let ctx = ScriptContext::default();
        let result = mgr.eval_with_language("ruby", "puts 42", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_call_script_function_mock() {
        let mgr = make_manager();
        let ctx = ScriptContext::default();
        let out = mgr.call_script_function(
            &PathBuf::from("deploy.sh"),
            "run",
            serde_json::json!({"env": "prod"}),
            &ctx,
        ).unwrap();
        assert!(out.success);
    }

    #[test]
    fn test_call_script_function_unsupported_extension() {
        let mgr = make_manager();
        let ctx = ScriptContext::default();
        let result = mgr.call_script_function(
            &PathBuf::from("script.rb"),
            "run",
            serde_json::json!({}),
            &ctx,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_load_scripts_from_nonexistent_dir() {
        let mgr = make_manager();
        let tools = mgr.load_scripts_from_dir(&PathBuf::from("/nonexistent/path"));
        assert!(tools.is_empty());
    }

    #[test]
    fn test_load_scripts_from_dir() {
        use tempfile::TempDir;
        use std::io::Write;
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("deploy.sh"), "call() { echo done; }").unwrap();
        std::fs::write(dir.path().join("analyze.py"), "def call(args): return 42").unwrap();
        std::fs::write(dir.path().join("README.md"), "# docs").unwrap(); // should be ignored

        let mgr = make_manager();
        let tools = mgr.load_scripts_from_dir(dir.path());
        assert_eq!(tools.len(), 2);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"deploy") || names.contains(&"analyze"));
    }

    #[test]
    fn test_script_tool_name_and_description() {
        let mgr = make_manager();
        let tool = ScriptTool::new(
            mgr,
            PathBuf::from("test.lua"),
            "my_tool",
            "My custom tool",
            "call",
        );
        assert_eq!(tool.name(), "my_tool");
        assert_eq!(tool.description(), "My custom tool");
    }
}
