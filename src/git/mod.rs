//! Git integration module

pub mod diff;

pub use diff::{ChangedFile, DiffContext, FileStatus, GitDiff};
