//! Requirements and documentation standardization layer.

pub mod docs;
pub mod task_store;

pub use docs::{generate_docs_scaffold, DocsError, DocsValidationResult, DocsValidator};
pub use task_store::{TaskRecord, TaskStatus, TaskStore};

