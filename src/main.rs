//! Zcode - A programming agent CLI tool
//!
//! This is the main entry point for the zcode CLI.

use clap::Parser;
use tracing_subscriber::EnvFilter;
use zcode_cli::args::{Args, Command};
use zcode_cli::commands::{execute_command, execute_default};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    init_tracing(&args);

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

fn init_tracing(args: &Args) {
    let filter = if args.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    if is_tui_command(args) {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::sink)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
}

fn is_tui_command(args: &Args) -> bool {
    matches!(args.command, None | Some(Command::Chat))
}
