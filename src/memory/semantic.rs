//! Semantic Index — TF-IDF + cosine similarity
//!
//! Lightweight semantic search without ML dependencies.
//! Can be swapped for fastembed in the future.

use std::collections::HashMap;

// ─── SearchResult ──────────────────────────────────────────────────────────────

/// A single semantic search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Identifier for this chunk (e.g. file path + line range)
    pub id: String,
    /// Cosine similarity score [0.0, 1.0]
    pub score: f32,
    /// The stored text
    pub text: String,
}

// ─── TF-IDF Embedding ──────────────────────────────────────────────────────────

/// Tokenize text into lowercase words (ASCII alphanumeric)
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && w.len() > 1)
        .map(|w| w.to_lowercase())
        .collect()
}

/// Compute term frequency for a token sequence
fn term_frequency(tokens: &[String]) -> HashMap<String, f32> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for token in tokens {
        *counts.entry(token.clone()).or_default() += 1;
    }
    let total = tokens.len().max(1) as f32;
    counts
        .into_iter()
        .map(|(k, v)| (k, v as f32 / total))
        .collect()
}

/// Compute IDF weights from a collection of documents
fn compute_idf(documents: &[Vec<String>]) -> HashMap<String, f32> {
    let n = documents.len() as f32;
    let mut doc_freq: HashMap<String, usize> = HashMap::new();
    for doc in documents {
        let unique_terms: std::collections::HashSet<_> = doc.iter().collect();
        for term in unique_terms {
            *doc_freq.entry(term.clone()).or_default() += 1;
        }
    }
    doc_freq
        .into_iter()
        .map(|(term, df)| (term, (n / (df as f32 + 1.0)).ln() + 1.0))
        .collect()
}

/// Compute TF-IDF vector for a token list given IDF weights
fn tfidf_vector(tokens: &[String], idf: &HashMap<String, f32>) -> HashMap<String, f32> {
    let tf = term_frequency(tokens);
    tf.into_iter()
        .map(|(term, tf)| {
            let idf_weight = idf.get(&term).copied().unwrap_or(1.0);
            (term, tf * idf_weight)
        })
        .collect()
}

/// Cosine similarity between two sparse TF-IDF vectors
fn cosine_similarity(a: &HashMap<String, f32>, b: &HashMap<String, f32>) -> f32 {
    let dot: f32 = a
        .iter()
        .filter_map(|(term, &w)| b.get(term).map(|&bw| w * bw))
        .sum();

    let norm_a: f32 = a.values().map(|v| v * v).sum::<f32>().sqrt();
    let norm_b: f32 = b.values().map(|v| v * v).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        (dot / (norm_a * norm_b)).clamp(0.0, 1.0)
    }
}

// ─── SemanticIndex ─────────────────────────────────────────────────────────────

/// Indexed document chunk
#[derive(Debug)]
struct Chunk {
    id: String,
    text: String,
    tokens: Vec<String>,
}

/// In-memory semantic index using TF-IDF
pub struct SemanticIndex {
    chunks: Vec<Chunk>,
    /// IDF weights (recomputed on re-index)
    idf: HashMap<String, f32>,
    /// Whether the index needs recomputing
    dirty: bool,
}

impl SemanticIndex {
    /// Create an empty index
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            idf: HashMap::new(),
            dirty: false,
        }
    }

    /// Add or update a text chunk in the index
    pub fn index_chunk(&mut self, id: impl Into<String>, text: impl Into<String>) {
        let id = id.into();
        let text = text.into();
        let tokens = tokenize(&text);

        // Replace existing chunk with same ID
        if let Some(pos) = self.chunks.iter().position(|c| c.id == id) {
            self.chunks[pos] = Chunk { id, text, tokens };
        } else {
            self.chunks.push(Chunk { id, text, tokens });
        }
        self.dirty = true;
    }

    /// Remove a chunk from the index
    pub fn remove_chunk(&mut self, id: &str) -> bool {
        let before = self.chunks.len();
        self.chunks.retain(|c| c.id != id);
        let removed = self.chunks.len() < before;
        if removed {
            self.dirty = true;
        }
        removed
    }

    /// Re-compute IDF weights (called lazily before search)
    fn rebuild_idf(&mut self) {
        let all_tokens: Vec<Vec<String>> = self.chunks.iter().map(|c| c.tokens.clone()).collect();
        self.idf = compute_idf(&all_tokens);
        self.dirty = false;
    }

    /// Search for the top-k most similar chunks to a query string
    pub fn search(&mut self, query: &str, top_k: usize) -> Vec<SearchResult> {
        if self.chunks.is_empty() {
            return Vec::new();
        }

        if self.dirty {
            self.rebuild_idf();
        }

        let query_tokens = tokenize(query);
        let query_vec = tfidf_vector(&query_tokens, &self.idf);

        let mut scored: Vec<(f32, &Chunk)> = self
            .chunks
            .iter()
            .map(|chunk| {
                let chunk_vec = tfidf_vector(&chunk.tokens, &self.idf);
                let score = cosine_similarity(&query_vec, &chunk_vec);
                (score, chunk)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(top_k)
            .filter(|(score, _)| *score > 0.0)
            .map(|(score, chunk)| SearchResult {
                id: chunk.id.clone(),
                score,
                text: chunk.text.clone(),
            })
            .collect()
    }

    /// Number of indexed chunks
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Whether the index is empty
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Clear all indexed chunks
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.idf.clear();
        self.dirty = false;
    }

    /// Compute an embedding vector for a text string (public API for external use)
    pub fn embed(&mut self, text: &str) -> Vec<f32> {
        if self.dirty || self.idf.is_empty() {
            self.rebuild_idf();
        }
        let tokens = tokenize(text);
        let vec = tfidf_vector(&tokens, &self.idf);
        // Return sorted by key for determinism
        let mut keys: Vec<&String> = vec.keys().collect();
        keys.sort();
        keys.iter().map(|k| vec[*k]).collect()
    }
}

