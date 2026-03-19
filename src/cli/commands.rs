//! Command handlers for zcode CLI
//!
//! This module implements the handlers for each CLI command.

use crate::cli::args::Command;
use crate::error::Result;
use crate::tui::{init_terminal, restore_terminal, TuiApp};
use crate::Settings;
use tracing::info;

/// Execute a CLI command
pub async fn execute_command(command: &Command, args: &crate::cli::args::Args) -> Result<()> {
    match command {
        Command::Run { task } => execute_run(task, args).await,
        Command::Chat => execute_chat(args).await,
        Command::Version => execute_version(),
    }
}

/// Execute the default interactive chat mode
pub async fn execute_default(args: &crate::cli::args::Args) -> Result<()> {
    execute_chat(args).await
}

/// Run a single task in non-interactive mode
async fn execute_run(task: &str, args: &crate::cli::args::Args) -> Result<()> {
    info!("Running task: {}", task);

    // Load settings
    let mut settings = Settings::load().unwrap_or_default();

    // Override model if specified
    if let Some(model) = &args.model {
        info!("Using model: {}", model);
        settings.llm.model = model.clone();
    }

    // TODO: Integrate with agent module for actual task execution
    // For now, we just print the task
    println!("Task: {}", task);
    println!("Model: {}", settings.llm.model);
    println!("(Full agent integration coming soon)");

    Ok(())
}

/// Start interactive chat mode
async fn execute_chat(args: &crate::cli::args::Args) -> Result<()> {
    info!("Starting interactive chat mode");

    // Load settings
    let mut settings = Settings::load().unwrap_or_default();

    // Override model if specified
    if let Some(model) = &args.model {
        info!("Using model: {}", model);
        settings.llm.model = model.clone();
    }

    // Log MCP servers if specified
    if !args.mcp.is_empty() {
        info!("MCP servers: {:?}", args.mcp);
    }

    // Initialize terminal
    let mut terminal = init_terminal()?;

    // Create and run TUI application
    let mut app = TuiApp::new();

    // Add welcome message
    app.chat.add_message(crate::tui::chat::ChatMessage::system(
        format!(
            "Welcome to zcode! Using model: {}. Type a message and press Enter to send.",
            settings.llm.model
        )
    ));

    // Run the event loop
    let result = app.run(&mut terminal);

    // Restore terminal
    restore_terminal(&mut terminal)?;

    result
}

/// Show version information
fn execute_version() -> Result<()> {
    println!("zcode {}", env!("CARGO_PKG_VERSION"));
    println!("A programming agent CLI tool");
    println!();
    println!("Authors: {}", env!("CARGO_PKG_AUTHORS"));
    println!("License: {}", env!("CARGO_PKG_LICENSE"));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::Args;

    #[test]
    fn test_version_output() {
        // This test verifies that execute_version doesn't error
        let result = execute_version();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_command_basic() {
        let args = Args {
            command: Some(Command::Run {
                task: "test task".to_string(),
            }),
            model: None,
            mcp: vec![],
            verbose: false,
        };

        if let Some(Command::Run { task }) = &args.command {
            let result = execute_run(task, &args).await;
            assert!(result.is_ok());
        } else {
            panic!("Expected Run command");
        }
    }

    #[tokio::test]
    async fn test_run_command_with_model() {
        let args = Args {
            command: Some(Command::Run {
                task: "test task".to_string(),
            }),
            model: Some("claude-3-opus".to_string()),
            mcp: vec![],
            verbose: false,
        };

        if let Some(Command::Run { task }) = &args.command {
            let result = execute_run(task, &args).await;
            assert!(result.is_ok());
        } else {
            panic!("Expected Run command");
        }
    }
}
