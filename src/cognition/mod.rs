//! Cognition Engine — Semantic search, code indexing, and knowledge management
//!
//! The Cognition Engine provides:
//! - **Vector-based semantic search** over code chunks
//! - **Smart code chunking** for Rust, Markdown, and other files
//! - **Session memory** for decisions and learned patterns
//! - **Knowledge management** from multiple sources
//! - **Persistent storage** via SQLite

pub mod engine;
pub mod embedding;
pub mod chunker;
pub mod vector;
pub mod storage;
pub mod memory;
pub mod knowledge;

pub use engine::{CognitionEngine, CognitionEngineImpl, IndexMode, IndexStats};
pub use embedding::{EmbeddingModel, MockEmbedding, cosine_similarity, vec_to_blob, blob_to_vec};
pub use chunker::{CodeChunker, CodeBlock, BlockKind};
pub use vector::{VectorIndex, SearchResult};
pub use storage::CognitionStorage;
pub use memory::{SessionMemory, DecisionRecord, LearnedPattern, MemoryRecall, MemoryType};
pub use knowledge::{KnowledgeSource, KnowledgeQuery, KnowledgeFragment, KnowledgeKind, KnowledgeContext};
