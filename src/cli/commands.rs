//! Command handlers for zcode CLI
//!
//! This module implements the handlers for each CLI command.

use crate::agent::loop_exec::{AgentLoop, ConversationMessage, LoopConfig, LlmResponse};
use crate::cli::args::{Command, DocsAction, TaskAction};
use crate::docs::{generate_docs_scaffold, DocsValidator};
use crate::error::Result;
use crate::llm::provider::{LlmProvider, RigProvider};
use crate::llm::{LlmConfig, Message, MessageRole};
use crate::skills::SkillsLoader;
use crate::task_store::{TaskRecord, TaskStatus, TaskStore};
use crate::tools::{register_default_tools, ToolRegistry};
use crate::tui::{init_terminal, restore_terminal, TuiApp};
use crate::Settings;
use std::path::Path;
use std::sync::Arc;
use tracing::info;

/// Execute a CLI command
pub async fn execute_command(command: &Command, args: &crate::cli::args::Args) -> Result<()> {
    // ── Harness Engineering docs validation ──────────────────────────
    // All non-docs commands require a valid docs/ directory, unless
    // --skip-docs-check is passed or the command is `docs`.
    if !args.skip_docs_check {
        if let Command::Run { .. } | Command::Chat = command {
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            run_docs_validation(&cwd)?;
        }
    }

    match command {
        Command::Run { task, resume } => execute_run(task, resume.as_deref(), args).await,
        Command::Chat => execute_chat(args).await,
        Command::Docs { action } => execute_docs(action),
        Command::Task { action } => execute_task(action),
        Command::Version => execute_version(),
    }
}

/// Validate docs/ in `project_dir` and print results; returns `Err` if invalid.
fn run_docs_validation(project_dir: &Path) -> Result<()> {
    let validator = DocsValidator::new(project_dir);
    let result = validator.validate();
    if result.is_valid() {
        info!("docs/ validation passed");
        return Ok(());
    }
    // Print all errors to stderr
    eprintln!("\n╔══════════════════════════════════════════════════════════╗");
    eprintln!("║  Harness Engineering: docs/ validation FAILED            ║");
    eprintln!("╚══════════════════════════════════════════════════════════╝");
    for (i, err) in result.errors.iter().enumerate() {
        eprintln!("  {}. {}", i + 1, err.message);
        eprintln!("     → {}", err.hint);
    }
    eprintln!();
    eprintln!("  Run `zcode docs init` to generate the required scaffolding.");
    eprintln!("  Run `zcode docs check` to see this report again.");
    eprintln!("  Use `--skip-docs-check` to bypass this validation.");
    eprintln!();
    Err(crate::error::ZcodeError::ConfigError(
        "docs/ validation failed. Fix the issues above before running zcode.".to_string(),
    ))
}

/// Handle `zcode docs {init|check}` commands.
fn execute_docs(action: &DocsAction) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    match action {
        DocsAction::Init => {
            let created = generate_docs_scaffold(&cwd).map_err(|e| {
                crate::error::ZcodeError::ConfigError(format!("Failed to create docs scaffold: {}", e))
            })?;
            if created.is_empty() {
                println!("docs/ scaffolding already exists — nothing to create.");
            } else {
                println!("Created {} file(s):", created.len());
                for path in &created {
                    println!("  {}", path.display());
                }
                println!("\nNext steps:");
                println!("  1. Fill in docs/prd/001-feature.md with your requirements.");
                println!("  2. Update docs/specs/coding.spec.md with your tech stack.");
                println!("  3. Add tasks to docs/tasks/001-feature.tasks.md.");
                println!("  4. Then run: zcode run \"<your task>\"");
            }
            Ok(())
        }
        DocsAction::Check => {
            let validator = DocsValidator::new(&cwd);
            let result = validator.validate();
            if result.is_valid() {
                println!("docs/ validation passed ✓");
            } else {
                run_docs_validation(&cwd)?;
            }
            Ok(())
        }
    }
}

/// Execute the default interactive chat mode
pub async fn execute_default(args: &crate::cli::args::Args) -> Result<()> {
    execute_chat(args).await
}

