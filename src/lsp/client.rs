//! LSP (Language Server Protocol) client
//!
//! Connects to a language server via stdio, supports:
//! goto-definition, find-references, hover, completion

use crate::error::{Result, ZcodeError};
use lsp_types::{Location, Range};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

/// Convert a file path to a `file://` URI string
fn path_to_uri(path: &Path) -> Result<String> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| ZcodeError::InternalError(e.to_string()))?
            .join(path)
    };
    Ok(format!("file://{}", abs.to_string_lossy()))
}

// ─── ID generation ─────────────────────────────────────────────────────────────

static NEXT_ID: AtomicI32 = AtomicI32::new(1);
fn next_id() -> i32 { NEXT_ID.fetch_add(1, Ordering::Relaxed) }

// ─── LSP Message ──────────────────────────────────────────────────────────────

fn make_request(id: i32, method: &str, params: Value) -> String {
    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    });
    let body = msg.to_string();
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body)
}

fn make_notification(method: &str, params: Value) -> String {
    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    });
    let body = msg.to_string();
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body)
}

// ─── LspConnection ─────────────────────────────────────────────────────────────

struct LspConnection {
    stdin: ChildStdin,
    reader: BufReader<Box<dyn std::io::Read + Send>>,
    _child: Child,
}

impl LspConnection {
    fn new(command: &str, args: &[&str], root: &Path) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| ZcodeError::InternalError(
                format!("Failed to start LSP server '{}': {}", command, e)
            ))?;

        let stdin = child.stdin.take().ok_or_else(|| {
            ZcodeError::InternalError("Failed to get LSP stdin".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ZcodeError::InternalError("Failed to get LSP stdout".to_string())
        })?;

        Ok(Self {
            stdin,
            reader: BufReader::new(Box::new(stdout)),
            _child: child,
        })
    }

    fn send(&mut self, message: &str) -> Result<()> {
        self.stdin.write_all(message.as_bytes())
            .map_err(|e| ZcodeError::InternalError(format!("LSP write error: {}", e)))?;
        self.stdin.flush()
            .map_err(|e| ZcodeError::InternalError(format!("LSP flush error: {}", e)))?;
        Ok(())
    }

    fn read_response(&mut self) -> Result<Value> {
        // Read headers
        let mut content_length: usize = 0;
        loop {
            let mut line = String::new();
            self.reader.read_line(&mut line)
                .map_err(|e| ZcodeError::InternalError(format!("LSP read error: {}", e)))?;
            let line = line.trim();
            if line.is_empty() { break; }
            if let Some(rest) = line.strip_prefix("Content-Length:") {
                content_length = rest.trim().parse().unwrap_or(0);
            }
        }

        // Read body
        let mut body = vec![0u8; content_length];
        use std::io::Read;
        self.reader.read_exact(&mut body)
            .map_err(|e| ZcodeError::InternalError(format!("LSP body read error: {}", e)))?;

        serde_json::from_slice(&body)
            .map_err(|e| ZcodeError::InternalError(format!("LSP parse error: {}", e)))
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = next_id();
        let msg = make_request(id, method, params);
        self.send(&msg)?;

        // Read responses until we get one matching our id
        // (notifications with different ids may arrive in between)
        loop {
            let resp = self.read_response()?;
            if resp.get("id").and_then(|v| v.as_i64()) == Some(id as i64) {
                if let Some(err) = resp.get("error") {
                    return Err(ZcodeError::InternalError(
                        format!("LSP error: {}", err)
                    ));
                }
                return Ok(resp["result"].clone());
            }
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        let msg = make_notification(method, params);
        self.send(&msg)
    }
}

// ─── HoverResult ──────────────────────────────────────────────────────────────

/// Result of an LSP hover request
#[derive(Debug, Clone)]
pub struct HoverResult {
    pub contents: String,
    pub range: Option<Range>,
}

// ─── LspClient ─────────────────────────────────────────────────────────────────

/// Client for a Language Server Protocol server
pub struct LspClient {
    conn: Mutex<LspConnection>,
    root_uri: String,
}

impl LspClient {
    /// Start an LSP server and initialize it
    pub fn start(
        command: &str,
        args: &[&str],
        workspace_root: &Path,
    ) -> Result<Self> {
        let conn = LspConnection::new(command, args, workspace_root)?;
        let root_uri = path_to_uri(workspace_root)?;

        let mut client = Self {
            conn: Mutex::new(conn),
            root_uri: root_uri.clone(),
        };

        client.initialize(&root_uri)?;
        Ok(client)
    }

