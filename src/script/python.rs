//! Python scripting engine (via pyo3)

use crate::error::{Result, ZcodeError};
use crate::script::engine::{ScriptContext, ScriptEngine, ScriptOutput};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use serde_json::Value;
use std::path::Path;

/// Python scripting engine backed by pyo3 (uses system Python)
pub struct PythonEngine;

impl PythonEngine {
    pub fn new() -> Self { Self }

    /// Convert a Python object to serde_json::Value
    fn py_to_json(py: Python, obj: &Bound<PyAny>) -> Value {
        if obj.is_none() {
            Value::Null
        } else if let Ok(b) = obj.extract::<bool>() {
            Value::Bool(b)
        } else if let Ok(i) = obj.extract::<i64>() {
            Value::Number(i.into())
        } else if let Ok(f) = obj.extract::<f64>() {
            serde_json::Number::from_f64(f)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        } else if let Ok(s) = obj.extract::<String>() {
            Value::String(s)
        } else if let Ok(list) = obj.downcast::<pyo3::types::PyList>() {
            let arr: Vec<Value> = list.iter()
                .map(|item| Self::py_to_json(py, &item))
                .collect();
            Value::Array(arr)
        } else if let Ok(dict) = obj.downcast::<PyDict>() {
            let mut map = serde_json::Map::new();
            for (k, v) in dict.iter() {
                if let Ok(key) = k.extract::<String>() {
                    map.insert(key, Self::py_to_json(py, &v));
                }
            }
            Value::Object(map)
        } else {
            // Fallback: convert to string repr
            Value::String(obj.str().map(|s| s.to_string()).unwrap_or_default())
        }
    }

    /// Build the `zcode` Python module and inject it into the given globals dict
    fn inject_zcode_module(py: Python, globals: &Bound<PyDict>) -> PyResult<()> {
        let code = r#"
import subprocess, os

class _ZcodeApi:
    def read_file(self, path):
        with open(path, 'r', encoding='utf-8') as f:
            return f.read()

    def write_file(self, path, content):
        os.makedirs(os.path.dirname(path) or '.', exist_ok=True)
        with open(path, 'w', encoding='utf-8') as f:
            f.write(content)
        return True

    def shell(self, cmd):
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        return {'stdout': result.stdout, 'stderr': result.stderr, 'code': result.returncode}

    def log(self, msg):
        import sys
        print(f'[zcode.py] {msg}', file=sys.stderr)

zcode = _ZcodeApi()
"#;
        py.run_bound(code, Some(globals), None)?;
        Ok(())
    }
}

impl Default for PythonEngine {
    fn default() -> Self { Self::new() }
}

impl ScriptEngine for PythonEngine {
    fn name(&self) -> &str { "python" }
    fn extensions(&self) -> &[&str] { &[".py", ".pyw"] }

    fn eval(&self, code: &str, _ctx: &ScriptContext) -> Result<ScriptOutput> {
        Python::with_gil(|py| {
            let globals = PyDict::new_bound(py);
            Self::inject_zcode_module(py, &globals)
                .map_err(|e| ZcodeError::InternalError(format!("Python inject error: {}", e)))?;

            let result = py.eval_bound(code, Some(&globals), None)
                .map_err(|e| ZcodeError::InternalError(format!("Python eval error: {}", e)))?;

            Ok(ScriptOutput::success(Self::py_to_json(py, &result)))
        })
    }

