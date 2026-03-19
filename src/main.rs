//! Zcode - A programming agent CLI tool
//!
//! This is the main entry point for the zcode CLI.

use std::sync::Arc;

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

    // Initialize tool registry with built-in tools
    let mut registry = zcode::ToolRegistry::new();
    registry.register_built_in_tools();
    tracing::debug!("Initialized tool registry with {} tools", registry.list().len());

    // Initialize LLM provider based on config
    let llm_config = zcode::LlmConfig {
        provider: settings.llm.provider.clone(),
        model: settings.llm.model.clone(),
        api_key: settings.llm.api_key.clone(),
        temperature: settings.llm.temperature,
        max_tokens: settings.llm.max_tokens,
    };

    let llm: Arc<dyn zcode::llm::LlmProvider> =
        if std::env::var("ANTHROPIC_API_KEY").is_ok() || llm_config.api_key.is_some() {
            tracing::info!(
                "Using {} provider with model {}",
                llm_config.provider,
                llm_config.model
            );
            Arc::new(zcode::llm::RigProvider::new(llm_config))
        } else {
            tracing::warn!("No API key found, using mock LLM provider");
            Arc::new(zcode::llm::provider::MockLlmProvider::new(
                "I'm a mock response. Set ANTHROPIC_API_KEY for real responses.",
            ))
        };

    let _registry = Arc::new(registry);
    let _llm = llm;

    // Initialize and run TUI
    let mut terminal = zcode::tui::init_terminal()?;
    let mut app = zcode::TuiApp::new();

    // Add welcome message
    app.chat.add_message(zcode::tui::chat::ChatMessage::system(
        "Welcome to zcode! Type a message and press Enter to chat. Esc to quit.",
    ));

    let result = app.run(&mut terminal);

    // Always restore terminal
    zcode::tui::restore_terminal(&mut terminal)?;

    result?;
    Ok(())
}
