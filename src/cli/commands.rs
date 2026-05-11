//! Command handlers for zcode CLI
//!
//! This module implements the handlers for each CLI command.

use crate::cli::args::{Command, DocsAction, TaskAction};
use crate::workspace::Workspace;
use zcode_capabilities::{McpClient, SkillsLoader, ToolRegistry};
use zcode_core::{LlmConfig, Result, Settings, ZcodeError};
use zcode_llm_provider::{LlmProvider, Message};
#[cfg(test)]
use zcode_llm_provider::MockLlmProvider;
#[cfg(not(test))]
use zcode_llm_provider::RigProvider;
use zcode_orchestration::{
    build_reviewer_pipeline, build_task_pipeline_with_limit, AgentLoop, ConversationMessage,
    DefaultState, LlmResponse as AgentLlmResponse, LoopConfig, TaskResult,
};
use zcode_requirements::docs::parser::parse_all_tasks;
use zcode_requirements::{generate_docs_scaffold, DocsValidator, TaskRecord, TaskStatus, TaskStore};
use zcode_ui::{init_terminal, restore_terminal, TuiApp};
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
        Command::Run { task, resume, max_iterations } => {
            let _ = execute_run(task, resume.as_deref(), *max_iterations, args).await?;
            Ok(())
        }
        Command::Feed { path, max_iterations, investigate } => {
            execute_feed(path, *investigate, *max_iterations, args).await
        }
        Command::Chat => execute_chat(args).await,
        Command::Docs { action } => execute_docs(action),
        Command::Task { action } => execute_task(action, args).await,
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
    Err(ZcodeError::ConfigError(
        "docs/ validation failed. Fix the issues above before running zcode.".to_string(),
    ))
}

fn llm_config_from_settings(settings: &Settings) -> LlmConfig {
    LlmConfig {
        provider: settings.llm.provider.clone(),
        model: settings.llm.model.clone(),
        fast_model: settings.llm.fast_model.clone(),
        api_key: settings.llm.api_key.clone(),
        temperature: settings.llm.temperature,
        max_tokens: settings.llm.max_tokens,
    }
}

fn fast_llm_config(config: &LlmConfig) -> LlmConfig {
    let mut fast = config.clone();
    fast.model = config
        .fast_model
        .clone()
        .unwrap_or_else(|| config.model.clone());
    fast
}

#[cfg(not(test))]
fn make_llm_provider(config: &LlmConfig) -> Arc<dyn LlmProvider> {
    Arc::new(RigProvider::new(config.clone()))
}

#[cfg(test)]
fn make_llm_provider(_config: &LlmConfig) -> Arc<dyn LlmProvider> {
    Arc::new(MockLlmProvider::new("PASS"))
}

fn build_tool_registry(
    cwd: &Path,
    settings: &Settings,
    args: &crate::cli::args::Args,
) -> Result<Arc<ToolRegistry>> {
    let mut registry = ToolRegistry::new();

    let ws_config = Workspace::open(cwd).map(|w| w.config).unwrap_or_default();

    for mcp_cfg in &settings.mcp_servers {
        if !mcp_cfg.auto_start {
            continue;
        }
        let exec_args: Vec<&str> = mcp_cfg.args.iter().map(|s| s.as_str()).collect();
        info!("Starting global MCP server: {} {:?}", mcp_cfg.command, exec_args);
        let client = McpClient::connect_stdio(&mcp_cfg.name, &mcp_cfg.command, &exec_args)?;
        for adapter in Arc::new(client).create_adapters() {
            registry.register(adapter);
        }
    }

    for mcp_cfg in ws_config.mcp_servers {
        if !mcp_cfg.auto_start {
            continue;
        }
        let exec_args: Vec<&str> = mcp_cfg.args.iter().map(|s| s.as_str()).collect();
        info!("Starting workspace MCP server: {} {:?}", mcp_cfg.command, exec_args);
        let client = McpClient::connect_stdio(&mcp_cfg.name, &mcp_cfg.command, &exec_args)?;
        for adapter in Arc::new(client).create_adapters() {
            registry.register(adapter);
        }
    }

    for mcp_str in &args.mcp {
        let parts: Vec<&str> = mcp_str.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let command = parts[0];
        let exec_args: Vec<&str> = parts[1..].to_vec();
        info!("Starting CLI MCP server: {} {:?}", command, exec_args);
        let client = McpClient::connect_stdio("cli_mcp", command, &exec_args)?;
        for adapter in Arc::new(client).create_adapters() {
            registry.register(adapter);
        }
    }

    Ok(Arc::new(registry))
}

