//! Cognition engine — orchestrates semantic search and knowledge management
//!
//! The main entry point for the Cognition system.

use crate::cognition::{
    chunker::CodeChunker,
    embedding::EmbeddingModel,
    knowledge::{KnowledgeContext, KnowledgeFragment, KnowledgeQuery, KnowledgeSource, KnowledgeKind},
    memory::{MemoryRecall, MemoryType, SessionMemory},
    storage::CognitionStorage,
    vector::SearchResult as VectorSearchResult,
    vector::VectorIndex,
};
use crate::error::{Result, ZcodeError};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::Instant;

/// Index mode for project indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexMode {
    /// Full re-index of all files
    Full,
    /// Incremental index with changed files
    Incremental { changed_files: Vec<String> },
}

/// Statistics from an indexing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Number of files indexed
    pub files_indexed: usize,
    /// Number of chunks created
    pub chunks_created: usize,
    /// Duration in milliseconds
    pub duration_ms: u128,
}

/// Main cognition engine trait
pub trait CognitionEngine {
    /// Search for relevant code
    fn search(&self, query: &str, top_k: usize) -> Result<Vec<VectorSearchResult>>;

    /// Index the project
    fn index_project(&mut self, mode: IndexMode) -> Result<IndexStats>;

    /// Assemble knowledge from multiple sources
    fn assemble_knowledge(&self, query: &KnowledgeQuery) -> Result<KnowledgeContext>;

    /// Store a session memory
    fn store_session_memory(&mut self, memory: &SessionMemory) -> Result<()>;

    /// Recall relevant memories
    fn recall_memories(&self, query: &str, limit: usize) -> Result<Vec<MemoryRecall>>;
}

/// Default implementation of the CognitionEngine
pub struct CognitionEngineImpl<E>
where
    E: EmbeddingModel,
{
    /// Embedding model for vector generation
    embedding_model: E,
    /// In-memory vector index
    vector_index: VectorIndex,
    /// Persistent storage
    storage: Option<CognitionStorage>,
    /// Code chunker
    chunker: CodeChunker,
    /// Knowledge sources
    knowledge_sources: Vec<Box<dyn KnowledgeSource>>,
}

impl<E> CognitionEngineImpl<E>
where
    E: EmbeddingModel,
{
    /// Create a new cognition engine
    pub fn new(embedding_model: E) -> Self {
        let dimension = embedding_model.dimension();
        Self {
            embedding_model,
            vector_index: VectorIndex::with_dimension(dimension),
            storage: None,
            chunker: CodeChunker::default(),
            knowledge_sources: Vec::new(),
        }
    }

    /// Create with persistent storage
    pub fn with_storage(mut self, storage: CognitionStorage) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Add a knowledge source
    pub fn add_knowledge_source(mut self, source: Box<dyn KnowledgeSource>) -> Self {
        self.knowledge_sources.push(source);
        self
    }

    /// Set the chunker configuration
    pub fn with_chunker(mut self, chunker: CodeChunker) -> Self {
        self.chunker = chunker;
        self
    }

    /// Get the vector index
    pub fn vector_index(&self) -> &VectorIndex {
        &self.vector_index
    }

    /// Get mutable vector index
    pub fn vector_index_mut(&mut self) -> &mut VectorIndex {
        &mut self.vector_index
    }

    /// Get the storage
    pub fn storage(&self) -> Option<&CognitionStorage> {
        self.storage.as_ref()
    }
}

