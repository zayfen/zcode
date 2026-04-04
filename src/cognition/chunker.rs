//! Code chunking for semantic indexing
//!
//! Splits code files into semantic chunks (functions, structs, modules)
//! for better embedding and retrieval.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Default maximum chunk size in characters
pub const DEFAULT_MAX_CHUNK_SIZE: usize = 1500;

/// Default overlap size between chunks
pub const DEFAULT_OVERLAP_SIZE: usize = 100;

/// Kind of code block
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockKind {
    /// Function definition
    Function,
    /// Method definition
    Method,
    /// Struct definition
    Struct,
    /// Enum definition
    Enum,
    /// Trait definition
    Trait,
    /// Impl block
    Impl,
    /// Module declaration
    Module,
    /// Test function
    Test,
    /// Config file (JSON, TOML, YAML)
    Config,
    /// Documentation comment
    DocComment,
    /// Markdown document
    Markdown,
    /// Unknown/other
    Unknown,
}

/// A chunk of code with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    /// File path
    pub path: String,
    /// Start line number (1-indexed)
    pub start_line: usize,
    /// End line number (1-indexed, inclusive)
    pub end_line: usize,
    /// Content of the block
    pub content: String,
    /// Kind of block
    pub kind: BlockKind,
    /// Optional identifier (function name, struct name, etc.)
    pub identifier: Option<String>,
}

impl CodeBlock {
    /// Create a new code block
    pub fn new(
        path: impl Into<String>,
        start_line: usize,
        end_line: usize,
        content: impl Into<String>,
        kind: BlockKind,
    ) -> Self {
        Self {
            path: path.into(),
            start_line,
            end_line,
            content: content.into(),
            kind,
            identifier: None,
        }
    }

    /// Set the identifier
    pub fn with_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.identifier = Some(identifier.into());
        self
    }

    /// Get a unique ID for this block
    pub fn id(&self) -> String {
        if let Some(ident) = &self.identifier {
            format!("{}:{}-{}:{}", self.path, self.start_line, self.end_line, ident)
        } else {
            format!("{}:{}-{}", self.path, self.start_line, self.end_line)
        }
    }

    /// Estimated token count (1 token ≈ 4 chars)
    pub fn estimated_tokens(&self) -> usize {
        self.content.len().div_ceil(4)
    }
}

/// Code chunker for splitting files into semantic blocks
#[derive(Debug, Clone)]
pub struct CodeChunker {
    /// Maximum chunk size in characters
    pub max_chunk_size: usize,
    /// Overlap size between chunks
    pub overlap_size: usize,
}

impl Default for CodeChunker {
    fn default() -> Self {
        Self {
            max_chunk_size: DEFAULT_MAX_CHUNK_SIZE,
            overlap_size: DEFAULT_OVERLAP_SIZE,
        }
    }
}

impl CodeChunker {
    /// Create a new chunker with custom settings
    pub fn new(max_chunk_size: usize, overlap_size: usize) -> Self {
        Self {
            max_chunk_size,
            overlap_size,
        }
    }

    /// Chunk a single file into blocks
    pub fn chunk_file(&self, path: &Path, content: &str) -> Vec<CodeBlock> {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension {
            "rs" => self.chunk_rust_file(path, content),
            "md" => self.chunk_markdown_file(path, content),
            "toml" | "yaml" | "yml" | "json" => self.chunk_config_file(path, content),
            _ => self.chunk_generic_file(path, content),
        }
    }

    /// Chunk a Rust source file
    fn chunk_rust_file(&self, path: &Path, content: &str) -> Vec<CodeBlock> {
        let mut blocks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // Skip empty lines and simple comments
            if line.is_empty() || line.starts_with("//") && !line.starts_with("///") {
                i += 1;
                continue;
            }

            // Detect block type and find its extent
            let (kind, identifier, end_idx) = self.detect_rust_block(&lines, i);

            if end_idx > i {
                let block_content = lines[i..=end_idx].join("\n");
                let block = CodeBlock::new(
                    path.display().to_string(),
                    i + 1,
                    end_idx + 1,
                    block_content,
                    kind,
                )
                .with_identifier(identifier.unwrap_or_else(|| format!("line_{}", i + 1)));

                blocks.push(block);
                i = end_idx + 1;
            } else {
                i += 1;
            }
        }

