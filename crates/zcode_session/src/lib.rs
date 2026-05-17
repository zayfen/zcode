//! Session management layer.

pub mod context;
pub mod intent;
pub mod lance_index;
pub mod session;
pub mod store;

pub use context::{ContextIntent, ContextPolicy, SelectedContext, OPTIONAL_CONTEXT_GUARD};
pub use intent::{HashedIntentVectorizer, IntentVector, IntentVectorIndex};
pub use session::{FileSnapshot, Snapshot, SnapshotDetail, SnapshotManager};
pub use store::{
    CompressionConfig, MatchedSessionTurn, Session, SessionContext, SessionContextConfig,
    SessionManager, SessionTurn,
};