/// Handle `zcode task {list|show|clean}` commands.
fn execute_task(action: &TaskAction) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let store = TaskStore::new(&cwd)?;
    match action {
        TaskAction::List => {
            let tasks = store.list()?;
            if tasks.is_empty() {
                println!("No saved tasks. Run `zcode run \"<task>\"` to create one.");
            } else {
                println!("{:<10} {:<12} {:<5} {}",
                    "ID", "STATUS", "ITER", "TASK");
                println!("{}", "-".repeat(70));
                for t in &tasks {
                    let snippet = if t.task.len() > 45 {
                        format!("{}…", &t.task[..45])
                    } else {
                        t.task.clone()
                    };
                    println!("{:<10} {:<12} {:<5} {}",
                        t.id, t.status.to_string(), t.iteration, snippet);
                }
            }
            Ok(())
        }
        TaskAction::Show { id } => {
            let record = store.load(id)?;
            println!("ID:        {}", record.id);
            println!("Status:    {}", record.status);
            println!("Iteration: {}", record.iteration);
            println!("Task:      {}", record.task);
            println!("Created:   {}", record.created_at);
            println!("Updated:   {}", record.updated_at);
            if let Some(result) = &record.result {
                println!("\nResult:\n{}", result);
            }
            if let Some(error) = &record.error {
                println!("\nError: {}", error);
            }
            println!("\nHistory: {} messages", record.history.len());
            Ok(())
        }
        TaskAction::Clean => {
            let deleted = store.clean()?;
            if deleted == 0 {
                println!("Nothing to clean — no completed/failed tasks.");
            } else {
                println!("Cleaned {} task record(s).", deleted);
            }
            Ok(())
        }
    }
}

