//! CLI integration tests for zcode
//!
//! Tests for argument parsing and command dispatch.

use clap::Parser;
use zcode::cli::args::{Args, Command};

/// Test that default mode (no subcommand) starts interactive chat
#[test]
fn test_default_mode_is_chat() {
    let args = Args::try_parse_from(["zcode"]);
    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(args.command.is_none());
}

/// Test that `zcode chat` command is parsed correctly
#[test]
fn test_chat_command() {
    let args = Args::try_parse_from(["zcode", "chat"]);
    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(matches!(args.command, Some(Command::Chat)));
}

/// Test that `zcode run <task>` command is parsed correctly
#[test]
fn test_run_command() {
    let args = Args::try_parse_from(["zcode", "run", "fix the bug"]);
    assert!(args.is_ok());
    let args = args.unwrap();
    if let Some(Command::Run { task, .. }) = args.command {
        assert_eq!(task, "fix the bug");
    } else {
        panic!("Expected Run command");
    }
}

/// Test that `zcode version` command is parsed correctly
#[test]
fn test_version_command() {
    let args = Args::try_parse_from(["zcode", "version"]);
    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(matches!(args.command, Some(Command::Version)));
}

/// Test that --model flag is parsed correctly
#[test]
fn test_model_flag() {
    let args = Args::try_parse_from(["zcode", "--model", "claude-3-opus"]);
    assert!(args.is_ok());
    let args = args.unwrap();
    assert_eq!(args.model, Some("claude-3-opus".to_string()));
}

/// Test that --mcp flag can be specified multiple times
#[test]
fn test_mcp_flag_multiple() {
    let args = Args::try_parse_from(["zcode", "--mcp", "server1", "--mcp", "server2"]);
    assert!(args.is_ok());
    let args = args.unwrap();
    assert_eq!(args.mcp, vec!["server1", "server2"]);
}

/// Test that -M short flag works for MCP
#[test]
fn test_mcp_short_flag() {
    let args = Args::try_parse_from(["zcode", "-M", "server1"]);
    assert!(args.is_ok());
    let args = args.unwrap();
    assert_eq!(args.mcp, vec!["server1"]);
}

/// Test that verbose flag is parsed
#[test]
fn test_verbose_flag() {
    let args = Args::try_parse_from(["zcode", "-v"]);
    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(args.verbose);
}

/// Test that long verbose flag works
#[test]
fn test_verbose_long_flag() {
    let args = Args::try_parse_from(["zcode", "--verbose"]);
    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(args.verbose);
}

/// Test combined flags and commands
#[test]
fn test_combined_flags_and_command() {
    let args = Args::try_parse_from(["zcode", "-v", "--model", "gpt-4", "run", "test task"]);
    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(args.verbose);
    assert_eq!(args.model, Some("gpt-4".to_string()));
    if let Some(Command::Run { task, .. }) = args.command {
        assert_eq!(task, "test task");
    } else {
        panic!("Expected Run command");
    }
}

/// Test that help flag works
#[test]
fn test_help_flag() {
    let result = Args::try_parse_from(["zcode", "--help"]);
    // --help causes clap to exit early with an error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.kind() == clap::error::ErrorKind::DisplayHelp);
}

/// Test version flag with command
#[test]
fn test_version_with_command() {
    let args = Args::try_parse_from(["zcode", "version"]);
    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(matches!(args.command, Some(Command::Version)));
}
