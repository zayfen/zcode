//! CLI module for zcode
//!
//! This module provides the command-line interface for the zcode programming agent.
//!
//! # Architecture
//!
//! The CLI is structured into two main components:
//!
//! - **args**: Command-line argument definitions using clap
//! - **commands**: Command handlers that execute the requested operations
//!
//! # Usage
//!
//! ```rust,no_run
//! use zcode::cli::args::Args;
//! use clap::Parser;
//!
//! let args = Args::parse();
//! ```

pub mod args;
pub mod commands;

pub use args::{Args, Command};
pub use commands::{execute_command, execute_default};
