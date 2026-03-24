//! Lua scripting engine (via mlua)

use crate::error::{Result, ZcodeError};
use crate::script::engine::{ScriptContext, ScriptEngine, ScriptOutput};
use mlua::prelude::*;
use serde_json::Value;
use std::path::Path;
/// Lua 5.4 script engine backed by mlua
pub struct LuaEngine;

impl LuaEngine {
    pub fn new() -> Self { Self }

    /// Build a fresh Lua VM with the zcode API injected
    fn make_lua(&self, ctx: &ScriptContext) -> Result<Lua> {
        let lua = Lua::new();
        Self::inject_zcode_api(&lua, ctx)
            .map_err(|e| ZcodeError::InternalError(format!("Lua init error: {}", e)))?;
        Ok(lua)
    }

    fn inject_zcode_api(lua: &Lua, ctx: &ScriptContext) -> LuaResult<()> {
        let globals = lua.globals();

        // zcode.read_file(path) -> string
        let read_file = lua.create_function(|_, path: String| {
            std::fs::read_to_string(&path)
                .map_err(LuaError::external)
        })?;

        // zcode.write_file(path, content) -> bool
        let write_file = lua.create_function(|_, (path, content): (String, String)| {
            if let Some(parent) = std::path::Path::new(&path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(&path, &content)
                .map(|_| true)
                .map_err(LuaError::external)
        })?;

        // zcode.shell(cmd) -> { stdout, stderr, code }
        let shell = lua.create_function(|lua_ctx, cmd: String| {
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .output()
                .map_err(LuaError::external)?;

            let t = lua_ctx.create_table()?;
            t.set("stdout", String::from_utf8_lossy(&output.stdout).to_string())?;
            t.set("stderr", String::from_utf8_lossy(&output.stderr).to_string())?;
            t.set("code", output.status.code().unwrap_or(-1))?;
            Ok(t)
        })?;

        // zcode.log(msg)
        let log_fn = lua.create_function(|_, msg: String| {
            eprintln!("[zcode.lua] {}", msg);
            Ok(())
        })?;

        let zcode = lua.create_table()?;
        zcode.set("read_file", read_file)?;
        zcode.set("write_file", write_file)?;
        zcode.set("shell", shell)?;
        zcode.set("log", log_fn)?;

        // Inject cwd if provided
        if let Some(cwd) = &ctx.cwd {
            zcode.set("cwd", cwd.as_str())?;
        }

        globals.set("zcode", zcode)?;
        Ok(())
    }

    /// Convert a Lua value to serde_json::Value
    fn lua_to_json(val: LuaValue) -> Value {
        match val {
            LuaValue::Nil => Value::Null,
            LuaValue::Boolean(b) => Value::Bool(b),
            LuaValue::Integer(i) => Value::Number(i.into()),
            LuaValue::Number(n) => {
                serde_json::Number::from_f64(n)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            }
            LuaValue::String(s) => Value::String(s.to_string_lossy().to_string()),
            LuaValue::Table(t) => {
                // Try array first
                let mut arr = Vec::new();
                let mut is_array = true;
                let mut i = 1i64;
                for (k, v) in t.clone().pairs::<LuaValue, LuaValue>().flatten() {
                    if let LuaValue::Integer(ki) = k {
                        if ki == i { arr.push(Self::lua_to_json(v)); i += 1; continue; }
                    }
                    is_array = false;
                    break;
                }
                if is_array && !arr.is_empty() {
                    return Value::Array(arr);
                }
                // Fall back to object
                let mut map = serde_json::Map::new();
                for (k, v) in t.pairs::<String, LuaValue>().flatten() {
                    map.insert(k, Self::lua_to_json(v));
                }
                Value::Object(map)
            }
            _ => Value::Null,
        }
    }
}

impl Default for LuaEngine {
    fn default() -> Self { Self::new() }
}

impl ScriptEngine for LuaEngine {
    fn name(&self) -> &str { "lua" }
    fn extensions(&self) -> &[&str] { &[".lua"] }

    fn eval(&self, code: &str, ctx: &ScriptContext) -> Result<ScriptOutput> {
        let lua = self.make_lua(ctx)?;
        let val: LuaValue = lua.load(code).eval()
            .map_err(|e| ZcodeError::InternalError(format!("Lua eval error: {}", e)))?;
        Ok(ScriptOutput::success(Self::lua_to_json(val)))
    }