    fn call_function(
        &self,
        script_path: &Path,
        function_name: &str,
        args: Value,
        _ctx: &ScriptContext,
    ) -> Result<ScriptOutput> {
        let script_code = std::fs::read_to_string(script_path)
            .map_err(|e| ZcodeError::IoError(e))?;

        Python::with_gil(|py| {
            let globals = PyDict::new_bound(py);
            Self::inject_zcode_module(py, &globals)
                .map_err(|e| ZcodeError::InternalError(e.to_string()))?;

            // Execute the script file
            py.run_bound(&script_code, Some(&globals), None)
                .map_err(|e| ZcodeError::InternalError(format!("Python script error: {}", e)))?;

            // Get the function
            let func = globals.get_item(function_name)
                .map_err(|e| ZcodeError::InternalError(e.to_string()))?
                .ok_or_else(|| ZcodeError::InternalError(
                    format!("Function '{}' not found in Python script", function_name)
                ))?;

            // Serialize args to JSON string and pass to function
            let args_str = serde_json::to_string(&args).unwrap_or("{}".to_string());
            let result = func.call((args_str,), None)
                .map_err(|e| ZcodeError::InternalError(format!("Python call error: {}", e)))?;

            Ok(ScriptOutput::success(Self::py_to_json(py, &result)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ScriptContext { ScriptContext::default() }

    #[test]
    fn test_python_engine_name() {
        assert_eq!(PythonEngine::new().name(), "python");
    }

    #[test]
    fn test_python_engine_extensions() {
        let e = PythonEngine::new();
        assert!(e.extensions().contains(&".py"));
        assert!(e.extensions().contains(&".pyw"));
    }

    #[test]
    fn test_python_eval_number() {
        let e = PythonEngine::new();
        let out = e.eval("42", &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value, serde_json::json!(42));
    }

    #[test]
    fn test_python_eval_string() {
        let e = PythonEngine::new();
        let out = e.eval(r#""hello from python""#, &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value, serde_json::json!("hello from python"));
    }

    #[test]
    fn test_python_eval_boolean() {
        let e = PythonEngine::new();
        let out = e.eval("True", &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value, serde_json::json!(true));
    }

    #[test]
    fn test_python_eval_none() {
        let e = PythonEngine::new();
        let out = e.eval("None", &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value, serde_json::json!(null));
    }

    #[test]
    fn test_python_eval_list() {
        let e = PythonEngine::new();
        let out = e.eval("[1, 2, 3]", &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn test_python_eval_dict() {
        let e = PythonEngine::new();
        let out = e.eval(r#"{"key": "value", "num": 42}"#, &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value["key"], "value");
        assert_eq!(out.value["num"], 42);
    }

    #[test]
    fn test_python_eval_arithmetic() {
        let e = PythonEngine::new();
        let out = e.eval("10 + 32", &ctx()).unwrap();
        assert_eq!(out.value, serde_json::json!(42));
    }

    #[test]
    fn test_python_zcode_api_available() {
        let e = PythonEngine::new();
        let out = e.eval("type(zcode).__name__", &ctx()).unwrap();
        assert!(out.success);
        assert_eq!(out.value, serde_json::json!("_ZcodeApi"));
    }

    #[test]
    fn test_python_shell() {
        let e = PythonEngine::new();
        let out = e.eval(
            r#"zcode.shell("echo hello_python")['stdout'].strip()"#,
            &ctx(),
        ).unwrap();
        assert!(out.success);
        assert_eq!(out.value, serde_json::json!("hello_python"));
    }

    #[test]
    fn test_python_read_write_file() {
        use tempfile::NamedTempFile;
        use std::io::Write;
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "test content").unwrap();
        let path = f.path().to_str().unwrap();

        let e = PythonEngine::new();
        let out = e.eval(
            &format!(r#"zcode.read_file("{}").strip()"#, path),
            &ctx()
        ).unwrap();
        assert!(out.success);
        assert_eq!(out.value, serde_json::json!("test content"));
    }

    #[test]
    fn test_python_syntax_error() {
        let e = PythonEngine::new();
        let result = e.eval("def broken( !!!!", &ctx());
        assert!(result.is_err());
    }

    #[test]
    fn test_call_function_from_file() {
        use tempfile::NamedTempFile;
        use std::io::Write;
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "import json\ndef process(args_json):\n    return 'processed: ' + args_json").unwrap();
        let e = PythonEngine::new();
        let out = e.call_function(f.path(), "process", serde_json::json!({"x": 1}), &ctx()).unwrap();
        assert!(out.success);
        assert!(out.value.as_str().unwrap().starts_with("processed:"));
    }
}