        // If no blocks found, fall back to generic chunking
        if blocks.is_empty() {
            self.chunk_generic_file(path, content)
        } else {
            blocks
        }
    }

    /// Detect the type and extent of a Rust code block
    fn detect_rust_block(&self, lines: &[&str], start: usize) -> (BlockKind, Option<String>, usize) {
        let line = lines[start].trim();

        // Check for various block types
        if line.starts_with("pub fn ") || line.starts_with("fn ") {
            let ident = extract_identifier(line);
            let end = self.find_block_end(lines, start, '{', '}');
            (BlockKind::Function, ident, end)
        } else if line.starts_with("pub struct ") || line.starts_with("struct ") {
            let ident = extract_identifier(line);
            let end = self.find_block_end(lines, start, '{', '}');
            (BlockKind::Struct, ident, end)
        } else if line.starts_with("pub enum ") || line.starts_with("enum ") {
            let ident = extract_identifier(line);
            let end = self.find_block_end(lines, start, '{', '}');
            (BlockKind::Enum, ident, end)
        } else if line.starts_with("pub trait ") || line.starts_with("trait ") {
            let ident = extract_identifier(line);
            let end = self.find_block_end(lines, start, '{', '}');
            (BlockKind::Trait, ident, end)
        } else if line.starts_with("impl ") {
            let ident = extract_identifier(line);
            let end = self.find_block_end(lines, start, '{', '}');
            (BlockKind::Impl, ident, end)
        } else if line.starts_with("pub mod ") || line.starts_with("mod ") {
            let ident = extract_identifier(line);
            // Module declarations usually end with ;
            let end = self.find_line_end(lines, start);
            (BlockKind::Module, ident, end)
        } else if line.starts_with("#[cfg(test)") || line.starts_with("#[test]") {
            let ident = Some("test".to_string());
            let end = self.find_block_end(lines, start, '{', '}');
            (BlockKind::Test, ident, end)
        } else if line.starts_with("///") || line.starts_with("//!") {
            let ident = None;
            let mut end = start;
            while end < lines.len() && (lines[end].trim().starts_with("///") || lines[end].trim().starts_with("//!")) {
                end += 1;
            }
            (BlockKind::DocComment, ident, end.saturating_sub(1))
        } else {
            (BlockKind::Unknown, None, start)
        }
    }

    /// Find the end of a brace-delimited block
    fn find_block_end(&self, lines: &[&str], start: usize, open: char, close: char) -> usize {
        let mut depth = 0i32;
        let mut end = start;

        for line in lines.iter().skip(start) {
            for ch in line.chars() {
                if ch == open {
                    depth += 1;
                } else if ch == close {
                    depth -= 1;
                }
            }
            end += 1;
            if depth == 0 && end > start {
                return end - 1;
            }
        }

        lines.len().saturating_sub(1)
    }

    /// Find the end of a line (for statements ending with ;)
    fn find_line_end(&self, _lines: &[&str], start: usize) -> usize {
        start
    }

    /// Chunk a Markdown file by headers
    fn chunk_markdown_file(&self, path: &Path, content: &str) -> Vec<CodeBlock> {
        let mut blocks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut start = 0;
        let mut current_header = String::from("root");

        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("##") {
                // Save previous section
                if i > start {
                    let block_content = lines[start..i].join("\n");
                    let block = CodeBlock::new(
                        path.display().to_string(),
                        start + 1,
                        i,
                        block_content,
                        BlockKind::Markdown,
                    )
                    .with_identifier(current_header.clone());
                    blocks.push(block);
                }

                // Extract new header
                current_header = line.trim_start_matches('#').trim().to_string();
                start = i;
            }
        }

        // Add last section
        if start < lines.len() {
            let block_content = lines[start..].join("\n");
            let block = CodeBlock::new(
                path.display().to_string(),
                start + 1,
                lines.len(),
                block_content,
                BlockKind::Markdown,
            )
            .with_identifier(current_header);
            blocks.push(block);
        }

        blocks
    }

    /// Chunk a config file (treat as single block)
    fn chunk_config_file(&self, path: &Path, content: &str) -> Vec<CodeBlock> {
        vec![CodeBlock::new(
            path.display().to_string(),
            1,
            content.lines().count(),
            content,
            BlockKind::Config,
        )]
    }

    /// Generic chunking by character count with overlap
    fn chunk_generic_file(&self, path: &Path, content: &str) -> Vec<CodeBlock> {
        let mut blocks = Vec::new();
        let chars: Vec<char> = content.chars().collect();
        let mut start = 0;

        while start < chars.len() {
            let end = (start + self.max_chunk_size).min(chars.len());

            // Try to break at a newline
            let mut break_pos = end;
            while break_pos > start && chars[break_pos - 1] != '\n' {
                break_pos -= 1;
            }

            let actual_end = if break_pos > start { break_pos } else { end };

            let block_content: String = chars[start..actual_end].iter().collect();
            let start_line = content[..start].lines().count() + 1;
            let end_line = content[..actual_end].lines().count();

            let block = CodeBlock::new(
                path.display().to_string(),
                start_line,
                end_line,
                block_content,
                BlockKind::Unknown,
            )
            .with_identifier(format!("chunk_{}", blocks.len() + 1));

            blocks.push(block);

            start = actual_end.saturating_sub(self.overlap_size);
        }

        blocks
    }

    /// Chunk all files in a directory
    pub fn chunk_directory(
        &self,
        root: &Path,
        exclude_patterns: &[&str],
    ) -> Result<Vec<CodeBlock>> {
        use walkdir::WalkDir;

        let mut all_blocks = Vec::new();

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();

            // Check exclusions
            if exclude_patterns.iter().any(|pat| {
                path.to_string_lossy().contains(pat) || path.extension().map_or(false, |e| e.to_str() == Some(*pat))
            }) {
                continue;
            }

            // Skip certain file types
            if let Some(ext) = path.extension() {
                match ext.to_str().unwrap_or("") {
                    "png" | "jpg" | "jpeg" | "gif" | "ico" | "pdf" | "zip" | "tar" | "gz" => continue,
                    _ => {}
                }
            }

            if let Ok(content) = std::fs::read_to_string(path) {
                let blocks = self.chunk_file(path, &content);
                all_blocks.extend(blocks);
            }
        }

        Ok(all_blocks)
    }
}

