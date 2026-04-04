//! Delivery pipeline module — automated delivery from verified tasks to PRs
//!
//! # Overview
//!
//! This module implements the delivery pipeline as described in the delivery pipeline design doc.
//! It provides:
//!
//! - **DeliveryPipeline**: Main orchestrator that runs gate checks, generates changelogs,
//!   creates branches, pushes, creates PRs, and monitors CI
//! - **DeliveryConfig**: Configuration for the delivery pipeline
//! - **ChangelogGenerator**: Generates markdown changelogs from task records
//! - **PullRequestCreator**: Creates PRs via platform CLIs (`gh`, `glab`, etc.)
//! - **CiMonitor**: Polls CI status with configurable timeout
//! - **GateChecker**: Runs pre-delivery gate checks
//!
//! # Example
//!
//! ```rust,no_run
//! use zcode::delivery::{DeliveryPipeline, DeliveryConfig, DeliveryContext};
//! use zcode::git::DiffContext;
//! use zcode::delivery::changelog::TaskRecord;
//! use std::path::PathBuf;
//!
//! # async fn example() {
//! let config = DeliveryConfig::default();
//! let pipeline = DeliveryPipeline::new(config);
//!
//! let ctx = DeliveryContext {
//!     tasks: vec![TaskRecord {
//!         task: "Add auth module".into(),
//!         final_score: Some(90.0),
//!         status: "completed".into(),
//!     }],
//!     commits: vec![],
//!     diff: DiffContext::default(),
//!     scores: vec![("auth".into(), 90.0)],
//!     project_root: PathBuf::from("/my/project"),
//!     branch_name: None,
//!     commit_message: None,
//! };
//!
//! let result = pipeline.deliver(&ctx).await;
//! # }
//! ```

pub mod config;
pub mod changelog;
pub mod version;
pub mod pull_request;
pub mod ci_monitor;
pub mod gate;
pub mod pipeline;

// Re-exports
pub use config::{DeliveryConfig, GitPlatform, CiConfig, CiPlatform, GateCheck, GateCheckType};
pub use changelog::{ChangelogGenerator, TaskRecord, ChangeEntry, ChangeCategory};
pub use version::{VersionManager, BumpType};
pub use pull_request::{PullRequestCreator, PrOptions, PrResult};
pub use ci_monitor::{CiMonitor, CiStatus};
pub use gate::{GateChecker, GateResult};
pub use pipeline::{DeliveryPipeline, DeliveryResult, DeliveryContext};
