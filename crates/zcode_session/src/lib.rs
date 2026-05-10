//! Session management layer.

pub mod session;
pub mod store;

pub use session::{FileSnapshot, Snapshot, SnapshotDetail, SnapshotManager};
pub use store::{CompressionConfig, Session, SessionManager};
