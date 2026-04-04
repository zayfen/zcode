//! Embedding model abstraction
//!
//! Defines the interface for text embedding models and provides a mock
//! implementation for testing and development. Real implementations can
//! use fastembed, candle, or external APIs.

use crate::error::Result;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Embedding model dimension for mock implementation
pub const MOCK_DIMENSION: usize = 384;

/// Trait for text embedding models
pub trait EmbeddingModel: Send + Sync {
    /// Returns the dimension of vectors produced by this model
    fn dimension(&self) -> usize;

    /// Generate a single embedding vector from text
    fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate multiple embeddings in batch
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// Model identifier (e.g., "BAAI/bge-small-en-v1.5")
    fn model_id(&self) -> &str;
}

/// Mock embedding implementation for testing
///
/// Uses a deterministic hash-based pseudo-embedding that produces
/// consistent vectors for the same text. Not semantically meaningful,
/// but allows compilation and basic testing of the pipeline.
#[derive(Debug, Clone)]
pub struct MockEmbedding {
    dimension: usize,
    model_id: String,
}

impl MockEmbedding {
    /// Create a new mock embedding model
    pub fn new() -> Self {
        Self {
            dimension: MOCK_DIMENSION,
            model_id: "mock/v1".to_string(),
        }
    }

    /// Create with custom dimension
    pub fn with_dimension(dimension: usize) -> Self {
        Self {
            dimension,
            model_id: format!("mock/v1-dim{}", dimension),
        }
    }

    /// Generate pseudo-embedding from text using hash
    fn hash_to_vector(&self, text: &str) -> Vec<f32> {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();

        let mut vec = Vec::with_capacity(self.dimension);
        for i in 0..self.dimension {
            // Simple deterministic pseudo-random values in [-1, 1]
            let val = ((seed.wrapping_mul(i as u64 + 1)) % 2000) as f32 / 1000.0 - 1.0;
            vec.push(val);
        }
        vec
    }
}

impl Default for MockEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingModel for MockEmbedding {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        Ok(self.hash_to_vector(text))
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }
}

/// Calculate cosine similarity between two vectors
///
/// Returns 1.0 for identical vectors, 0.0 for orthogonal vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let norm_a = norm_a.sqrt();
    let norm_b = norm_b.sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

/// Convert vector to byte blob for SQLite storage
pub fn vec_to_blob(vec: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(vec.len() * 4);
    for &v in vec {
        blob.extend_from_slice(&v.to_le_bytes());
    }
    blob
}

/// Convert byte blob back to vector
pub fn blob_to_vec(blob: &[u8], dim: usize) -> Vec<f32> {
    let mut vec = Vec::with_capacity(dim);
    for chunk in blob.chunks_exact(4) {
        if let Ok(bytes) = chunk.try_into() {
            vec.push(f32::from_le_bytes(bytes));
        }
    }
    vec.truncate(dim);
    vec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_embedding_dimension() {
        let model = MockEmbedding::new();
        assert_eq!(model.dimension(), MOCK_DIMENSION);
    }

    #[test]
    fn test_mock_embedding_custom_dimension() {
        let model = MockEmbedding::with_dimension(128);
        assert_eq!(model.dimension(), 128);
        assert_eq!(model.model_id(), "mock/v1-dim128");
    }

    #[test]
    fn test_mock_embedding_consistent() {
        let model = MockEmbedding::new();
        let text = "test text";
        let v1 = model.embed(text).unwrap();
        let v2 = model.embed(text).unwrap();
        assert_eq!(v1.len(), MOCK_DIMENSION);
        assert_eq!(v1, v2, "Same text should produce same vector");
    }

    #[test]
    fn test_mock_embedding_different_text() {
        let model = MockEmbedding::new();
        let v1 = model.embed("text one").unwrap();
        let v2 = model.embed("text two").unwrap();
        assert_ne!(v1, v2, "Different text should produce different vectors");
    }

    #[test]
    fn test_mock_embedding_batch() {
        let model = MockEmbedding::new();
        let texts = vec!["one", "two", "three"];
        let results = model.embed_batch(&texts).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].len(), MOCK_DIMENSION);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![0.0, 1.0, 0.0];
        assert_eq!(cosine_similarity(&v1, &v2), 0.0);
    }

    #[test]
    fn test_cosine_similarity_parallel() {
        let v1 = vec![1.0, 2.0, 3.0];
        let v2 = vec![2.0, 4.0, 6.0]; // v1 * 2
        assert!((cosine_similarity(&v1, &v2) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_cosine_similarity_different_length() {
        let v1 = vec![1.0, 2.0];
        let v2 = vec![1.0, 2.0, 3.0];
        assert_eq!(cosine_similarity(&v1, &v2), 0.0);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let v1 = vec![0.0, 0.0, 0.0];
        let v2 = vec![1.0, 2.0, 3.0];
        assert_eq!(cosine_similarity(&v1, &v2), 0.0);
    }

    #[test]
    fn test_vec_to_blob_roundtrip() {
        let original = vec![1.0, -2.5, 3.14, 0.0];
        let blob = vec_to_blob(&original);
        let recovered = blob_to_vec(&blob, original.len());
        assert_eq!(recovered.len(), original.len());
        for (i, (&o, &r)) in original.iter().zip(recovered.iter()).enumerate() {
            assert!((o - r).abs() < f32::EPSILON, "Mismatch at index {}", i);
        }
    }

    #[test]
    fn test_blob_to_vec_truncates() {
        let blob = vec_to_blob(&vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let truncated = blob_to_vec(&blob, 3);
        assert_eq!(truncated.len(), 3);
        assert_eq!(truncated[2], 3.0);
    }

    #[test]
    fn test_blob_to_vec_empty() {
        let vec = blob_to_vec(&[], 10);
        assert!(vec.is_empty());
    }

    #[test]
    fn test_model_id() {
        let model = MockEmbedding::new();
        assert_eq!(model.model_id(), "mock/v1");
    }
}
