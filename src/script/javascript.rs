//! JavaScript scripting engine (via rquickjs — vendored QuickJS)
//!
//! Uses the correct rquickjs 0.7 API patterns.

use crate::error::{Result, ZcodeError};
use crate::script::engine::{ScriptContext, ScriptEngine, ScriptOutput};
use rquickjs::prelude::Func;
use rquickjs::{Context, Object, Runtime, Value as JsValue};
use serde_json::Value;
use std::path::Path;

/// JavaScript engine backed by rquickjs (QuickJS, no Node/Deno)
pub struct JsEngine;

impl JsEngine {
    pub fn new() -> Self { Self }

    /// Inject the `zcode` global object into a QuickJS context (via ctx.with)
    fn inject_zcode(ctx: &rquickjs::Ctx) -> rquickjs::Result<()> {
        let zcode = Object::new(ctx.clone())?;

        zcode.set("read_file", Func::new(|path: String| {
            std::fs::read_to_string(&path).unwrap_or_default()
        }))?;

        zcode.set("write_file", Func::new(|path: String, content: String| {
            if let Some(parent) = std::path::Path::new(&path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(&path, &content).is_ok()
        }))?;

        zcode.set("log", Func::new(|msg: String| {
            eprintln!("[zcode.js] {}", msg);
        }))?;

        ctx.globals().set("zcode", zcode)?;
        Ok(())
    }

    /// Convert a QuickJS Value to serde_json::Value
    fn js_to_json(val: &JsValue) -> Value {
        use rquickjs::Type;
        match val.type_of() {
            Type::Null | Type::Undefined => Value::Null,
            Type::Bool => val.as_bool().map(Value::Bool).unwrap_or(Value::Null),
            Type::Int => val.as_int().map(|i| Value::Number(i.into())).unwrap_or(Value::Null),
            Type::Float => val.as_float()
                .and_then(|f| serde_json::Number::from_f64(f))
                .map(Value::Number)
                .unwrap_or(Value::Null),
            Type::String => val.as_string()
                .and_then(|s| s.to_string().ok())
                .map(Value::String)
                .unwrap_or(Value::Null),
            Type::Array => {
                if let Some(arr) = val.as_array() {
                    let items: Vec<Value> = arr.iter::<JsValue>()
                        .filter_map(|v| v.ok())
                        .map(|v| Self::js_to_json(&v))
                        .collect();
                    Value::Array(items)
                } else {
                    Value::Null
                }
            }
            Type::Object => {
                if let Some(obj) = val.as_object() {
                    let mut map = serde_json::Map::new();
                    for key in obj.keys::<String>().filter_map(|k| k.ok()) {
                        if let Ok(v) = obj.get::<_, JsValue>(key.as_str()) {
                            map.insert(key, Self::js_to_json(&v));
                        }
                    }
                    Value::Object(map)
                } else {
                    Value::Null
                }
            }
            _ => Value::Null,
        }
    }
}

impl Default for JsEngine {
    fn default() -> Self { Self::new() }
}

impl ScriptEngine for JsEngine {
    fn name(&self) -> &str { "javascript" }
    fn extensions(&self) -> &[&str] { &[".js", ".mjs"] }

    fn eval(&self, code: &str, _ctx: &ScriptContext) -> Result<ScriptOutput> {
        let rt = Runtime::new()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        let ctx = Context::full(&rt)
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        let result: Result<Value> = ctx.with(|js_ctx| {
            Self::inject_zcode(&js_ctx)
                .map_err(|e| ZcodeError::InternalError(format!("JS inject error: {}", e)))?;

            let val: JsValue = js_ctx.eval(code.as_bytes())
                .map_err(|e| ZcodeError::InternalError(format!("JS eval error: {}", e)))?;

            Ok(Self::js_to_json(&val))
        });

        result.map(ScriptOutput::success)
    }

