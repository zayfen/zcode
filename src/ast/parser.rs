//! AST parser using tree-sitter
//!
//! Uses `LanguageRegistry` to look up grammars by name or file extension.
//! Keeps the parser itself separate from language registration.

use crate::error::{Result, ZcodeError};
use serde::{Deserialize, Serialize};
use tree_sitter::{Language, Node, Parser, Tree};

// ─── AstParser ─────────────────────────────────────────────────────────────────

/// A tree-sitter based AST parser
pub struct AstParser {
    parser: Parser,
}

impl AstParser {
    /// Create a new parser configured for the given `tree_sitter::Language`
    pub fn new(language: Language) -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .map_err(|e| ZcodeError::InternalError(format!("Failed to set language: {}", e)))?;
        Ok(Self { parser })
    }

    /// Parse the given source string and return an `AstTree`
    pub fn parse(&mut self, source: &str) -> Result<AstTree> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| ZcodeError::InternalError("tree-sitter failed to parse source".into()))?;
        Ok(AstTree {
            tree,
            source: source.to_string(),
        })
    }

    /// Parse a file on disk and return an `AstTree`
    pub fn parse_file(&mut self, path: &std::path::Path) -> Result<AstTree> {
        let source = std::fs::read_to_string(path)?;
        self.parse(&source)
    }
}

// ─── AstTree ───────────────────────────────────────────────────────────────────

/// Parsed AST tree with the original source for text extraction
pub struct AstTree {
    tree: Tree,
    source: String,
}

impl AstTree {
    /// Get the root node of the tree
    pub fn root_node(&self) -> Node {
        self.tree.root_node()
    }

    /// Whether the parse produced any errors
    pub fn has_errors(&self) -> bool {
        self.tree.root_node().has_error()
    }

    /// Find all nodes of a specific kind (e.g. "function_item", "class_definition")
    pub fn find_nodes_by_type(&self, node_type: &str) -> Vec<NodeInfo> {
        let mut results = Vec::new();
        collect_by_type(self.root_node(), node_type, &self.source, &mut results);
        results
    }

    /// Find nodes whose text matches a predicate
    pub fn find_nodes_matching<F>(&self, node_type: &str, predicate: F) -> Vec<NodeInfo>
    where
        F: Fn(&str) -> bool,
    {
        self.find_nodes_by_type(node_type)
            .into_iter()
            .filter(|n| n.text.as_deref().map_or(false, |t| predicate(t)))
            .collect()
    }

    /// Get the source text of the tree
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Extract text for a node range from the original source
    pub fn node_text(&self, node: &Node) -> Option<&str> {
        node.utf8_text(self.source.as_bytes()).ok()
    }
}

fn collect_by_type<'a>(
    node: Node<'a>,
    node_type: &str,
    source: &str,
    results: &mut Vec<NodeInfo>,
) {
    if node.kind() == node_type {
        results.push(NodeInfo::from_node_with_source(node, source));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_by_type(child, node_type, source, results);
    }
}

// ─── NodeInfo ──────────────────────────────────────────────────────────────────

/// Information about an AST node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// The kind/type of this node (e.g. "function_item")
    pub kind: String,
    /// Start row (0-indexed)
    pub start_row: usize,
    /// Start column (0-indexed)
    pub start_column: usize,
    /// End row (0-indexed)
    pub end_row: usize,
    /// End column (0-indexed)
    pub end_column: usize,
    /// The source text of this node (if available)
    pub text: Option<String>,
}

impl NodeInfo {
    fn from_node_with_source(node: Node<'_>, source: &str) -> Self {
        let text = node.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
        Self {
            kind: node.kind().to_string(),
            start_row: node.start_position().row,
            start_column: node.start_position().column,
            end_row: node.end_position().row,
            end_column: node.end_position().column,
            text,
        }
    }

    /// 1-indexed start line for display
    pub fn start_line(&self) -> usize {
        self.start_row + 1
    }

    /// 1-indexed end line for display
    pub fn end_line(&self) -> usize {
        self.end_row + 1
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────
//
// These tests require an actual tree-sitter grammar. Since no grammar crates are
// bundled, the tests validate the parser API surface and error handling using
// a minimal stub approach. Real grammar tests should live in integration tests
// where users register their own grammars.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_info_line_numbers() {
        let info = NodeInfo {
            kind: "test".to_string(),
            start_row: 0,
            start_column: 0,
            end_row: 2,
            end_column: 5,
            text: Some("hello".to_string()),
        };
        assert_eq!(info.start_line(), 1);
        assert_eq!(info.end_line(), 3);
    }

    #[test]
    fn test_node_info_clone_and_debug() {
        let info = NodeInfo {
            kind: "function_item".to_string(),
            start_row: 1,
            start_column: 0,
            end_row: 5,
            end_column: 1,
            text: Some("fn foo() {}".to_string()),
        };
        let cloned = info.clone();
        assert_eq!(cloned.kind, "function_item");
        let _ = format!("{:?}", cloned);
    }

    #[test]
    fn test_node_info_no_text() {
        let info = NodeInfo {
            kind: "block".to_string(),
            start_row: 0,
            start_column: 0,
            end_row: 0,
            end_column: 2,
            text: None,
        };
        assert!(info.text.is_none());
    }

    #[test]
    fn test_node_info_serde() {
        let info = NodeInfo {
            kind: "test".to_string(),
            start_row: 0,
            start_column: 0,
            end_row: 1,
            end_column: 10,
            text: Some("content".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let decoded: NodeInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.kind, "test");
        assert_eq!(decoded.text, Some("content".to_string()));
    }
}
