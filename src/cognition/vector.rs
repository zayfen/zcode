//! In-memory vector index for semantic search
//!
//! Provides simple cosine similarity search over embeddings.
//! For production use, consider hnswlib or faiss for better performance.

use crate::cognition::embedding::cosine_similarity;
use serde::{Deserialize, Serialize};

/// A search result with relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Unique identifier for the result
    pub id: String,
    /// Similarity score (0-1, higher is better)
    pub score: f32,
    /// Content snippet
    pub content: String,
    /// File path
    pub path: String,
    /// Start line number
    pub start_line: usize,
    /// End line number
    pub end_line: usize,
}

/// A vector entry in the index
#[derive(Debug, Clone)]
struct VectorEntry {
    /// Auto-incrementing ID
    id: i64,
    /// File path
    path: String,
    /// Start line number
    start_line: usize,
    /// End line number
    pub end_line: usize,
    /// Optional identifier (function name, etc.)
    identifier: Option<String>,
    /// Embedding vector
    vector: Vec<f32>,
    /// Content for retrieval
    content: String,
}

/// In-memory vector index using cosine similarity
#[derive(Debug, Clone)]
pub struct VectorIndex {
    /// All indexed entries
    entries: Vec<VectorEntry>,
    /// Next ID to assign
    next_id: i64,
    /// Vector dimension
    dimension: usize,
}