    fn call_function(
        &self,
        script_path: &Path,
        function_name: &str,
        args: Value,
        ctx: &ScriptContext,
    ) -> Result<ScriptOutput> {
        let code = std::fs::read_to_string(script_path)
            .map_err(ZcodeError::IoError)?;
        let lua = self.make_lua(ctx)?;

        // Load the script
        lua.load(&code).exec()
            .map_err(|e| ZcodeError::InternalError(format!("Lua load error: {}", e)))?;

        // Get the function
        let func: LuaFunction = lua.globals().get(function_name)
            .map_err(|_| ZcodeError::InternalError(format!("Function '{}' not found in Lua script", function_name)))?;

        // Convert args to Lua table
        let args_str = serde_json::to_string(&args).unwrap_or("{}".to_string());
        let _lua_args: LuaValue = lua.load(
            format!("return (require('json') or {{}}).decode and require('json').decode([=[{}]=]) or load('return {}')() ", args_str, args_str)
        ).eval().unwrap_or(LuaValue::Nil);

        // For simplicity, pass args as a JSON string the function can parse
        let result: LuaValue = func.call(args_str)
            .map_err(|e| ZcodeError::InternalError(format!("Lua call error: {}", e)))?;

        Ok(ScriptOutput::success(Self::lua_to_json(result)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ScriptContext { ScriptContext::default() }

    #[test]
    fn test_lua_engine_name() {
        assert_eq!(LuaEngine::new().name(), "lua");
    }

    #[test]
    fn test_lua_engine_extensions() {
        assert!(LuaEngine::new().extensions().contains(&".lua"));
    }

    #[test]
    fn test_lua_eval_number() {
        let e = LuaEngine::new();
        let out = e.eval("return 42", &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value, serde_json::json!(42));
    }

    #[test]
    fn test_lua_eval_string() {
        let e = LuaEngine::new();
        let out = e.eval(r#"return "hello from lua""#, &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value, serde_json::json!("hello from lua"));
    }

    #[test]
    fn test_lua_eval_boolean() {
        let e = LuaEngine::new();
        let out = e.eval("return true", &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value, serde_json::json!(true));
    }

    #[test]
    fn test_lua_eval_nil() {
        let e = LuaEngine::new();
        let out = e.eval("return nil", &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value, serde_json::json!(null));
    }

    #[test]
    fn test_lua_eval_table_as_object() {
        let e = LuaEngine::new();
        let out = e.eval(r#"return {name = "zcode", version = "1"}"#, &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value["name"], "zcode");
    }

    #[test]
    fn test_lua_eval_arithmetic() {
        let e = LuaEngine::new();
        let out = e.eval("return 10 + 32", &ctx()).unwrap();
        assert_eq!(out.value, serde_json::json!(42));
    }

    #[test]
    fn test_lua_zcode_api_available() {
        let e = LuaEngine::new();
        let out = e.eval("return type(zcode)", &ctx()).unwrap();
        assert_eq!(out.value, serde_json::json!("table"));
    }

    #[test]
    fn test_lua_read_file() {
        use tempfile::NamedTempFile;
        use std::io::Write;
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "hello lua").unwrap();
        let path = f.path().to_str().unwrap().to_string();

        let e = LuaEngine::new();
        let out = e.eval(&format!(r#"return zcode.read_file("{}")"#, path), &ctx()).unwrap();
        assert!(out.success);
        assert!(out.value.as_str().unwrap().contains("hello lua"));
    }

    #[test]
    fn test_lua_write_file() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("output.txt");

        let e = LuaEngine::new();
        let out = e.eval(
            &format!(r#"return zcode.write_file("{}", "lua output")"#, path.display()),
            &ctx(),
        ).unwrap();
        assert!(out.success);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "lua output");
    }

    #[test]
    fn test_lua_shell() {
        let e = LuaEngine::new();
        let out = e.eval(
            r#"local r = zcode.shell("echo hello_world"); return r.stdout"#,
            &ctx(),
        ).unwrap();
        assert!(out.success);
        assert!(out.value.as_str().unwrap().contains("hello_world"));
    }

    #[test]
    fn test_lua_syntax_error() {
        let e = LuaEngine::new();
        let result = e.eval("this is not valid lua !!!", &ctx());
        assert!(result.is_err());
    }

    #[test]
    fn test_call_function_from_file() {
        use tempfile::NamedTempFile;
        use std::io::Write;
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, r#"function greet(args_json) return "Hello, zcode!" end"#).unwrap();
        let e = LuaEngine::new();
        let out = e.call_function(f.path(), "greet", serde_json::json!({}), &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value.as_str().unwrap(), "Hello, zcode!");
    }
}
