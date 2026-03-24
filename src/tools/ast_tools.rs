//! AST-based tools for zcode
//!
//! These tools leverage the `LanguageRegistry` + `AstParser` to provide
//! structural code search and editing capabilities.

use crate::ast::{AstParser, LanguageRegistry, NodeInfo};
use crate::error::ZcodeError;
use crate::tools::{Tool, ToolResult};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

// ─── AstSearchTool ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AstSearchInput {
    /// File path to analyse
    path: String,
    /// Tree-sitter node type to find (e.g. "function_item", "class_definition")
    node_type: String,
    /// Optional text filter: only return nodes whose text contains this string
    #[serde(default)]
    text_contains: Option<String>,
}

/// Search for AST nodes of a given type inside a file.
///
/// The tool uses the shared `LanguageRegistry` to detect the language from the
/// file extension and then parses the file with tree-sitter.
pub struct AstSearchTool {
    registry: Arc<LanguageRegistry>,
}

impl AstSearchTool {
    pub fn new(registry: Arc<LanguageRegistry>) -> Self {
        Self { registry }
    }
}

impl Tool for AstSearchTool {
    fn name(&self) -> &str {
        "ast_search"
    }

    fn description(&self) -> &str {
        "Search for AST nodes of a specific type in a source file. \
         Requires a grammar to be registered in the LanguageRegistry for the file's extension."
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let params: AstSearchInput = serde_json::from_value(input)
            .map_err(|e| ZcodeError::InvalidToolInput(e.to_string()))?;

        let path = Path::new(&params.path);

        if !path.exists() {
            return Err(ZcodeError::FileNotFound {
                path: params.path.clone(),
            });
        }

        // Look up language from registry using file extension
        let language = self
            .registry
            .from_path(path)
            .ok_or_else(|| ZcodeError::InvalidToolInput(format!(
                "No grammar registered for file: {}. \
                 Register a LanguageProvider for this extension first.",
                params.path
            )))?;

        let mut parser = AstParser::new(language)?;
        let tree = parser.parse_file(path)?;

        let mut nodes = tree.find_nodes_by_type(&params.node_type);

        // Apply optional text filter
        if let Some(ref filter) = params.text_contains {
            nodes.retain(|n| n.text.as_deref().map_or(false, |t| t.contains(filter.as_str())));
        }

        let count = nodes.len();
        let json_nodes: Vec<Value> = nodes.iter().map(node_info_to_json).collect();

        Ok(serde_json::json!({
            "file": params.path,
            "node_type": params.node_type,
            "count": count,
            "nodes": json_nodes,
            "has_errors": tree.has_errors()
        }))
    }
}

// ─── AstEditTool ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AstEditInput {
    /// File to edit
    path: String,
    /// The exact source text of the node to replace
    old_text: String,
    /// The replacement text
    new_text: String,
    /// Whether to replace all occurrences (default: false = first only)
    #[serde(default)]
    replace_all: bool,
}

/// Edit a source file by replacing the text of AST node(s).
///
/// This is a text-level edit guided by exact source text matching — it does not
/// structurally validate the AST. Use `AstSearchTool` first to find exact texts.
pub struct AstEditTool {
    registry: Arc<LanguageRegistry>,
}

impl AstEditTool {
    pub fn new(registry: Arc<LanguageRegistry>) -> Self {
        Self { registry }
    }
}

impl Tool for AstEditTool {
    fn name(&self) -> &str {
        "ast_edit"
    }

    fn description(&self) -> &str {
        "Edit a source file by replacing exact AST node text. \
         Parses the file first to validate syntax, then performs precise text replacement."
    }

