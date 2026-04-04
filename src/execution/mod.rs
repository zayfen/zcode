//! Execution Engine — DAG-driven task execution with parallel execution, budgets, and checkpoints
//!
//! # Overview
//!
//! This module provides:
//! - **TaskGraph**: DAG-based task dependency management with topological sort
//! - **ExecutionBudget**: Resource limits (tokens, iterations, cost, time)
//! - **BudgetTracker**: Atomic real-time budget tracking
//! - **CheckpointPolicy**: Human-in-the-loop approval for high-risk operations
//! - **PlanVerifier**: Validates planner output before execution
//!
//! # Example
//!
//! ```rust,no_run
//! use zcode::execution::{TaskGraph, ExecutionBudget, TaskNode};
//!
//! let graph = TaskGraph::new();
//! // Add tasks and dependencies, then execute in topological order
//! ```

pub mod graph;
pub mod budget;
pub mod checkpoint;
pub mod plan_verify;

pub use graph::{TaskGraph, TaskNode, TaskId, TaskNodeStatus};
pub use budget::{ExecutionBudget, BudgetTracker, BudgetReport};
pub use checkpoint::{CheckpointPolicy, CheckpointMode, HighRiskPattern};
pub use plan_verify::{PlanVerifier, PlanVerificationResult, PlanIssue};