impl<E> CognitionEngine for CognitionEngineImpl<E>
where
    E: EmbeddingModel,
{
    fn search(&self, query: &str, top_k: usize) -> Result<Vec<VectorSearchResult>> {
        // Generate embedding for query
        let query_vector = self.embedding_model.embed(query)?;

        // Search vector index
        Ok(self.vector_index.search(&query_vector, top_k))
    }

    fn index_project(&mut self, mode: IndexMode) -> Result<IndexStats> {
        let start = Instant::now();

        // Determine files to index
        let files_to_index: Vec<String> = match mode {
            IndexMode::Full => {
                // For now, index current directory
                vec![".".to_string()]
            }
            IndexMode::Incremental { changed_files } => {
                // Filter files that actually changed
                if let Some(storage) = &self.storage {
                    changed_files
                        .into_iter()
                        .filter(|path| {
                            if let Ok(content) = std::fs::read_to_string(path) {
                                // Simple hash using std::collections::hash_map::DefaultHasher
                                let mut hasher = DefaultHasher::new();
                                content.hash(&mut hasher);
                                let hash = format!("{:x}", hasher.finish());
                                storage.get_file_hash(path).unwrap_or(None) != Some(hash)
                            } else {
                                false
                            }
                        })
                        .collect()
                } else {
                    changed_files
                }
            }
        };

        // Chunk files
        let mut total_chunks = 0;
        for root in &files_to_index {
            let path = Path::new(root);
            let blocks = self.chunker.chunk_directory(path, &["target", "node_modules", ".git"])?;

            for block in blocks {
                // Generate embedding
                let vector = self.embedding_model.embed(&block.content)?;

                // Store in persistent storage if available
                if let Some(storage) = &self.storage {
                    let embedding_blob = Some(crate::cognition::embedding::vec_to_blob(&vector));
                    storage.store_block(
                        &block.path,
                        block.start_line,
                        block.end_line,
                        block.identifier.as_deref(),
                        &block.content,
                        &format!("{:?}", block.kind),
                        embedding_blob.as_deref(),
                    )?;
                }

                // Store in vector index (after storage to avoid move)
                self.vector_index.add_entry(
                    &block.path,
                    block.start_line,
                    block.end_line,
                    block.identifier.clone(),
                    vector,
                    &block.content,
                );

                total_chunks += 1;
            }
        }

        let duration = start.elapsed();

        Ok(IndexStats {
            files_indexed: files_to_index.len(),
            chunks_created: total_chunks,
            duration_ms: duration.as_millis(),
        })
    }

    fn assemble_knowledge(&self, query: &KnowledgeQuery) -> Result<KnowledgeContext> {
        let mut all_fragments = Vec::new();

        // Note: In production, this would be async. For Phase 1, we use
        // synchronous polling or a simple approach.
        // For now, we'll skip async knowledge sources since they require a runtime.
        // This is a known limitation of the Phase 1 implementation.

        // Add code search results if relevant
        if query.kind == KnowledgeKind::CodeExample || query.kind == KnowledgeKind::ProjectContext {
            let code_results = self.search(&query.requirement, 5)?;
            for result in code_results {
                if result.score > 0.5 {
                    all_fragments.push(KnowledgeFragment::new(
                        format!("code:{}", result.path),
                        result.content,
                        result.score,
                        KnowledgeKind::CodeExample,
                    ));
                }
            }
        }

        // Sort by relevance and apply token budget
        all_fragments.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        Ok(KnowledgeContext::from_fragments(all_fragments, Some(10000)))
    }

    fn store_session_memory(&mut self, memory: &SessionMemory) -> Result<()> {
        let storage = self.storage.as_ref().ok_or_else(|| {
            ZcodeError::InternalError("No storage configured".to_string())
        })?;

        // Store session memory
        let memory_id = storage.store_session_memory(&memory.session_id, &memory.summary)?;

        // Store decisions with embeddings
        for decision in &memory.decisions {
            let embedding_blob = decision.embedding.as_ref()
                .map(|v| crate::cognition::embedding::vec_to_blob(v))
                .or_else(|| {
                    let text = format!("{} {}", decision.description, decision.rationale);
                    self.embedding_model.embed(&text)
                        .ok()
                        .map(|v| crate::cognition::embedding::vec_to_blob(&v))
                });

            let context_files = if decision.context_files.is_empty() {
                None
            } else {
                Some(decision.context_files.join(","))
            };

            storage.store_decision(
                memory_id,
                &decision.description,
                &decision.rationale,
                context_files.as_deref(),
                embedding_blob.as_deref(),
            )?;
        }

        // Store learned patterns with embeddings
        for pattern in &memory.learned_patterns {
            let embedding_blob = pattern.embedding.as_ref()
                .map(|v| crate::cognition::embedding::vec_to_blob(v))
                .or_else(|| {
                    let text = format!("{} {}", pattern.name, pattern.description);
                    self.embedding_model.embed(&text)
                        .ok()
                        .map(|v| crate::cognition::embedding::vec_to_blob(&v))
                });

            let examples = if pattern.examples.is_empty() {
                None
            } else {
                Some(pattern.examples.join("\n\n"))
            };

            storage.store_learned_pattern(
                memory_id,
                &pattern.name,
                &pattern.description,
                examples.as_deref(),
                embedding_blob.as_deref(),
            )?;
        }

        Ok(())
    }

    fn recall_memories(&self, query: &str, limit: usize) -> Result<Vec<MemoryRecall>> {
        let storage = self.storage.as_ref().ok_or_else(|| {
            ZcodeError::InternalError("No storage configured".to_string())
        })?;

        // Note: In production, would use the query embedding for semantic search
        let _query_vector = self.embedding_model.embed(query)?;

        // Load session memories
        let memories = storage.recall_memories(limit)?;
        let mut recalls = Vec::new();

        for memory in memories {
            // Simple keyword matching for now
            // In production, would use proper semantic search over stored embeddings
            let relevance = if memory.summary.to_lowercase().contains(&query.to_lowercase()) {
                0.8
            } else {
                0.3
            };

            recalls.push(MemoryRecall::new(
                MemoryType::SessionSummary,
                memory.summary,
                relevance,
            ).with_session(memory.session_id));
        }

        // Sort by relevance
        recalls.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        recalls.truncate(limit);

        Ok(recalls)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::embedding::MockEmbedding;

    #[test]
    fn test_engine_new() {
        let model = MockEmbedding::new();
        let engine = CognitionEngineImpl::new(model);
        assert!(engine.vector_index().is_empty());
    }

    #[test]
    fn test_engine_search_empty() {
        let model = MockEmbedding::new();
        let engine = CognitionEngineImpl::new(model);

        let results = engine.search("test query", 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_engine_search_with_content() {
        let model = MockEmbedding::new();
        let mut engine = CognitionEngineImpl::new(model);

        // Add some content
        let vec = engine.vector_index_mut();
        vec.add_entry("test.rs", 1, 10, Some("foo"), vec![1.0, 0.0], "content");

        // Search
        let results = engine.search("test", 5).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "test.rs");
    }

    #[test]
    fn test_engine_assemble_knowledge_empty() {
        let model = MockEmbedding::new();
        let engine = CognitionEngineImpl::new(model);

        let query = KnowledgeQuery::new("test", KnowledgeKind::CodeExample);
        let context = engine.assemble_knowledge(&query).unwrap();

        assert!(context.is_empty());
    }

    #[test]
    fn test_index_mode_serialization() {
        let mode = IndexMode::Full;
        let json = serde_json::to_string(&mode).unwrap();
        let decoded: IndexMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, decoded);
    }

    #[test]
    fn test_index_stats_serialization() {
        let stats = IndexStats {
            files_indexed: 10,
            chunks_created: 100,
            duration_ms: 1000,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let decoded: IndexStats = serde_json::from_str(&json).unwrap();
        assert_eq!(stats.files_indexed, decoded.files_indexed);
    }

    #[test]
    fn test_engine_with_chunker() {
        let model = MockEmbedding::new();
        let chunker = CodeChunker::new(100, 10);
        let engine = CognitionEngineImpl::new(model).with_chunker(chunker);

        // Just verify it compiles
        assert!(engine.vector_index().is_empty());
    }

    #[test]
    fn test_engine_with_storage() {
        let model = MockEmbedding::new();
        let storage = CognitionStorage::in_memory().unwrap();
        let engine = CognitionEngineImpl::new(model).with_storage(storage);

        assert!(engine.storage().is_some());
    }

    #[test]
    fn test_engine_store_session_memory_without_storage() {
        let model = MockEmbedding::new();
        let mut engine = CognitionEngineImpl::new(model);

        let memory = SessionMemory::new("session-1", "Test session");
        let result = engine.store_session_memory(&memory);

        assert!(result.is_err());
    }

    #[test]
    fn test_engine_recall_memories_without_storage() {
        let model = MockEmbedding::new();
        let engine = CognitionEngineImpl::new(model);

        let result = engine.recall_memories("test", 10);
        assert!(result.is_err());
    }
}