    fn call_function(
        &self,
        script_path: &Path,
        function_name: &str,
        args: Value,
        _ctx: &ScriptContext,
    ) -> Result<ScriptOutput> {
        let code = std::fs::read_to_string(script_path)?;
        let fn_name = function_name.to_string();
        let args_str = serde_json::to_string(&args).unwrap_or("{}".to_string());

        let rt = Runtime::new()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;
        let ctx = Context::full(&rt)
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

        let result: Result<Value> = ctx.with(|js_ctx| {
            Self::inject_zcode(&js_ctx)
                .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

            // Execute the script file
            let _: JsValue = js_ctx.eval::<JsValue, _>(code.as_bytes())
                .map_err(|e| ZcodeError::InternalError(format!("JS script error: {}", e)))?;

            // Get the function and call it with args_str
            let call_code = format!("{}({})", fn_name, serde_json::to_string(&args_str).unwrap_or("\"{}\"".to_string()));
            let val: JsValue = js_ctx.eval::<JsValue, _>(call_code.as_bytes())
                .map_err(|e| ZcodeError::InternalError(format!("JS call error: {}", e)))?;

            Ok(Self::js_to_json(&val))
        });

        result.map(ScriptOutput::success)
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ScriptContext { ScriptContext::default() }

    #[test]
    fn test_js_engine_name() {
        assert_eq!(JsEngine::new().name(), "javascript");
    }

    #[test]
    fn test_js_engine_extensions() {
        let e = JsEngine::new();
        assert!(e.extensions().contains(&".js"));
        assert!(e.extensions().contains(&".mjs"));
    }

    #[test]
    fn test_js_eval_number() {
        let e = JsEngine::new();
        let out = e.eval("42", &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value, serde_json::json!(42));
    }

    #[test]
    fn test_js_eval_string() {
        let e = JsEngine::new();
        let out = e.eval(r#""hello from js""#, &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value, serde_json::json!("hello from js"));
    }

    #[test]
    fn test_js_eval_boolean() {
        let e = JsEngine::new();
        let out = e.eval("true", &ctx()).unwrap();
        assert_eq!(out.value, serde_json::json!(true));
    }

    #[test]
    fn test_js_eval_null() {
        let e = JsEngine::new();
        let out = e.eval("null", &ctx()).unwrap();
        assert_eq!(out.value, serde_json::json!(null));
    }

    #[test]
    fn test_js_eval_array() {
        let e = JsEngine::new();
        let out = e.eval("[1, 2, 3]", &ctx()).unwrap();
        assert_eq!(out.value, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn test_js_eval_object() {
        let e = JsEngine::new();
        let out = e.eval(r#"({name: "zcode", version: 5})"#, &ctx()).unwrap();
        assert_eq!(out.value["name"], "zcode");
        assert_eq!(out.value["version"], 5);
    }

    #[test]
    fn test_js_eval_arithmetic() {
        let e = JsEngine::new();
        let out = e.eval("10 + 32", &ctx()).unwrap();
        assert_eq!(out.value, serde_json::json!(42));
    }

    #[test]
    fn test_js_zcode_api_available() {
        let e = JsEngine::new();
        let out = e.eval("typeof zcode", &ctx()).unwrap();
        assert_eq!(out.value, serde_json::json!("object"));
    }

    #[test]
    fn test_js_read_file() {
        use tempfile::NamedTempFile;
        use std::io::Write;
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "js content").unwrap();
        let path = f.path().to_str().unwrap().replace('\\', "/");

        let e = JsEngine::new();
        let out = e.eval(
            &format!(r#"zcode.read_file("{}")"#, path),
            &ctx()
        ).unwrap();
        assert!(out.success);
        assert!(out.value.as_str().unwrap().contains("js content"));
    }

    #[test]
    fn test_js_syntax_error() {
        let e = JsEngine::new();
        let result = e.eval("if (", &ctx());
        assert!(result.is_err());
    }

    #[test]
    fn test_call_function_from_file() {
        use tempfile::NamedTempFile;
        use std::io::Write;
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "function greet(args) {{ return 'Hello from JS: ' + args; }}").unwrap();

        let e = JsEngine::new();
        let out = e.call_function(
            f.path(),
            "greet",
            serde_json::json!({"user": "world"}),
            &ctx()
        ).unwrap();
        assert!(out.success);
        assert!(out.value.as_str().unwrap().starts_with("Hello from JS:"));
    }

    #[test]
    fn test_js_handles_extensions() {
        use std::path::PathBuf;
        let e = JsEngine::new();
        assert!(e.handles(&PathBuf::from("app.js")));
        assert!(e.handles(&PathBuf::from("module.mjs")));
        assert!(!e.handles(&PathBuf::from("script.py")));
    }
}
