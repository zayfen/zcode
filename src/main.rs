//! Zcode - A programming agent CLI tool
//!
//! This is the main entry point for the zcode CLI.

use clap::Parser;
use tracing_subscriber::EnvFilter;

/// Zcode programming agent
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the project directory
    #[arg(short, long, default_value = ".")]
    path: String,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

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

    tracing::info!("Starting zcode in directory: {}", args.path);

    // Load settings
    let settings = zcode::Settings::load().unwrap_or_default();
    tracing::debug!("Loaded settings: {:?}", settings);

    // Initialize tool registry
    let registry = zcode::ToolRegistry::new();
    tracing::debug!("Initialized tool registry with {} tools", registry.list().len());

    tracing::info!("Zcode initialized successfully");

    Ok(())
}