impl VectorIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self::with_dimension(384)
    }

    /// Create with a specific dimension
    pub fn with_dimension(dimension: usize) -> Self {
        Self {
            entries: Vec::new(),
            next_id: 0,
            dimension,
        }
    }

    /// Add an entry to the index
    pub fn add_entry(
        &mut self,
        path: impl Into<String>,
        start_line: usize,
        end_line: usize,
        identifier: Option<String>,
        vector: Vec<f32>,
        content: impl Into<String>,
    ) -> i64 {
        let id = self.next_id;
        self.next_id += 1;

        self.entries.push(VectorEntry {
            id,
            path: path.into(),
            start_line,
            end_line,
            identifier,
            vector,
            content: content.into(),
        });

        id
    }

    /// Search for similar entries
    ///
    /// Returns up to `top_k` results sorted by similarity score.
    pub fn search(&self, query_vector: &[f32], top_k: usize) -> Vec<SearchResult> {
        if self.entries.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<SearchResult> = self
            .entries
            .iter()
            .map(|entry| {
                let score = cosine_similarity(query_vector, &entry.vector);
                SearchResult {
                    id: if let Some(ident) = &entry.identifier {
                        format!("{}:{}-{}:{}", entry.path, entry.start_line, entry.end_line, ident)
                    } else {
                        format!("{}:{}-{}", entry.path, entry.start_line, entry.end_line)
                    },
                    score,
                    content: entry.content.clone(),
                    path: entry.path.clone(),
                    start_line: entry.start_line,
                    end_line: entry.end_line,
                }
            })
            .filter(|r| r.score > 0.0) // Only include results with some similarity
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Return top_k
        results.truncate(top_k);
        results
    }

    /// Number of entries in the index
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries from the index
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_id = 0;
    }

    /// Get the dimension of vectors in this index
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Remove entries for a specific file
    pub fn remove_file(&mut self, path: &str) {
        self.entries.retain(|e| e.path != path);
    }
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vec(values: &[f32]) -> Vec<f32> {
        values.to_vec()
    }

    #[test]
    fn test_vector_index_new() {
        let idx = VectorIndex::new();
        assert_eq!(idx.len(), 0);
        assert!(idx.is_empty());
        assert_eq!(idx.dimension(), 384);
    }

    #[test]
    fn test_vector_index_with_dimension() {
        let idx = VectorIndex::with_dimension(128);
        assert_eq!(idx.dimension(), 128);
    }

    #[test]
    fn test_vector_index_add_entry() {
        let mut idx = VectorIndex::new();
        let vec = make_vec(&[1.0, 0.0, 0.0]);

        let id = idx.add_entry("test.rs", 1, 10, Some("foo".to_string()), vec.clone(), "content");
        assert_eq!(id, 0);
        assert_eq!(idx.len(), 1);
        assert!(!idx.is_empty());
    }

    #[test]
    fn test_vector_index_multiple_entries() {
        let mut idx = VectorIndex::new();

        idx.add_entry("a.rs", 1, 10, None, make_vec(&[1.0, 0.0]), "a");
        idx.add_entry("b.rs", 1, 10, None, make_vec(&[0.0, 1.0]), "b");

        assert_eq!(idx.len(), 2);
    }

    #[test]
    fn test_vector_index_search_identical() {
        let mut idx = VectorIndex::new();
        let vec = make_vec(&[1.0, 2.0, 3.0]);

        idx.add_entry("test.rs", 1, 10, Some("foo".to_string()), vec.clone(), "content");

        let results = idx.search(&vec, 5);
        assert_eq!(results.len(), 1);
        assert!((results[0].score - 1.0).abs() < f32::EPSILON);
        assert_eq!(results[0].path, "test.rs");
        assert_eq!(results[0].content, "content");
        assert_eq!(results[0].start_line, 1);
        assert_eq!(results[0].end_line, 10);
    }

    #[test]
    fn test_vector_index_search_orthogonal() {
        let mut idx = VectorIndex::new();

        idx.add_entry("a.rs", 1, 10, None, make_vec(&[1.0, 0.0, 0.0]), "a");
        idx.add_entry("b.rs", 1, 10, None, make_vec(&[0.0, 1.0, 0.0]), "b");

        let query = make_vec(&[1.0, 0.0, 0.0]);
        let results = idx.search(&query, 5);

        assert_eq!(results.len(), 1);
        assert!((results[0].score - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vector_index_search_top_k() {
        let mut idx = VectorIndex::new();

        // Add entries with varying similarity
        idx.add_entry("a.rs", 1, 10, None, make_vec(&[1.0, 0.0]), "a");
        idx.add_entry("b.rs", 1, 10, None, make_vec(&[0.9, 0.1]), "b");
        idx.add_entry("c.rs", 1, 10, None, make_vec(&[0.0, 1.0]), "c");

        let query = make_vec(&[1.0, 0.0]);
        let results = idx.search(&query, 2);

        assert_eq!(results.len(), 2);
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn test_vector_index_search_empty() {
        let idx = VectorIndex::new();
        let results = idx.search(&make_vec(&[1.0, 2.0]), 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_vector_index_clear() {
        let mut idx = VectorIndex::new();
        idx.add_entry("test.rs", 1, 10, None, make_vec(&[1.0]), "content");

        assert_eq!(idx.len(), 1);

        idx.clear();
        assert_eq!(idx.len(), 0);
        assert!(idx.is_empty());
    }

    #[test]
    fn test_vector_index_remove_file() {
        let mut idx = VectorIndex::new();

        idx.add_entry("a.rs", 1, 10, None, make_vec(&[1.0]), "a");
        idx.add_entry("b.rs", 1, 10, None, make_vec(&[2.0]), "b");
        idx.add_entry("a.rs", 11, 20, None, make_vec(&[3.0]), "a2");

        assert_eq!(idx.len(), 3);

        idx.remove_file("a.rs");
        assert_eq!(idx.len(), 1);
        assert_eq!(idx.entries[0].path, "b.rs");
    }

    #[test]
    fn test_search_result_id_with_identifier() {
        let mut idx = VectorIndex::new();
        idx.add_entry("test.rs", 5, 10, Some("foo".to_string()), make_vec(&[1.0]), "content");

        let results = idx.search(&make_vec(&[1.0]), 1);
        assert_eq!(results[0].id, "test.rs:5-10:foo");
    }

    #[test]
    fn test_search_result_id_without_identifier() {
        let mut idx = VectorIndex::new();
        idx.add_entry("test.rs", 5, 10, None, make_vec(&[1.0]), "content");

        let results = idx.search(&make_vec(&[1.0]), 1);
        assert_eq!(results[0].id, "test.rs:5-10");
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            id: "test:1-10:foo".to_string(),
            score: 0.95,
            content: "fn foo() {}".to_string(),
            path: "test.rs".to_string(),
            start_line: 1,
            end_line: 10,
        };

        let json = serde_json::to_string(&result).unwrap();
        let decoded: SearchResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.id, decoded.id);
        assert!((result.score - decoded.score).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vector_index_default() {
        let idx = VectorIndex::default();
        assert!(idx.is_empty());
    }
}
