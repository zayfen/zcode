//! Script module — multi-language scripting engine for zcode
//!
//! Supports Lua, Python, JavaScript, and Shell.

pub mod engine;
pub mod hooks;
pub mod lua;
pub mod python;
pub mod javascript;
pub mod shell;
pub mod manager;

pub use engine::{ScriptContext, ScriptEngine, ScriptOutput};
pub use hooks::{HookContext, HookRegistry, HookType};
pub use lua::LuaEngine;
pub use python::PythonEngine;
pub use javascript::JsEngine;
pub use shell::ShellEngine;
pub use manager::{ScriptManager, ScriptTool};

use std::sync::Arc;

/// Build a ScriptManager pre-loaded with all supported engines
pub fn default_script_manager() -> Arc<ScriptManager> {
    let mut mgr = ScriptManager::new();
    mgr.add_engine(Box::new(LuaEngine::new()));
    mgr.add_engine(Box::new(PythonEngine::new()));
    mgr.add_engine(Box::new(JsEngine::new()));
    mgr.add_engine(Box::new(ShellEngine::new()));
    Arc::new(mgr)
}