    fn execute(&self, input: Value) -> ToolResult<Value> {
        let params: AstEditInput = serde_json::from_value(input)
            .map_err(|e| ZcodeError::InvalidToolInput(e.to_string()))?;

        let path = Path::new(&params.path);

        if !path.exists() {
            return Err(ZcodeError::FileNotFound {
                path: params.path.clone(),
            });
        }

        // Validate file can be parsed by trying to look up the language
        let language = self
            .registry
            .from_path(path)
            .ok_or_else(|| ZcodeError::InvalidToolInput(format!(
                "No grammar registered for file: {}",
                params.path
            )))?;

        // Read and parse to validate syntax before editing
        let source = std::fs::read_to_string(path)?;
        let mut parser = AstParser::new(language)?;
        let tree = parser.parse(&source)?;

        if tree.has_errors() {
            tracing::warn!("File {} has parse errors — edit may be imprecise", params.path);
        }

        // Perform text replacement
        let (new_source, replacements) = if params.replace_all {
            let count = source.matches(&params.old_text).count();
            (source.replace(&params.old_text, &params.new_text), count)
        } else {
            let new_source = source.replacen(&params.old_text, &params.new_text, 1);
            let count = if new_source != source { 1 } else { 0 };
            (new_source, count)
        };

        if replacements == 0 {
            return Err(ZcodeError::ToolExecutionFailed {
                name: "ast_edit".to_string(),
                message: format!("Text '{}' not found in file", params.old_text),
            });
        }

        std::fs::write(path, &new_source)?;

        Ok(serde_json::json!({
            "success": true,
            "path": params.path,
            "replacements": replacements,
            "had_parse_errors": tree.has_errors()
        }))
    }
}

// ─── Helpers ───────────────────────────────────────────────────────────────────

fn node_info_to_json(node: &NodeInfo) -> Value {
    serde_json::json!({
        "kind": node.kind,
        "start_line": node.start_line(),
        "start_column": node.start_column,
        "end_line": node.end_line(),
        "end_column": node.end_column,
        "text": node.text
    })
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn empty_registry() -> Arc<LanguageRegistry> {
        Arc::new(LanguageRegistry::new())
    }

    // ── AstSearchTool ──

    #[test]
    fn test_ast_search_no_grammar_registered() {
        let tool = AstSearchTool::new(empty_registry());
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, "fn main() {}").unwrap();

        let result = tool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "node_type": "function_item"
        }));

        assert!(result.is_err());
        // Should fail because no grammar is registered
        match result.unwrap_err() {
            ZcodeError::InvalidToolInput(msg) => {
                assert!(msg.contains("No grammar registered"));
            }
            _ => panic!("Expected InvalidToolInput"),
        }
    }

    #[test]
    fn test_ast_search_nonexistent_file() {
        let tool = AstSearchTool::new(empty_registry());
        let result = tool.execute(serde_json::json!({
            "path": "/nonexistent/file.rs",
            "node_type": "function_item"
        }));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ZcodeError::FileNotFound { .. }));
    }

    #[test]
    fn test_ast_search_invalid_input() {
        let tool = AstSearchTool::new(empty_registry());
        let result = tool.execute(serde_json::json!({}));
        assert!(result.is_err());
    }

    // ── AstEditTool ──

    #[test]
    fn test_ast_edit_no_grammar_registered() {
        let tool = AstEditTool::new(empty_registry());
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, "fn old_name() {}").unwrap();

        let result = tool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "old_text": "old_name",
            "new_text": "new_name"
        }));

        assert!(result.is_err());
        match result.unwrap_err() {
            ZcodeError::InvalidToolInput(msg) => {
                assert!(msg.contains("No grammar registered"));
            }
            _ => panic!("Expected InvalidToolInput"),
        }
    }

    #[test]
    fn test_ast_edit_nonexistent_file() {
        let tool = AstEditTool::new(empty_registry());
        let result = tool.execute(serde_json::json!({
            "path": "/nonexistent/file.rs",
            "old_text": "foo",
            "new_text": "bar"
        }));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ZcodeError::FileNotFound { .. }));
    }

    #[test]
    fn test_ast_edit_invalid_input() {
        let tool = AstEditTool::new(empty_registry());
        let result = tool.execute(serde_json::json!({}));
        assert!(result.is_err());
    }

    // ── Tool trait interface ──

    #[test]
    fn test_ast_search_name_and_description() {
        let tool = AstSearchTool::new(empty_registry());
        assert_eq!(tool.name(), "ast_search");
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_ast_edit_name_and_description() {
        let tool = AstEditTool::new(empty_registry());
        assert_eq!(tool.name(), "ast_edit");
        assert!(!tool.description().is_empty());
    }
}