/// Extract identifier from a line (e.g., "fn foo(" -> "foo")
fn extract_identifier(line: &str) -> Option<String> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    for token in tokens {
        // Remove common trailing characters
        let cleaned = token.trim_end_matches('(').trim_end_matches('{').trim_end_matches(':');
        if cleaned.contains(|c: char| c.is_alphanumeric() || c == '_') {
            return Some(cleaned.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_code_block_creation() {
        let block = CodeBlock::new("test.rs", 1, 10, "content", BlockKind::Function);
        assert_eq!(block.path, "test.rs");
        assert_eq!(block.start_line, 1);
        assert_eq!(block.end_line, 10);
        assert_eq!(block.content, "content");
        assert_eq!(block.kind, BlockKind::Function);
        assert!(block.identifier.is_none());
    }

    #[test]
    fn test_code_block_with_identifier() {
        let block = CodeBlock::new("test.rs", 1, 10, "content", BlockKind::Function)
            .with_identifier("foo");
        assert_eq!(block.identifier, Some("foo".to_string()));
    }

    #[test]
    fn test_code_block_id() {
        let block = CodeBlock::new("test.rs", 5, 10, "content", BlockKind::Function)
            .with_identifier("foo");
        assert_eq!(block.id(), "test.rs:5-10:foo");

        let block_no_ident = CodeBlock::new("test.rs", 5, 10, "content", BlockKind::Function);
        assert_eq!(block_no_ident.id(), "test.rs:5-10");
    }

    #[test]
    fn test_code_block_estimated_tokens() {
        let block = CodeBlock::new("test.rs", 1, 1, "fn main() {}", BlockKind::Function);
        // "fn main() {}" = 12 chars / 4 = 3 tokens
        assert_eq!(block.estimated_tokens(), 3);
    }

    #[test]
    fn test_chunker_default() {
        let chunker = CodeChunker::default();
        assert_eq!(chunker.max_chunk_size, DEFAULT_MAX_CHUNK_SIZE);
        assert_eq!(chunker.overlap_size, DEFAULT_OVERLAP_SIZE);
    }

    #[test]
    fn test_chunker_custom() {
        let chunker = CodeChunker::new(500, 50);
        assert_eq!(chunker.max_chunk_size, 500);
        assert_eq!(chunker.overlap_size, 50);
    }

    #[test]
    fn test_chunk_rust_file() {
        let chunker = CodeChunker::default();
        let content = r#"
pub fn hello() {
    println!("hello");
}

pub struct Foo {
    x: i32,
}

/// This is a test
/// with multiple lines
"#;
        let path = PathBuf::from("test.rs");
        let blocks = chunker.chunk_file(&path, content);

        assert!(!blocks.is_empty());
        assert!(blocks.iter().any(|b| b.kind == BlockKind::Function));
        assert!(blocks.iter().any(|b| b.kind == BlockKind::Struct));
        assert!(blocks.iter().any(|b| b.kind == BlockKind::DocComment));
    }

    #[test]
    fn test_chunk_markdown_file() {
        let chunker = CodeChunker::default();
        let content = r#"
# Introduction

This is the intro.

## Section 1

Content here.

## Section 2

More content.
"#;
        let path = PathBuf::from("test.md");
        let blocks = chunker.chunk_file(&path, content);

        assert!(!blocks.is_empty());
        assert!(blocks.iter().all(|b| b.kind == BlockKind::Markdown));
    }

    #[test]
    fn test_chunk_config_file() {
        let chunker = CodeChunker::default();
        let content = r#"
[package]
name = "test"
version = "0.1.0"
"#;
        let path = PathBuf::from("Cargo.toml");
        let blocks = chunker.chunk_file(&path, content);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].kind, BlockKind::Config);
    }

    #[test]
    fn test_chunk_generic_file() {
        let chunker = CodeChunker::new(50, 10);
        let content = "a".repeat(200); // 200 chars
        let path = PathBuf::from("test.txt");
        let blocks = chunker.chunk_file(&path, &content);

        // Should split into multiple chunks with overlap
        assert!(blocks.len() > 1);
    }

    #[test]
    fn test_extract_identifier() {
        assert_eq!(extract_identifier("fn foo()"), Some("foo".to_string()));
        assert_eq!(extract_identifier("pub struct Bar {"), Some("Bar".to_string()));
        assert_eq!(extract_identifier("impl Baz"), Some("Baz".to_string()));
        assert_eq!(extract_identifier("mod test;"), Some("test".to_string()));
    }

    #[test]
    fn test_block_kind_equality() {
        assert_eq!(BlockKind::Function, BlockKind::Function);
        assert_ne!(BlockKind::Function, BlockKind::Struct);
    }

    #[test]
    fn test_block_kind_serialization() {
        let kind = BlockKind::Function;
        let json = serde_json::to_string(&kind).unwrap();
        let decoded: BlockKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, decoded);
    }
}
