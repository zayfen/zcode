//! Zcode - A programming agent CLI tool
//!
//! This is the main entry point for the zcode CLI.

use clap::Parser;
use tracing_subscriber::EnvFilter;
use zcode::cli::args::Args;
use zcode::cli::commands::{execute_command, execute_default};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let filter = if args.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    tracing::info!("Starting zcode");

    // Execute command or default to interactive chat
    if let Some(command) = &args.command {
        execute_command(command, &args).await?;
    } else {
        execute_default(&args).await?;
    }

    tracing::info!("Zcode finished");
    Ok(())
}