    fn initialize(&mut self, root_uri: &str) -> Result<()> {
        let params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "definition": { "dynamicRegistration": false },
                    "references": { "dynamicRegistration": false },
                    "hover": {
                        "dynamicRegistration": false,
                        "contentFormat": ["plaintext", "markdown"]
                    },
                    "completion": {
                        "dynamicRegistration": false,
                        "completionItem": { "snippetSupport": false }
                    }
                },
                "workspace": {}
            },
            "initializationOptions": null
        });

        let mut conn = self.conn.lock().unwrap();
        conn.request("initialize", params)?;
        conn.notify("initialized", serde_json::json!({}))?;
        Ok(())
    }

    /// Get the workspace root URI used during initialization
    pub fn root_uri(&self) -> &str {
        &self.root_uri
    }

    /// Open a document in the LSP server
    pub fn open_document(&self, file_path: &Path, content: &str) -> Result<()> {
        let uri = path_to_uri(file_path)?;
        let lang_id = Self::language_id(file_path);
        let mut conn = self.conn.lock().unwrap();
        conn.notify("textDocument/didOpen", serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": lang_id,
                "version": 1,
                "text": content
            }
        }))
    }

    /// Go to definition
    pub fn goto_definition(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<Location>> {
        let uri = path_to_uri(file_path)?;
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });
        let mut conn = self.conn.lock().unwrap();
        let result = conn.request("textDocument/definition", params)?;
        drop(conn);
        Self::parse_locations(result)
    }

    /// Find all references
    pub fn find_references(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<Vec<Location>> {
        let uri = path_to_uri(file_path)?;
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": include_declaration }
        });
        let mut conn = self.conn.lock().unwrap();
        let result = conn.request("textDocument/references", params)?;
        drop(conn);
        Self::parse_locations(result)
    }

    /// Hover information at position
    pub fn hover(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Option<HoverResult>> {
        let uri = path_to_uri(file_path)?;
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });
        let mut conn = self.conn.lock().unwrap();
        let result = conn.request("textDocument/hover", params)?;
        drop(conn);
        if result.is_null() { return Ok(None); }
        let contents = result.get("contents").map(|c| {
            if let Some(s) = c.as_str() { s.to_string() }
            else if let Some(obj) = c.as_object() {
                obj.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string()
            } else { c.to_string() }
        }).unwrap_or_default();
        Ok(Some(HoverResult { contents, range: None }))
    }

    /// Get completion items at position
    pub fn completion(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<String>> {
        let uri = path_to_uri(file_path)?;
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "triggerKind": 1 }
        });
        let mut conn = self.conn.lock().unwrap();
        let result = conn.request("textDocument/completion", params)?;
        drop(conn);
        let items = if let Some(list) = result.get("items") {
            list.as_array().cloned().unwrap_or_default()
        } else if result.is_array() {
            result.as_array().cloned().unwrap_or_default()
        } else { vec![] };
        let labels = items.iter()
            .filter_map(|item| item.get("label")?.as_str().map(str::to_string))
            .collect();
        Ok(labels)
    }

    // ─── Helpers ─────────────────────────────────────────────────────────────

    fn language_id(path: &Path) -> &'static str {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => "rust",
            Some("py") => "python",
            Some("js") | Some("mjs") => "javascript",
            Some("ts") => "typescript",
            Some("go") => "go",
            Some("lua") => "lua",
            Some("sh") | Some("bash") => "shellscript",
            Some("toml") => "toml",
            Some("json") => "json",
            Some("yaml") | Some("yml") => "yaml",
            _ => "plaintext",
        }
    }

    fn parse_locations(result: Value) -> Result<Vec<Location>> {
        if result.is_null() {
            return Ok(vec![]);
        }
        let locs: Vec<Location> = if result.is_array() {
            serde_json::from_value(result).unwrap_or_default()
        } else {
            // Single location
            serde_json::from_value(result)
                .map(|l| vec![l])
                .unwrap_or_default()
        };
        Ok(locs)
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_request_format() {
        let msg = make_request(1, "initialize", serde_json::json!({}));
        assert!(msg.starts_with("Content-Length:"));
        assert!(msg.contains("\"method\":\"initialize\""));
        assert!(msg.contains("\"id\":1"));
    }

    #[test]
    fn test_make_notification_format() {
        let msg = make_notification("initialized", serde_json::json!({}));
        assert!(msg.starts_with("Content-Length:"));
        assert!(msg.contains("\"method\":\"initialized\""));
        assert!(!msg.contains("\"id\":")); // notifications have no id
    }

    #[test]
    fn test_next_id_increments() {
        let id1 = next_id();
        let id2 = next_id();
        assert!(id2 > id1);
    }

    #[test]
    fn test_language_id_rust() {
        assert_eq!(LspClient::language_id(Path::new("main.rs")), "rust");
    }

    #[test]
    fn test_language_id_python() {
        assert_eq!(LspClient::language_id(Path::new("script.py")), "python");
    }

    #[test]
    fn test_language_id_javascript() {
        assert_eq!(LspClient::language_id(Path::new("app.js")), "javascript");
    }

    #[test]
    fn test_language_id_typescript() {
        assert_eq!(LspClient::language_id(Path::new("index.ts")), "typescript");
    }

    #[test]
    fn test_language_id_go() {
        assert_eq!(LspClient::language_id(Path::new("main.go")), "go");
    }

    #[test]
    fn test_language_id_shell() {
        assert_eq!(LspClient::language_id(Path::new("deploy.sh")), "shellscript");
    }

    #[test]
    fn test_language_id_unknown() {
        assert_eq!(LspClient::language_id(Path::new("file.xyz")), "plaintext");
    }

    #[test]
    fn test_parse_locations_null() {
        let result = LspClient::parse_locations(serde_json::json!(null)).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_locations_empty_array() {
        let result = LspClient::parse_locations(serde_json::json!([])).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_locations_array() {
        let result = LspClient::parse_locations(serde_json::json!([
            {
                "uri": "file:///tmp/test.rs",
                "range": {
                    "start": { "line": 5, "character": 0 },
                    "end": { "line": 5, "character": 10 }
                }
            }
        ])).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].range.start.line, 5);
    }

    #[test]
    fn test_lsp_client_start_invalid_command() {
        let result = LspClient::start(
            "/nonexistent/lsp/binary",
            &[],
            Path::new("/tmp"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_content_length_correct() {
        let params = serde_json::json!({ "key": "value" });
        let msg = make_request(42, "test/method", params);
        let parts: Vec<&str> = msg.splitn(2, "\r\n\r\n").collect();
        let header = parts[0];
        let body = parts[1];
        let declared_len: usize = header
            .split("Content-Length:")
            .nth(1)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_eq!(declared_len, body.len());
    }
}