impl Default for SemanticIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Hello World, this is a test!");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        // Single chars filtered out ('a')
        assert!(!tokens.contains(&"a".to_string()));
    }

    #[test]
    fn test_tokenize_empty() {
        assert!(tokenize("").is_empty());
    }

    #[test]
    fn test_term_frequency() {
        let tokens = tokenize("the cat sat on the mat");
        let tf = term_frequency(&tokens);
        // "the" appears twice out of 6 tokens
        assert!(*tf.get("the").unwrap() > 0.0);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let mut a = HashMap::new();
        a.insert("foo".to_string(), 1.0_f32);
        a.insert("bar".to_string(), 0.5_f32);
        let score = cosine_similarity(&a, &a);
        assert!((score - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_zero() {
        let mut a = HashMap::new();
        a.insert("foo".to_string(), 1.0_f32);
        let mut b = HashMap::new();
        b.insert("bar".to_string(), 1.0_f32);
        let score = cosine_similarity(&a, &b);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_index_empty() {
        let mut idx = SemanticIndex::new();
        assert!(idx.is_empty());
        let results = idx.search("anything", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_index_and_search() {
        let mut idx = SemanticIndex::new();
        idx.index_chunk("doc1", "Rust programming language systems code");
        idx.index_chunk("doc2", "Python machine learning data science");
        idx.index_chunk("doc3", "Rust async tokio web server");

        let results = idx.search("Rust programming", 3);
        assert!(!results.is_empty());
        // Top result should be Rust-related
        assert!(results[0].id == "doc1" || results[0].id == "doc3");
    }

    #[test]
    fn test_search_returns_top_k() {
        let mut idx = SemanticIndex::new();
        for i in 0..10 {
            idx.index_chunk(format!("doc{}", i), format!("document number {} content", i));
        }
        let results = idx.search("document content", 3);
        assert!(results.len() <= 3);
    }

    #[test]
    fn test_search_scores_ordered() {
        let mut idx = SemanticIndex::new();
        idx.index_chunk("exact", "rust systems programming");
        idx.index_chunk("partial", "rust web framework");
        idx.index_chunk("unrelated", "python data analysis machine learning");

        let results = idx.search("rust systems programming", 3);
        assert!(!results.is_empty());
        // Exact match should score highest
        assert_eq!(results[0].id, "exact");
        // Scores should be decreasing
        for i in 1..results.len() {
            assert!(results[i - 1].score >= results[i].score);
        }
    }

    #[test]
    fn test_update_chunk() {
        let mut idx = SemanticIndex::new();
        idx.index_chunk("doc1", "original content");
        idx.index_chunk("doc1", "updated content rust");
        assert_eq!(idx.len(), 1); // No duplicates
        let results = idx.search("rust", 1);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_remove_chunk() {
        let mut idx = SemanticIndex::new();
        idx.index_chunk("doc1", "content to remove");
        idx.index_chunk("doc2", "content to keep");
        assert!(idx.remove_chunk("doc1"));
        assert_eq!(idx.len(), 1);
        assert!(!idx.remove_chunk("nonexistent"));
    }

    #[test]
    fn test_clear() {
        let mut idx = SemanticIndex::new();
        idx.index_chunk("doc1", "content");
        idx.clear();
        assert!(idx.is_empty());
    }

    #[test]
    fn test_embed_returns_vector() {
        let mut idx = SemanticIndex::new();
        idx.index_chunk("doc1", "rust programming systems");
        idx.index_chunk("doc2", "python machine learning");
        let v = idx.embed("rust programming");
        assert!(!v.is_empty());
    }
}
