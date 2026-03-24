//! Integration tests: Multi-language scripting engines
//!
//! Tests Lua, Python, JavaScript, and Shell engines on real scripts.

use std::io::Write;
use tempfile::NamedTempFile;
use zcode::script::{ScriptContext, ScriptEngine};
use zcode::script::lua::LuaEngine;
use zcode::script::python::PythonEngine;
use zcode::script::javascript::JsEngine;
use zcode::script::shell::ShellEngine;

fn ctx() -> ScriptContext { ScriptContext::default() }

// ─── Lua integration tests ────────────────────────────────────────────────────

#[test]
fn test_lua_fibonacci() {
    let engine = LuaEngine::new();
    let code = r#"
        local function fib(n)
            if n <= 1 then return n end
            return fib(n-1) + fib(n-2)
        end
        return fib(10)
    "#;
    let result = engine.eval(code, &ctx()).unwrap();
    assert_eq!(result.value, serde_json::json!(55));
}

#[test]
fn test_lua_string_operations() {
    let engine = LuaEngine::new();
    let result = engine.eval(
        r#"return string.upper("hello from lua")"#,
        &ctx()
    ).unwrap();
    assert_eq!(result.value, serde_json::json!("HELLO FROM LUA"));
}

#[test]
fn test_lua_table_manipulation() {
    let engine = LuaEngine::new();
    let result = engine.eval(
        r#"
        local t = {1, 2, 3, 4, 5}
        local sum = 0
        for _, v in ipairs(t) do sum = sum + v end
        return sum
        "#,
        &ctx()
    ).unwrap();
    assert_eq!(result.value, serde_json::json!(15));
}

#[test]
fn test_lua_read_file_api() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "lua reads this").unwrap();
    let path = f.path().to_str().unwrap().to_string();

    let engine = LuaEngine::new();
    let code = format!(r#"return zcode.read_file("{}")"#, path);
    let result = engine.eval(&code, &ctx()).unwrap();
    assert!(result.value.as_str().unwrap().contains("lua reads this"));
}

#[test]
fn test_lua_write_and_read_file() {
    use tempfile::TempDir;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("output.txt");
    let path_str = path.to_str().unwrap().replace('\\', "/");

    let engine = LuaEngine::new();
    let write_code = format!(
        r#"return zcode.write_file("{}", "hello from lua write")"#,
        path_str
    );
    let result = engine.eval(&write_code, &ctx()).unwrap();
    assert_eq!(result.value, serde_json::json!(true));

    let content = std::fs::read_to_string(&path).unwrap();
    assert_eq!(content, "hello from lua write");
}

#[test]
fn test_lua_shell_command() {
    let engine = LuaEngine::new();
    let result = engine.eval(
        r#"
        local r = zcode.shell("echo integration_test_output")
        return r.stdout
        "#,
        &ctx()
    ).unwrap();
    assert!(result.value.as_str().unwrap().contains("integration_test_output"));
}

#[test]
fn test_lua_function_call_from_file() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, r#"
function process(args_json)
    local n = 0
    for c in string.gmatch(args_json, ".") do n = n + 1 end
    return "processed:" .. n
end
"#).unwrap();

    let engine = LuaEngine::new();
    let result = engine.call_function(
        f.path(),
        "process",
        serde_json::json!({"key": "value"}),
        &ctx()
    ).unwrap();
    assert!(result.value.as_str().unwrap().starts_with("processed:"));
}

// ─── JavaScript integration tests ─────────────────────────────────────────────

#[test]
fn test_js_fibonacci() {
    let engine = JsEngine::new();
    let code = r#"
        function fib(n) {
            if (n <= 1) return n;
            return fib(n-1) + fib(n-2);
        }
        fib(10)
    "#;
    let result = engine.eval(code, &ctx()).unwrap();
    assert_eq!(result.value, serde_json::json!(55));
}

#[test]
fn test_js_array_operations() {
    let engine = JsEngine::new();
    let result = engine.eval(
        "[1, 2, 3, 4, 5].reduce((a, b) => a + b, 0)",
        &ctx()
    ).unwrap();
    assert_eq!(result.value, serde_json::json!(15));
}

