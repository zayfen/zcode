//! CLI argument definitions for zcode
//!
//! This module defines the command-line interface using clap.

use clap::{Parser, Subcommand};

/// Zcode - A programming agent CLI tool
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Skip the Harness Engineering docs validation check
    #[arg(long, global = true, help = "Skip docs/ validation")]
    pub skip_docs_check: bool,

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
        /// Resume a previously interrupted task by its ID
        #[arg(long, value_name = "TASK_ID")]
        resume: Option<String>,
    },

    /// Start interactive chat mode (default)
    Chat,

    /// Manage Harness Engineering docs
    Docs {
        #[command(subcommand)]
        action: DocsAction,
    },

    /// Manage saved task records
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Show version information
    Version,
}

/// Actions for the `docs` subcommand
#[derive(Subcommand, Debug, Clone)]
pub enum DocsAction {
    /// Initialise docs/ scaffolding in the current working directory
    Init,
    /// Validate docs/ against the Harness Engineering convention
    Check,
}

/// Actions for the `task` subcommand
#[derive(Subcommand, Debug, Clone)]
pub enum TaskAction {
    /// List all saved tasks
    List,
    /// Show details of a specific task
    Show {
        /// Task ID to display
        id: String,
    },
    /// Delete all completed, failed or interrupted tasks
    Clean,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Args default tests
    // ============================================================