/// Run a single task in non-interactive mode using AgentLoop + LLM
async fn execute_run(task: &str, resume_id: Option<&str>, args: &crate::cli::args::Args) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // ── Task Store ────────────────────────────────────────────────────
    let store = TaskStore::new(&cwd)?;
    let mut task_record: TaskRecord = if let Some(id) = resume_id {
        let mut record = store.load(id)?;
        if record.status == TaskStatus::Completed {
            println!("⚠  Task '{}' already completed. Starting fresh.", id);
            record = store.create(task);
        } else {
            println!("▶  Resuming task '{}' from iteration {}.", record.id, record.iteration);
        }
        record
    } else {
        store.create(task)
    };
    info!("Task record id={}", task_record.id);

    // ── Skills ────────────────────────────────────────────────────────
    let skills = SkillsLoader::load(&cwd);
    if !skills.is_empty() {
        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        println!("📚 Loaded {} skill(s): {}", skills.len(), names.join(", "));
    }

    // ── Model / LLM config ────────────────────────────────────────────
    let mut settings = Settings::load().unwrap_or_default();
    if let Some(model) = &args.model {
        settings.llm.model = model.clone();
    }
    let llm_config = LlmConfig {
        provider: settings.llm.provider.clone(),
        model: settings.llm.model.clone(),
        api_key: settings.llm.api_key.clone(),
        temperature: settings.llm.temperature,
        max_tokens: settings.llm.max_tokens,
    };
    let provider: Arc<dyn LlmProvider> = Arc::new(RigProvider::new(llm_config.clone()));

    // ── Tool registry ─────────────────────────────────────────────────
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let registry = Arc::new(registry);

    // ── System prompt (base + skills) ─────────────────────────────────
    let base_prompt = format!(
        "You are zcode, a senior AI coding agent using model {}. \
         You have access to tools for reading/writing files, running shell commands, \
         and searching code. Execute the user's task completely and report results.",
        llm_config.model
    );
    let system_prompt = SkillsLoader::build_system_prompt(&base_prompt, &skills);
    let loop_config = LoopConfig { max_iterations: 20, system_prompt };
    let agent_loop = AgentLoop::new(loop_config, Arc::clone(&registry));

    println!("🤖 zcode agent starting...");
    println!("📋 Task: {} [id={}]", task, task_record.id);
    println!("🧠 Model: {}", llm_config.model);
    println!("💾 Progress saved to .zcode/tasks/{}.json", task_record.id);
    println!();

    // Save initial state
    let _ = store.save(&mut task_record);

    let resume_history = task_record.history.clone();
    let provider_clone = Arc::clone(&provider);
    let result = agent_loop.run(
        task,
        &[],
        move |messages, tools| {
            let p = Arc::clone(&provider_clone);
            async move {
                let llm_messages: Vec<Message> = messages.iter()
                    .filter_map(|v| {
                        let role = v.get("role")?.as_str()?;
                        let content = v.get("content")?.as_str().unwrap_or("").to_string();
                        let role = match role {
                            "system" => MessageRole::System,
                            "assistant" => MessageRole::Assistant,
                            _ => MessageRole::User,
                        };
                        Some(Message { role, content })
                    })
                    .collect();

                match p.chat(&llm_messages, &tools) {
                    Ok(resp) => {
                        // Directly use AgentLoop's format parser
                        if let Ok(agent_resp) = LlmResponse::from_anthropic_response(&resp.raw_response) {
                            Ok(agent_resp)
                        } else {
                            Ok(LlmResponse::Text(resp.content))
                        }
                    }
                    Err(crate::error::ZcodeError::MissingApiKey(provider)) => {
                        Ok(LlmResponse::Text(format!(
                            "Task acknowledged. No API key found for provider '{}'. \
                             Set the corresponding env variable to enable LLM responses.",
                            provider
                        )))
                    }
                    Err(e) => Err(e),
                }
            }
        },
    ).await;

    // Persist resume history (best-effort; we don't have per-iteration hooks yet)
    task_record.history = resume_history;

    match result {
        Ok(loop_result) => {
            task_record.status = TaskStatus::Completed;
            task_record.result = Some(loop_result.answer.clone());
            task_record.history = loop_result.history.clone();
            task_record.iteration = loop_result.llm_calls;
            let _ = store.save(&mut task_record);

            println!("\n✅ Task complete ({} LLM calls, {} tool calls)",
                loop_result.llm_calls, loop_result.tool_calls_executed);
            if loop_result.hit_max_iterations {
                println!("⚠  Max iterations reached.");
            }
            println!("\n📤 Result:\n{}", loop_result.answer);
            Ok(())
        }
        Err(e) => {
            task_record.status = TaskStatus::Failed;
            task_record.error = Some(e.to_string());
            let _ = store.save(&mut task_record);
            Err(e)
        }
    }
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

    // Build LLM provider from settings
    let llm_config = LlmConfig {
        provider: settings.llm.provider.clone(),
        model: settings.llm.model.clone(),
        api_key: settings.llm.api_key.clone(),
        temperature: settings.llm.temperature,
        max_tokens: settings.llm.max_tokens,
    };

    let provider: Arc<dyn LlmProvider> = Arc::new(RigProvider::new(llm_config.clone()));

    // Initialize terminal
    let mut terminal = init_terminal()?;

    // Create TUI application with real LLM provider
    let mut app = TuiApp::with_provider(provider);
    app.chat.add_message(crate::tui::chat::ChatMessage::system(
        format!(
            "Model: {} | Press Esc or Ctrl+C to quit",
            llm_config.model
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

    // ============================================================
    // execute_version tests
    // ============================================================

    #[test]
    fn test_execute_version_success() {
        let result = execute_version();
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_version_returns_unit() {
        let result: Result<()> = execute_version();
        assert!(result.is_ok());
        assert!(matches!(result, Ok(())));
    }

    // ============================================================
    // execute_run tests
    // ============================================================

    #[tokio::test]
    async fn test_execute_run_basic() {
        let args = Args {
            command: Some(Command::Run {
                task: "test task".to_string(),
                resume: None,
            }),
            model: None,
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None }) = &args.command {
            let result = execute_run(task, None, &args).await;
            assert!(result.is_ok());
        } else {
            panic!("Expected Run command");
        }
    }

    #[tokio::test]
    async fn test_execute_run_with_model() {
        let args = Args {
            command: Some(Command::Run {
                task: "test task".to_string(),
                resume: None,
            }),
            model: Some("claude-3-opus".to_string()),
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None }) = &args.command {
            let result = execute_run(task, None, &args).await;
            assert!(result.is_ok());
        } else {
            panic!("Expected Run command");
        }
    }

    #[tokio::test]
    async fn test_execute_run_empty_task() {
        let args = Args {
            command: Some(Command::Run {
                task: "".to_string(),
                resume: None,
            }),
            model: None,
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None }) = &args.command {
            let result = execute_run(task, None, &args).await;
            assert!(result.is_ok());
        } else {
            panic!("Expected Run command");
        }
    }

    #[tokio::test]
    async fn test_execute_run_long_task() {
        let long_task = "x".repeat(1000);
        let args = Args {
            command: Some(Command::Run {
                task: long_task.clone(),
                resume: None,
            }),
            model: None,
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None }) = &args.command {
            let result = execute_run(task, None, &args).await;
            assert!(result.is_ok());
        } else {
            panic!("Expected Run command");
        }
    }

    #[tokio::test]
    async fn test_execute_run_with_mcp_servers() {
        let args = Args {
            command: Some(Command::Run {
                task: "test".to_string(),
                resume: None,
            }),
            model: None,
            mcp: vec!["server1".to_string(), "server2".to_string()],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None }) = &args.command {
            let result = execute_run(task, None, &args).await;
            assert!(result.is_ok());
        } else {
            panic!("Expected Run command");
        }
    }

    #[tokio::test]
    async fn test_execute_run_verbose() {
        let args = Args {
            command: Some(Command::Run {
                task: "test".to_string(),
                resume: None,
            }),
            model: None,
            mcp: vec![],
            verbose: true,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None }) = &args.command {
            let result = execute_run(task, None, &args).await;
            assert!(result.is_ok());
        } else {
            panic!("Expected Run command");
        }
    }

    #[tokio::test]
    async fn test_execute_run_special_characters() {
        let args = Args {
            command: Some(Command::Run {
                task: "Fix \"bug\" #123 @user".to_string(),
                resume: None,
            }),
            model: None,
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None }) = &args.command {
            let result = execute_run(task, None, &args).await;
            assert!(result.is_ok());
        } else {
            panic!("Expected Run command");
        }
    }

    #[tokio::test]
    async fn test_execute_run_unicode() {
        let args = Args {
            command: Some(Command::Run {
                task: "你好世界 🎉".to_string(),
                resume: None,
            }),
            model: None,
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None }) = &args.command {
            let result = execute_run(task, None, &args).await;
            assert!(result.is_ok());
        } else {
            panic!("Expected Run command");
        }
    }

    // ============================================================
    // execute_command tests
    // ============================================================

    #[tokio::test]
    async fn test_execute_command_run() {
        let args = Args {
            command: Some(Command::Run {
                task: "test".to_string(),
                resume: None,
            }),
            model: None,
            mcp: vec![],
            verbose: false,
            skip_docs_check: true, // no docs/ in test env
        };

        if let Some(ref cmd) = args.command {
            let result = execute_command(cmd, &args).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_execute_command_version() {
        let args = Args {
            command: Some(Command::Version),
            model: None,
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(ref cmd) = args.command {
            let result = execute_command(cmd, &args).await;
            assert!(result.is_ok());
        }
    }

    // ============================================================
    // execute_default tests
    // ============================================================

    #[test]
    fn test_execute_default_exists() {
        // Verify the function exists - we can't test execution without TUI
        // Just check that the function is accessible by referencing it
        let _ = || execute_default;
    }

    // ============================================================
    // Command enum tests
    // ============================================================

    #[test]
    fn test_command_run_clone() {
        let cmd = Command::Run {
            task: "test".to_string(),
            resume: None,
        };
        let cloned = cmd.clone();
        if let Command::Run { task, resume: None } = cloned {
            assert_eq!(task, "test");
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_command_chat_clone() {
        let cmd = Command::Chat;
        let cloned = cmd.clone();
        assert!(matches!(cloned, Command::Chat));
    }

    #[test]
    fn test_command_version_clone() {
        let cmd = Command::Version;
        let cloned = cmd.clone();
        assert!(matches!(cloned, Command::Version));
    }

    #[test]
    fn test_command_debug() {
        let cmd = Command::Chat;
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Chat"));
    }

    // ============================================================
    // Args struct tests
    // ============================================================

    #[test]
    fn test_args_construction() {
        let args = Args {
            command: Some(Command::Version),
            model: Some("gpt-4".to_string()),
            mcp: vec!["server1".to_string()],
            verbose: true,
            skip_docs_check: false,
        };

        assert!(matches!(args.command, Some(Command::Version)));
        assert_eq!(args.model, Some("gpt-4".to_string()));
        assert_eq!(args.mcp, vec!["server1"]);
        assert!(args.verbose);
    }

    #[test]
    fn test_args_clone() {
        let args = Args {
            command: Some(Command::Chat),
            model: Some("claude".to_string()),
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };
        let cloned = args.clone();
        assert!(matches!(cloned.command, Some(Command::Chat)));
        assert_eq!(cloned.model, Some("claude".to_string()));
    }

    #[test]
    fn test_args_debug() {
        let args = Args {
            command: Some(Command::Version),
            model: None,
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };
        let debug_str = format!("{:?}", args);
        assert!(debug_str.contains("Args"));
        assert!(debug_str.contains("Version"));
    }

    // ============================================================
    // Edge cases
    // ============================================================

    #[tokio::test]
    async fn test_execute_run_multiple_mcp_servers() {
        let args = Args {
            command: Some(Command::Run {
                task: "test".to_string(),
                resume: None,
            }),
            model: None,
            mcp: vec![
                "server1".to_string(),
                "server2".to_string(),
                "server3".to_string(),
            ],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None }) = &args.command {
            let result = execute_run(task, None, &args).await;
            assert!(result.is_ok());
        } else {
            panic!("Expected Run command");
        }
    }

    #[tokio::test]
    async fn test_execute_run_all_options() {
        let args = Args {
            command: Some(Command::Run {
                task: "complex task".to_string(),
                resume: None,
            }),
            model: Some("claude-3-opus".to_string()),
            mcp: vec!["mcp-server".to_string()],
            verbose: true,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None }) = &args.command {
            let result = execute_run(task, None, &args).await;
            assert!(result.is_ok());
        } else {
            panic!("Expected Run command");
        }
    }

    // ============================================================
    // Result type tests
    // ============================================================

    #[test]
    fn test_result_ok() {
        let result: Result<()> = Ok(());
        assert!(result.is_ok());
    }

    #[test]
    fn test_result_err() {
        let result: Result<()> = Err(crate::error::ZcodeError::Cancelled);
        assert!(result.is_err());
    }
}