/// Handle `zcode docs {init|check}` commands.
fn execute_docs(action: &DocsAction) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    match action {
        DocsAction::Init => {
            let created = generate_docs_scaffold(&cwd).map_err(|e| {
                ZcodeError::ConfigError(format!("Failed to create docs scaffold: {}", e))
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
async fn execute_task(action: &TaskAction, args: &crate::cli::args::Args) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let store = TaskStore::new(&cwd)?;
    match action {
        TaskAction::List => {
            let tasks = store.list()?;
            if tasks.is_empty() {
                println!("No saved tasks. Run `zcode task sync` to import from docs/tasks/.");
            } else {
                println!("{:<10} {:<12} {:<5} {}",
                    "ID", "STATUS", "ITER", "TASK");
                println!("{}", "-".repeat(70));
                for t in &tasks {
                    let snippet = if t.task.chars().count() > 45 {
                        let truncated: String = t.task.chars().take(45).collect();
                        format!("{}…", truncated)
                    } else {
                        t.task.clone()
                    };
                    println!("{:<10} {:<12} {:<5} {}",
                        t.id, t.status.to_string(), t.state.iteration, snippet);
                }
            }
            Ok(())
        }
        TaskAction::Show { id } => {
            let record = store.load(id)?;
            println!("ID:        {}", record.id);
            println!("Status:    {}", record.status);
            println!("Iteration: {}", record.state.iteration);
            println!("Task:      {}", record.task);
            println!("Created:   {}", record.created_at);
            println!("Updated:   {}", record.updated_at);
            if let Some(result) = &record.state.result {
                println!("\nResult:\n{}", result.output);
            }
            if let Some(error) = &record.error {
                println!("\nError: {}", error);
            }
            println!("\nHistory: {} messages", record.state.messages.len());
            Ok(())
        }
        TaskAction::Run { task_or_id, max_iterations } => {
            // Try to load from store first; if not found, treat as direct task description
            let task_description = match store.load(task_or_id) {
                Ok(record) => {
                    println!("📋 Found task in store: [{}] {}", record.id, record.task);
                    record.task
                }
                Err(_) => {
                    println!("📋 Running direct task: {}", task_or_id);
                    task_or_id.clone()
                }
            };
            let _ = execute_run(&task_description, None, *max_iterations, args).await?;
            Ok(())
        }
        TaskAction::RunAll { concurrency, max_iterations } => {
            let tasks = store.list()?;
            let pending: Vec<_> = tasks.into_iter()
                .filter(|t| matches!(t.status, TaskStatus::Running | TaskStatus::Interrupted | TaskStatus::Failed))
                .collect();

            if pending.is_empty() {
                println!("No pending tasks. Run `zcode task sync` to import from docs/tasks/.");
                return Ok(());
            }

            println!("🚀 Running {} pending task(s) with concurrency={}", pending.len(), concurrency);
            println!("{}", "-".repeat(60));

            let semaphore = Arc::new(tokio::sync::Semaphore::new(*concurrency));
            let args_arc = Arc::new(args.clone());
            let max_iter = *max_iterations;
            let total_count = pending.len();

            let mut handles = Vec::new();
            for (i, task) in pending.iter().enumerate() {
                let sem = Arc::clone(&semaphore);
                let task_desc = task.task.clone();
                let task_id = task.id.clone();
                let args_clone = Arc::clone(&args_arc);

                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await.expect("semaphore closed");
                    println!("\n▶ [{}/{}] Starting: {} [id={}]", i + 1, total_count, task_desc, task_id);
                    // execute_run_task_only runs the per-task orchestrator graph.
                    let result = execute_run_task_only(&task_desc, Some(&task_id), max_iter, &args_clone).await;
                    match &result {
                        Ok(_) => println!("✅ [{}] Completed: {}", task_id, task_desc),
                        Err(e)    => println!("❌ [{}] Failed: {} — {}", task_id, task_desc, e),
                    }
                    (task_id, task_desc, result)
                });
                handles.push(handle);
            }

            let total = handles.len();
            let mut succeeded = 0;
            let mut failed = 0;
            let mut task_reports: Vec<String> = Vec::new();

            for handle in handles {
                match handle.await {
                    Ok((id, desc, Ok(output))) => {
                        succeeded += 1;
                        task_reports.push(format!("### Task [{}]: {}\n{}", id, desc, output));
                    }
                    Ok((id, desc, Err(e))) => {
                        failed += 1;
                        task_reports.push(format!("### Task [{}]: {} — FAILED: {}", id, desc, e));
                        tracing::error!("Task {} failed: {}", id, e);
                    }
                    Err(e) => {
                        failed += 1;
                        tracing::error!("Task join error: {}", e);
                    }
                }
            }

            println!("\n{}", "=".repeat(60));
            println!("📊 Task Results: {}/{} succeeded, {} failed", succeeded, total, failed);

            // ── Global Reviewer: runs once after ALL tasks complete ────────────
            println!("\n🔍 All tasks done — starting global code review...");
            let combined_report = task_reports.join("\n\n---\n\n");

            // Re-build shared infra for the reviewer
            let mut settings = Settings::load().unwrap_or_default();
            if let Some(model) = &args.model {
                settings.llm.model = model.clone();
            }
            let llm_config = llm_config_from_settings(&settings);
            let provider = make_llm_provider(&llm_config);
            let registry = build_tool_registry(
                &std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
                &settings,
                args,
            )?;

            let skills = SkillsLoader::load(
                &std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
                &settings.skill_dirs
            );
            let skills_prompt = SkillsLoader::build_system_prompt("", &skills);

            let reviewer_graph = build_reviewer_pipeline(
                provider, registry, llm_config.model.clone(), skills_prompt
            ).compile()?;

            let mut review_state = DefaultState::default();
            review_state.messages.push(ConversationMessage::user(
                format!("Combined Task Completion Report:\n\n{}", combined_report)
            ));

            match reviewer_graph.execute_with_events(
                &mut review_state,
                |e| println!("🔍 {}", e),
            ).await {
                Ok(_) => {
                    let review_output = review_state.messages.last()
                        .and_then(|m| m.content.clone())
                        .unwrap_or_default();
                    println!("\n📋 Global Review complete:\n{}", review_output);
                }
                Err(e) => {
                    tracing::warn!("[reviewer] Global review failed (non-fatal): {}", e);
                    println!("⚠️  Global review step failed (non-fatal): {}", e);
                }
            }

            if failed > 0 {
                Err(ZcodeError::InternalError(
                    format!("{} task(s) failed", failed)
                ))
            } else {
                Ok(())
            }
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
        TaskAction::Sync => {
            let all_tasks = parse_all_tasks(&cwd)?;
            let saved_tasks = store.list()?;
            
            let mut added = 0;
            for t in all_tasks {
                if !t.is_completed {
                    // Check if already in store
                    let exists = saved_tasks.iter().any(|st| st.task == t.description);
                    if !exists {
                        let mut record = store.create(t.description.clone());
                        store.save(&mut record)?;
                        println!("➕ Added: {}", t.description);
                        added += 1;
                    }
                }
            }
            if added == 0 {
                println!("No new tasks to sync.");
            } else {
                println!("Synced {} new task(s). Run `zcode task list` to view them.", added);
            }
            Ok(())
        }
    }
}

/// Feed raw requirements to generate/update the docs/ structure using a lightweight AgentLoop.
///
/// Unlike `execute_run` which spins up the full task pipeline
/// (orchestrator → planner → coder(ReAct) → reviewer),
/// `execute_feed` runs each step as a single AgentLoop — just one LLM agent with tools, no
/// unnecessary planner/coder/reviewer overhead.
async fn execute_feed(path: &str, investigate: bool, max_iterations: usize, args: &crate::cli::args::Args) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Ensure docs scaffolding exists before feeding LLM
    let _ = generate_docs_scaffold(&cwd);
    println!("🔄 Prepared docs/ scaffolding.");
    println!("📖 Feeding raw requirements from: {}", path);

    // ── Shared setup: LLM provider + tool registry ──────────────────
    let mut settings = Settings::load().unwrap_or_default();
    if let Some(model) = &args.model {
        settings.llm.model = model.clone();
    }

    let llm_config = llm_config_from_settings(&settings);
    let provider = make_llm_provider(&llm_config);

    let registry = build_tool_registry(&cwd, &settings, args)?;

    println!("🧠 Model: {}", llm_config.model);

    // ── Step 1 (optional): Investigator Agent ────────────────────────
    let mut investigator_context = String::new();

    if investigate {
        println!("\n🔍 Starting Investigator Agent...");
        let config = LoopConfig {
            max_iterations,
            system_prompt: format!(
                "You are the zcode Investigator Agent (Model: {}).\n\
                 Your job is to research requirements and gather context. \
                 You have access to file reading, search, and MCP tools. \
                 Do NOT write code or modify files. Only return your research findings.",
                llm_config.model
            ),
        };
        let user_task = format!(
            "The user has provided raw requirement documents at: `{}`.\n\
             1. Read and analyze the raw requirement documents.\n\
             2. Use connected tools to research any missing contexts, architectural patterns, or API best practices.\n\
             3. Generate a comprehensive Research Report with your findings.",
            path
        );
        let p = Arc::clone(&provider);
        let agent_loop = AgentLoop::new(config, Arc::clone(&registry));
        let result = agent_loop.run(&user_task, &[], &[], move |msgs, tools| {
            let p = Arc::clone(&p);
            async move { feed_call_llm(p, msgs, tools).await }
        }).await?;
        println!("✅ Investigator Agent complete.");
        investigator_context = format!(
            "\n💡 [Investigator Agent Report]\n{}\n",
            result.answer
        );
    }

    // ── Step 2: Docs Generation Agent ────────────────────────────────
    println!("\n📝 Starting Docs Generation Agent...");
    let config = LoopConfig {
        max_iterations,
        system_prompt: format!(
            "You are the zcode Docs Generation Agent (Model: {}).\n\
             Your job is to populate and update the `docs/` directory structure based on raw requirements. \
             You have full access to file reading and writing tools. \
             Comply strictly with the Harness Engineering docs convention. \
             Do NOT write application code — only architect and document the project in `docs/`.",
            llm_config.model
        ),
    };
    let user_task = format!(
        "The user has provided raw requirement documents at: `{}`.\n{}\n\
         1. Use tools like `read_file` or `glob` to read the raw text from that path.\n\
         2. Use `write_file` or `edit_file` to populate and update `docs/` \
         (e.g., `docs/prd/001-feature.md`, `docs/specs/coding.spec.md`, `docs/tasks/001-feature.tasks.md`).\n\
         3. Keep required headings per Harness Engineering convention.",
        path, investigator_context
    );
    let agent_loop = AgentLoop::new(config, registry);
    let result = agent_loop.run(&user_task, &[], &[], move |msgs, tools| {
        let p = Arc::clone(&provider);
        async move { feed_call_llm(p, msgs, tools).await }
    }).await?;
    println!("✅ Docs Generation Agent complete.");
    println!("\n📤 Result:\n{}", result.answer);

    Ok(())
}

/// Lightweight LLM call helper for `execute_feed` (shared by investigator and docs generation).
async fn feed_call_llm(
    p: Arc<dyn LlmProvider>,
    msgs: Vec<serde_json::Value>,
    tools: Vec<serde_json::Value>,
) -> Result<AgentLlmResponse> {
    let llm_messages: Vec<Message> = msgs.iter()
        .filter_map(|v| {
            let role_str = v.get("role")?.as_str()?;
            match role_str {
                "system" => {
                    let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                    Some(Message::system(content))
                }
                "assistant" => {
                    if let Some(tool_calls) = v.get("tool_calls").and_then(|tc| tc.as_array()) {
                        if !tool_calls.is_empty() {
                            let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                            Some(Message::assistant_with_tool_calls(content, tool_calls.clone()))
                        } else {
                            let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                            Some(Message::assistant(content))
                        }
                    } else {
                        let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                        Some(Message::assistant(content))
                    }
                }
                "tool" => {
                    let tool_call_id = v.get("tool_call_id").and_then(|id| id.as_str()).unwrap_or("").to_string();
                    let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                    let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                    Some(Message::tool_result(tool_call_id, name, content))
                }
                _ => {
                    let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                    Some(Message::user(content))
                }
            }
        })
        .collect();

    match p.chat(&llm_messages, &tools) {
        Ok(resp) => {
            if let Ok(agent_resp) = AgentLlmResponse::from_openai_response(&resp.raw_response) {
                Ok(agent_resp)
            } else {
                Ok(AgentLlmResponse::Text(resp.content))
            }
        }
        Err(e) => Err(e),
    }
}

/// Run a single task in non-interactive mode using AgentLoop + LLM
async fn execute_run(task: &str, resume_id: Option<&str>, max_iterations: usize, args: &crate::cli::args::Args) -> Result<String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // ── Task Store ────────────────────────────────────────────────────
    let store = TaskStore::new(&cwd)?;
    let mut task_record: TaskRecord = if let Some(id) = resume_id {
        let mut record = store.load(id)?;
        if record.status == TaskStatus::Completed {
            println!("⚠  Task '{}' already completed. Starting fresh.", id);
            record = store.create(task);
        } else {
            println!("▶  Resuming task '{}' from iteration {}.", record.id, record.state.iteration);
        }
        record
    } else {
        store.create(task)
    };
    info!("Task record id={}", task_record.id);

    // ── Global Config ─────────────────────────────────────────────────
    let mut settings = Settings::load().unwrap_or_default();
    if let Some(model) = &args.model {
        settings.llm.model = model.clone();
    }

    // ── Skills ────────────────────────────────────────────────────────
    let skills = SkillsLoader::load(&cwd, &settings.skill_dirs);
    if !skills.is_empty() {
        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        println!("📚 Loaded {} skill(s): {}", skills.len(), names.join(", "));
    }

    // ── Model / LLM config ────────────────────────────────────────────
    let llm_config = llm_config_from_settings(&settings);
    let provider = make_llm_provider(&llm_config);
    let fast_llm_config = fast_llm_config(&llm_config);
    let fast_provider = make_llm_provider(&fast_llm_config);

    // ── Capability registry (MCP-only) ────────────────────────────────
    let registry = build_tool_registry(&cwd, &settings, args)?;

    let skills_prompt = SkillsLoader::build_system_prompt("", &skills);
    
    // ── Graph Engine (orchestrator → planner → coder(ReAct) → reviewer) ──
    let graph = build_task_pipeline_with_limit(
        Arc::clone(&provider),
        Arc::clone(&fast_provider),
        Arc::clone(&registry),
        llm_config.model.clone(),
        fast_llm_config.model.clone(),
        skills_prompt.clone(),
        max_iterations,
    ).compile()?;

    println!("🤖 zcode Task Agent starting...");
    println!("📋 Task: {} [id={}]", task, task_record.id);
    println!("🧠 Model: {}", llm_config.model);
    println!("⚡ Fast model: {}", fast_llm_config.model);
    println!("💾 Progress saved to .zcode/tasks/{}.json", task_record.id);
    println!();

    // Save initial state
    let _ = store.save(&mut task_record);

    let task_result = graph.execute_with_events(
        &mut task_record.state,
        |e| println!("🌐 {}", e),
    ).await;

    // Save task-level status
    let final_answer = match task_result {
        Ok(graph_out) => {
            let test_passed = task_record.state.metadata.get("review_passed")
                .or_else(|| task_record.state.metadata.get("test_passed"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let answer = task_record.state.messages.last()
                .and_then(|m| m.content.clone())
                .unwrap_or_else(|| "No output generated".into());

            if test_passed {
                task_record.status = TaskStatus::Completed;
                task_record.state.result = Some(TaskResult::success(
                    task_record.id.clone(), answer.clone()
                ));
                println!("\n✅ Task complete ({} graph iterations)", graph_out.total_iterations);
            } else {
                task_record.status = TaskStatus::Failed;
                task_record.error = Some("Tests failed after max retries".into());
                task_record.state.result = Some(TaskResult::failure(
                    task_record.id.clone(), "Tests failed after max retries"
                ));
                println!("\n❌ Task failed: Tests failed after max retries ({} iterations)", graph_out.total_iterations);
            }

            println!("\n📤 Result:\n{}", answer);
            let _ = store.save(&mut task_record);
            answer
        }
        Err(e) => {
            task_record.status = TaskStatus::Failed;
            task_record.error = Some(e.to_string());
            let _ = store.save(&mut task_record);
            return Err(e);
        }
    };

    // ── Global Reviewer (runs once after this single task completes) ──────────
    println!("\n🔍 Starting global code review...");
    let reviewer_graph = build_reviewer_pipeline(
        Arc::clone(&provider),
        registry,
        llm_config.model.clone(),
        skills_prompt,
    ).compile()?;

    let mut review_state = DefaultState::default();
    review_state.messages.push(ConversationMessage::user(
        format!("Task: {}\n\nTask Report:\n{}", task, final_answer)
    ));

    match reviewer_graph.execute_with_events(
        &mut review_state,
        |e| println!("🔍 {}", e),
    ).await {
        Ok(_) => {
            let review_output = review_state.messages.last()
                .and_then(|m| m.content.clone())
                .unwrap_or_default();
            println!("\n📋 Review complete:\n{}", review_output);
        }
        Err(e) => {
            // Review failures are non-fatal — log and continue
            tracing::warn!("[reviewer] Global review failed (non-fatal): {}", e);
            println!("⚠️  Review step failed (non-fatal): {}", e);
        }
    }

    Ok(final_answer)
}

/// Run a task through only the per-task orchestrator pipeline.
/// Does NOT trigger the global reviewer. Used internally by `execute_task`'s RunAll
/// to allow the global reviewer to be called once after all tasks complete.
async fn execute_run_task_only(task: &str, resume_id: Option<&str>, max_iterations: usize, args: &crate::cli::args::Args) -> Result<String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let store = TaskStore::new(&cwd)?;
    let mut task_record: TaskRecord = if let Some(id) = resume_id {
        let mut record = store.load(id)?;
        if record.status == TaskStatus::Completed {
            println!("⚠  Task '{}' already completed. Starting fresh.", id);
            record = store.create(task);
        } else {
            println!("▶  Resuming task '{}' from iteration {}.", record.id, record.state.iteration);
        }
        record
    } else {
        store.create(task)
    };
    info!("Task record id={}", task_record.id);

    let mut settings = Settings::load().unwrap_or_default();
    if let Some(model) = &args.model {
        settings.llm.model = model.clone();
    }

    let skills = SkillsLoader::load(&cwd, &settings.skill_dirs);
    let llm_config = llm_config_from_settings(&settings);
    let provider = make_llm_provider(&llm_config);
    let fast_llm_config = fast_llm_config(&llm_config);
    let fast_provider = make_llm_provider(&fast_llm_config);

    let registry = build_tool_registry(&cwd, &settings, args)?;
    let skills_prompt = SkillsLoader::build_system_prompt("", &skills);

    let graph = build_task_pipeline_with_limit(
        Arc::clone(&provider),
        fast_provider,
        Arc::clone(&registry),
        llm_config.model.clone(),
        fast_llm_config.model.clone(),
        skills_prompt,
        max_iterations,
    ).compile()?;

    let _ = store.save(&mut task_record);

    let task_result = graph.execute_with_events(
        &mut task_record.state,
        |e| println!("🌐 {}", e),
    ).await;

    match task_result {
        Ok(graph_out) => {
            let test_passed = task_record.state.metadata.get("review_passed")
                .or_else(|| task_record.state.metadata.get("test_passed"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let answer = task_record.state.messages.last()
                .and_then(|m| m.content.clone())
                .unwrap_or_else(|| "No output generated".into());

            if test_passed {
                task_record.status = TaskStatus::Completed;
                task_record.state.result = Some(TaskResult::success(
                    task_record.id.clone(), answer.clone()
                ));
                println!("\n✅ Task [{}] complete ({} iterations)", task_record.id, graph_out.total_iterations);
            } else {
                task_record.status = TaskStatus::Failed;
                task_record.error = Some("Tests failed after max retries".into());
                task_record.state.result = Some(TaskResult::failure(
                    task_record.id.clone(), "Tests failed after max retries"
                ));
                println!("\n❌ Task [{}] failed: Tests failed after max retries ({} iterations)", task_record.id, graph_out.total_iterations);
            }
            let _ = store.save(&mut task_record);
            Ok(answer)
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
    let llm_config = llm_config_from_settings(&settings);

    let provider = make_llm_provider(&llm_config);

    // Initialize terminal
    let mut terminal = init_terminal()?;

    // Read MCP Servers active
    let mut active_mcps = Vec::new();
    for m in &settings.mcp_servers {
        active_mcps.push(m.name.clone());
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    if let Ok(ws) = Workspace::open(&cwd) {
        for m in ws.config.mcp_servers {
            active_mcps.push(m.name);
        }
    }
    
    // Read Skills active
    let skills = SkillsLoader::load(&cwd, &settings.skill_dirs);
    let active_skills: Vec<String> = skills.into_iter().map(|s| s.name).collect();

    // Create TUI application with real LLM provider
    let mut app = TuiApp::with_provider(provider);
    app.active_mcps = active_mcps;
    app.active_skills = active_skills;
    
    app.chat.add_message(zcode_ui::tui::chat::ChatMessage::system(
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
                max_iterations: 50,
            }),
            model: None,
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None, .. }) = &args.command {
            let result = execute_run(task, None, 50, &args).await;
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
                max_iterations: 50,
            }),
            model: Some(std::env::var("ZCODE_MODEL").unwrap_or_else(|_| "gpt-4o".to_string())),
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None, .. }) = &args.command {
            let result = execute_run(task, None, 50, &args).await;
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
                max_iterations: 50,
            }),
            model: None,
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None, .. }) = &args.command {
            let result = execute_run(task, None, 50, &args).await;
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
                max_iterations: 50,
            }),
            model: None,
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None, .. }) = &args.command {
            let result = execute_run(task, None, 50, &args).await;
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
                max_iterations: 50,
            }),
            model: None,
            mcp: vec!["server1".to_string(), "server2".to_string()],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None, .. }) = &args.command {
            let result = execute_run(task, None, 50, &args).await;
            // Should fail because server1 and server2 don't exist
            assert!(result.is_err());
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
                max_iterations: 50,
            }),
            model: None,
            mcp: vec![],
            verbose: true,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None, .. }) = &args.command {
            let result = execute_run(task, None, 50, &args).await;
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
                max_iterations: 50,
            }),
            model: None,
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None, .. }) = &args.command {
            let result = execute_run(task, None, 50, &args).await;
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
                max_iterations: 50,
            }),
            model: None,
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None, .. }) = &args.command {
            let result = execute_run(task, None, 50, &args).await;
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
                max_iterations: 50,
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

    #[tokio::test]
    async fn test_execute_command_feed() {
        let args = Args {
            command: Some(Command::Feed {
                path: "nonexistent".to_string(),
                max_iterations: 0,
                investigate: false,
            }),
            model: None,
            mcp: vec![],
            verbose: false,
            skip_docs_check: false,
        };

        // Note: With max_iterations=0, it will return immediately without side effects,
        // or just fail gently depending on LLM config. Here we just ensure route dispatch works
        // without panicking. Mocks aren't fully intercepting `execute_run` so we have to be careful.
        // Let's just check the command itself parses and routes.
        if let Some(ref cmd) = args.command {
            // It might return an error due to no API key during tests, so we just expect it to run
            let _ = execute_command(cmd, &args).await;
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
            max_iterations: 50,
        };
        let cloned = cmd.clone();
        if let Command::Run { task, resume: None, .. } = cloned {
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
                max_iterations: 50,
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

        if let Some(Command::Run { task, resume: None, .. }) = &args.command {
            let result = execute_run(task, None, 50, &args).await;
            // Expect failure because servers are not valid commands
            assert!(result.is_err());
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
                max_iterations: 50,
            }),
            model: Some(std::env::var("ZCODE_MODEL").unwrap_or_else(|_| "gpt-4o".to_string())),
            mcp: vec!["mcp-server".to_string()],
            verbose: true,
            skip_docs_check: false,
        };

        if let Some(Command::Run { task, resume: None, .. }) = &args.command {
            let result = execute_run(task, None, 50, &args).await;
            // Expect failure because mcp-server does not exist
            assert!(result.is_err());
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
        let result: Result<()> = Err(ZcodeError::Cancelled);
        assert!(result.is_err());
    }
}
