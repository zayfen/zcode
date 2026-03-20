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

    // Build system prompt with available tools and their schemas
    let tools_desc = registry
        .tool_definitions()
        .into_iter()
        .map(|def| {
            let name = def["name"].as_str().unwrap_or("unknown");
            let desc = def["description"].as_str().unwrap_or("");
            let schema = serde_json::to_string_pretty(&def["parameters"]).unwrap_or_default();
            format!("- {}\n  Description: {}\n  Input Schema:\n{}", name, desc, schema)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let system_prompt = format!(
        r#"You are zcode, a programming assistant. You can help with:
- Reading and writing files
- Executing shell commands
- Answering programming questions

Available tools:
{}

When you need to use a tool, respond with a JSON block like:
```json
{{"tool": "tool_name", "input": {{"arg": "value"}}}}
```

Otherwise, respond with helpful text."#,
        tools_desc
    );

    let registry = Arc::new(registry);

    // Create the agent
    let agent = zcode::agent::Agent::new("zcode", llm, registry, system_prompt);

    // Initialize and run TUI
    let mut terminal = zcode::tui::init_terminal()?;
    let mut app = zcode::TuiApp::new();
    app.set_agent(agent);

    // Add welcome message
    app.chat.add_message(zcode::tui::chat::ChatMessage::system(
        "Welcome to zcode! Type a message and press Enter to chat. Esc to quit.",
    ));

    let result = app.run_async(&mut terminal).await;

    // Always restore terminal
    zcode::tui::restore_terminal(&mut terminal)?;

    result?;
    Ok(())
}
