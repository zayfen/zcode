//! CLI argument definitions for zcode
//!
//! This module defines the command-line interface using clap.

use clap::{Parser, Subcommand};

/// Zcode - A programming agent CLI tool
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Model to use for LLM interactions
    #[arg(short, long, global = true)]
    pub model: Option<String>,

    /// MCP servers to connect to (can be specified multiple times)
    #[arg(short = 'M', long, global = true)]
    pub mcp: Vec<String>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

/// Available subcommands
#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Run a single task in non-interactive mode
    Run {
        /// The task description to execute
        task: String,
    },

    /// Start interactive chat mode (default)
    Chat,

    /// Show version information
    Version,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_default() {
        let args = Args::try_parse_from(["zcode"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(args.command.is_none());
        assert!(!args.verbose);
        assert!(args.model.is_none());
        assert!(args.mcp.is_empty());
    }

    #[test]
    fn test_run_command_parsing() {
        let args = Args::try_parse_from(["zcode", "run", "test task"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        if let Some(Command::Run { task }) = args.command {
            assert_eq!(task, "test task");
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_chat_command_parsing() {
        let args = Args::try_parse_from(["zcode", "chat"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(matches!(args.command, Some(Command::Chat)));
    }

    #[test]
    fn test_version_command_parsing() {
        let args = Args::try_parse_from(["zcode", "version"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(matches!(args.command, Some(Command::Version)));
    }

    #[test]
    fn test_model_flag_parsing() {
        let args = Args::try_parse_from(["zcode", "--model", "claude-3-opus"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert_eq!(args.model, Some("claude-3-opus".to_string()));
    }

    #[test]
    fn test_mcp_flag_parsing() {
        let args = Args::try_parse_from(["zcode", "--mcp", "server1", "--mcp", "server2"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert_eq!(args.mcp, vec!["server1", "server2"]);
    }

    #[test]
    fn test_verbose_flag_parsing() {
        let args = Args::try_parse_from(["zcode", "-v"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(args.verbose);
    }
}