    #[test]
    fn test_args_default_no_command() {
        let args = Args::try_parse_from(["zcode"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(args.command.is_none());
    }

    #[test]
    fn test_args_default_no_model() {
        let args = Args::try_parse_from(["zcode"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(args.model.is_none());
    }

    #[test]
    fn test_args_default_no_mcp() {
        let args = Args::try_parse_from(["zcode"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(args.mcp.is_empty());
    }

    #[test]
    fn test_args_default_not_verbose() {
        let args = Args::try_parse_from(["zcode"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(!args.verbose);
    }

    // ============================================================
    // Command parsing tests - Run
    // ============================================================

    #[test]
    fn test_run_command_basic() {
        let args = Args::try_parse_from(["zcode", "run", "test task"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        if let Some(Command::Run { task, .. }) = args.command {
            assert_eq!(task, "test task");
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_run_command_empty_task() {
        let args = Args::try_parse_from(["zcode", "run", ""]);
        assert!(args.is_ok());
        let args = args.unwrap();
        if let Some(Command::Run { task, .. }) = args.command {
            assert_eq!(task, "");
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_run_command_long_task() {
        let long_task = "x".repeat(1000);
        let args = Args::try_parse_from(["zcode", "run", &long_task]);
        assert!(args.is_ok());
        let args = args.unwrap();
        if let Some(Command::Run { task, .. }) = args.command {
            assert_eq!(task.len(), 1000);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_run_command_with_quotes() {
        let args = Args::try_parse_from(["zcode", "run", "fix the \"bug\" in code"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        if let Some(Command::Run { task, .. }) = args.command {
            assert!(task.contains("bug"));
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_run_command_unicode() {
        let args = Args::try_parse_from(["zcode", "run", "你好世界 🎉"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        if let Some(Command::Run { task, .. }) = args.command {
            assert!(task.contains("你好"));
        } else {
            panic!("Expected Run command");
        }
    }

    // ============================================================
    // Command parsing tests - Chat
    // ============================================================

    #[test]
    fn test_chat_command_basic() {
        let args = Args::try_parse_from(["zcode", "chat"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(matches!(args.command, Some(Command::Chat)));
    }

    // ============================================================
    // Command parsing tests - Version
    // ============================================================

    #[test]
    fn test_version_command_basic() {
        let args = Args::try_parse_from(["zcode", "version"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(matches!(args.command, Some(Command::Version)));
    }

    // ============================================================
    // Global flags tests - model
    // ============================================================

    #[test]
    fn test_model_flag_long() {
        let args = Args::try_parse_from(["zcode", "--model", "claude-3-opus"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert_eq!(args.model, Some("claude-3-opus".to_string()));
    }

    #[test]
    fn test_model_flag_short() {
        let args = Args::try_parse_from(["zcode", "-m", "gpt-4"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert_eq!(args.model, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_model_flag_with_command() {
        let args = Args::try_parse_from(["zcode", "-m", "claude-3", "run", "task"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert_eq!(args.model, Some("claude-3".to_string()));
        assert!(matches!(args.command, Some(Command::Run { .. })));
    }

    // ============================================================
    // Global flags tests - mcp
    // ============================================================

    #[test]
    fn test_mcp_flag_single() {
        let args = Args::try_parse_from(["zcode", "--mcp", "server1"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert_eq!(args.mcp, vec!["server1"]);
    }

    #[test]
    fn test_mcp_flag_multiple() {
        let args = Args::try_parse_from(["zcode", "--mcp", "server1", "--mcp", "server2"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert_eq!(args.mcp, vec!["server1", "server2"]);
    }

    #[test]
    fn test_mcp_flag_short() {
        let args = Args::try_parse_from(["zcode", "-M", "server1"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert_eq!(args.mcp, vec!["server1"]);
    }

    #[test]
    fn test_mcp_flag_three_servers() {
        let args = Args::try_parse_from([
            "zcode", "--mcp", "s1", "--mcp", "s2", "--mcp", "s3",
        ]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert_eq!(args.mcp, vec!["s1", "s2", "s3"]);
    }

    // ============================================================
    // Global flags tests - verbose
    // ============================================================

    #[test]
    fn test_verbose_flag_long() {
        let args = Args::try_parse_from(["zcode", "--verbose"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(args.verbose);
    }

    #[test]
    fn test_verbose_flag_short() {
        let args = Args::try_parse_from(["zcode", "-v"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(args.verbose);
    }

    #[test]
    fn test_verbose_flag_with_command() {
        let args = Args::try_parse_from(["zcode", "-v", "chat"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(args.verbose);
        assert!(matches!(args.command, Some(Command::Chat)));
    }

    // ============================================================
    // Combined flags tests
    // ============================================================

    #[test]
    fn test_multiple_flags_with_run() {
        let args = Args::try_parse_from([
            "zcode", "-v", "-m", "gpt-4", "-M", "server1", "run", "task",
        ]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(args.verbose);
        assert_eq!(args.model, Some("gpt-4".to_string()));
        assert_eq!(args.mcp, vec!["server1"]);
        if let Some(Command::Run { task, .. }) = args.command {
            assert_eq!(task, "task");
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_all_flags_with_chat() {
        let args = Args::try_parse_from([
            "zcode", "--verbose", "--model", "claude-3", "--mcp", "s1", "--mcp", "s2", "chat",
        ]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(args.verbose);
        assert_eq!(args.model, Some("claude-3".to_string()));
        assert_eq!(args.mcp, vec!["s1", "s2"]);
        assert!(matches!(args.command, Some(Command::Chat)));
    }

    // ============================================================
    // Command Debug trait test
    // ============================================================

    #[test]
    fn test_command_debug() {
        let cmd = Command::Chat;
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Chat"));
    }

    #[test]
    fn test_command_clone() {
        let cmd = Command::Chat;
        let cloned = cmd.clone();
        assert!(matches!(cloned, Command::Chat));
    }

    #[test]
    fn test_args_debug() {
        let args = Args::try_parse_from(["zcode", "chat"]).unwrap();
        let debug_str = format!("{:?}", args);
        assert!(debug_str.contains("Args"));
    }

    // ============================================================
    // Error cases
    // ============================================================

    #[test]
    fn test_run_command_missing_task() {
        let args = Args::try_parse_from(["zcode", "run"]);
        assert!(args.is_err());
    }

    #[test]
    fn test_unknown_flag() {
        let args = Args::try_parse_from(["zcode", "--unknown"]);
        assert!(args.is_err());
    }

    #[test]
    fn test_model_flag_missing_value() {
        let args = Args::try_parse_from(["zcode", "--model"]);
        assert!(args.is_err());
    }

    #[test]
    fn test_mcp_flag_missing_value() {
        let args = Args::try_parse_from(["zcode", "--mcp"]);
        assert!(args.is_err());
    }

    // ============================================================
    // Parser trait verification
    // ============================================================

    #[test]
    fn test_args_parser_trait() {
        // Verify Args implements Parser
        let _args = Args::parse_from(["zcode"]);
    }

    // ============================================================
    // Edge cases
    // ============================================================

    #[test]
    fn test_run_command_with_special_characters() {
        let args = Args::try_parse_from(["zcode", "run", "Fix bug #123: handle @mentions"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        if let Some(Command::Run { task, .. }) = args.command {
            assert!(task.contains("#123"));
            assert!(task.contains("@mentions"));
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_run_command_newlines() {
        // This tests how clap handles tasks with potential newlines
        let args = Args::try_parse_from(["zcode", "run", "Line 1"]);
        assert!(args.is_ok());
    }

    #[test]
    fn test_flags_order_independence() {
        // Flags should work in any order
        let args1 = Args::try_parse_from(["zcode", "-v", "-m", "gpt-4", "chat"]).unwrap();
        let args2 = Args::try_parse_from(["zcode", "-m", "gpt-4", "-v", "chat"]).unwrap();

        assert_eq!(args1.verbose, args2.verbose);
        assert_eq!(args1.model, args2.model);
    }
}