#[test]
fn test_js_string_template() {
    let engine = JsEngine::new();
    let result = engine.eval(
        r#"
        const name = "zcode";
        const version = 7;
        `Welcome to ${name} v${version}!`
        "#,
        &ctx()
    ).unwrap();
    assert_eq!(result.value, serde_json::json!("Welcome to zcode v7!"));
}

#[test]
fn test_js_object_manipulation() {
    let engine = JsEngine::new();
    let result = engine.eval(
        r#"
        const config = { name: "test", score: 42, active: true };
        config
        "#,
        &ctx()
    ).unwrap();
    assert_eq!(result.value["name"], "test");
    assert_eq!(result.value["score"], 42);
    assert_eq!(result.value["active"], true);
}

#[test]
fn test_js_read_file_api() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "js reads this").unwrap();
    let path = f.path().to_str().unwrap().replace('\\', "/");

    let engine = JsEngine::new();
    let code = format!(r#"zcode.read_file("{}")"#, path);
    let result = engine.eval(&code, &ctx()).unwrap();
    assert!(result.value.as_str().unwrap().contains("js reads this"));
}

#[test]
fn test_js_zcode_global_available() {
    let engine = JsEngine::new();
    let result = engine.eval("typeof zcode", &ctx()).unwrap();
    assert_eq!(result.value, serde_json::json!("object"));
}

// ─── Shell integration tests ──────────────────────────────────────────────────

#[test]
fn test_shell_echo() {
    let engine = ShellEngine::new();
    let result = engine.eval("echo 'shell_integration_ok'", &ctx()).unwrap();
    assert!(result.value["stdout"].as_str().unwrap().contains("shell_integration_ok"));
}

#[test]
fn test_shell_exit_code_success() {
    let engine = ShellEngine::new();
    let result = engine.eval("true", &ctx()).unwrap();
    assert_eq!(result.value["exit_code"], 0);
}

#[test]
fn test_shell_exit_code_failure() {
    let engine = ShellEngine::new();
    let result = engine.eval("false", &ctx()).unwrap();
    assert_ne!(result.value["exit_code"], 0);
}

#[test]
fn test_shell_capture_stderr() {
    let engine = ShellEngine::new();
    let result = engine.eval("echo 'err output' >&2", &ctx()).unwrap();
    assert!(result.value["stderr"].as_str().unwrap().contains("err output"));
}

#[test]
fn test_shell_multiline_script() {
    let engine = ShellEngine::new();
    let script = r#"
        A=10
        B=32
        echo $((A + B))
    "#;
    let result = engine.eval(script, &ctx()).unwrap();
    assert!(result.value["stdout"].as_str().unwrap().contains("42"));
}

#[test]
fn test_shell_file_from_script() {
    use tempfile::NamedTempFile;
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "#!/bin/sh\necho 'script_file_ok'").unwrap();

    let engine = ShellEngine::new();
    let result = engine.call_function(f.path(), "main", serde_json::json!({}), &ctx()).unwrap();
    assert!(result.value["stdout"].as_str().unwrap().contains("script_file_ok"));
}

// ─── Cross-engine: script as tool ────────────────────────────────────────────

#[test]
fn test_lua_handles_file_by_extension() {
    use std::path::PathBuf;
    let engine = LuaEngine::new();
    assert!(engine.handles(&PathBuf::from("script.lua")));
    assert!(!engine.handles(&PathBuf::from("script.js")));
}

#[test]
fn test_js_handles_file_by_extension() {
    use std::path::PathBuf;
    let engine = JsEngine::new();
    assert!(engine.handles(&PathBuf::from("app.js")));
    assert!(engine.handles(&PathBuf::from("module.mjs")));
    assert!(!engine.handles(&PathBuf::from("script.py")));
}

#[test]
fn test_shell_handles_file_by_extension() {
    use std::path::PathBuf;
    let engine = ShellEngine::new();
    assert!(engine.handles(&PathBuf::from("deploy.sh")));
    assert!(!engine.handles(&PathBuf::from("app.js")));
}
