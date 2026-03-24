//! Memory system for zcode
//!
//! Three-tier memory architecture:
//! - **Working Memory**: session-scoped in-memory (LRU file tracking, tool history)
//! - **Project Memory**: SQLite persistent (architecture decisions, code chunks)
//! - **Semantic Index**: TF-IDF vector search (code similarity)
//! - **Context Assembler**: Token budget manager for LLM context preparation

pub mod working;
pub mod project;
pub mod semantic;
pub mod context;

pub use working::{WorkingMemory, RecentFile, ToolExecution, TokenUsage};
pub use project::{ProjectMemory, MemoryEntry, CodeChunk};
pub use semantic::{SemanticIndex, SearchResult};
pub use context::{ContextAssembler, AssembledContext, TokenBudget, estimate_tokens};
