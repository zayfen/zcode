//! Command handlers for zcode CLI
//!
//! This module implements the handlers for each CLI command.

use crate::args::{Args, Command, DocsAction, TaskAction};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use tracing::info;
use zcode_capabilities::{register_workspace_tools, AskUserTool, McpClient, SkillsLoader, ToolRegistry};
use zcode_core::{AskUserSender, LlmConfig, ProjectConfig, Result, Settings, ZcodeError};
#[cfg(test)]
use zcode_llm_provider::MockLlmProvider;
#[cfg(not(test))]
use zcode_llm_provider::RigProvider;
use zcode_llm_provider::{LlmProvider, Message};
use zcode_orchestration::{
    build_reviewer_pipeline_with_runtime, build_task_pipeline_with_limit, AgentLoop,
    AgentModelLabels, AgentRuntime, ConversationMessage, DefaultState, GraphEvent,
    LlmResponse as AgentLlmResponse, LoopConfig, LoopEvent, TaskAgentRuntimes, TaskResult,
};
use zcode_requirements::docs::parser::parse_all_tasks;
use zcode_requirements::{
    generate_docs_scaffold, DocsValidator, TaskRecord, TaskStatus, TaskStore,
};
use zcode_ui::{init_terminal, restore_terminal, TaskExecutor, TaskRequest, TaskUiEvent, TuiApp};

/// Execute a CLI command
pub async fn execute_command(command: &Command, args: &Args) -> Result<()> {
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
        Command::Run {
            task,
            resume,
            max_iterations,
        } => {
            let _ = execute_run(task, resume.as_deref(), *max_iterations, args).await?;
            Ok(())
        }
        Command::Feed {
            path,
            max_iterations,
            investigate,
        } => execute_feed(path, *investigate, *max_iterations, args).await,
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
        base_url: settings.llm.base_url.clone(),
        api_key: settings.llm.api_key.clone(),
        api_key_env: settings.llm.api_key_env.clone(),
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

#[derive(Clone)]
struct AgentLlm {
    provider: Arc<dyn LlmProvider>,
    config: LlmConfig,
    explicit_model: bool,
}

#[derive(Clone)]
struct AgentProviders {
    fast: AgentLlm,
    planner: AgentLlm,
    coder: AgentLlm,
    reviewer: AgentLlm,
    investigator: AgentLlm,
    docs: AgentLlm,
}

impl AgentProviders {
    fn task_models(&self) -> AgentModelLabels {
        AgentModelLabels {
            investigator: self.investigator.config.model.clone(),
            planner: self.planner.config.model.clone(),
            coder: self.coder.config.model.clone(),
            reviewer: self.reviewer.config.model.clone(),
            fast: self.fast.config.model.clone(),
        }
    }

    fn task_runtimes(
        &self,
        event_sink: Option<Arc<dyn Fn(GraphEvent) + Send + Sync>>,
    ) -> TaskAgentRuntimes {
        let attach_sink = |runtime: AgentRuntime| {
            if let Some(event_sink) = &event_sink {
                runtime.with_event_sink(Arc::clone(event_sink))
            } else {
                runtime
            }
        };

        TaskAgentRuntimes {
            investigator: attach_sink(AgentRuntime::new(
                Arc::clone(&self.investigator.provider),
                self.investigator.config.model.clone(),
                self.investigator.explicit_model,
            )),
            planner: attach_sink(AgentRuntime::new(
                Arc::clone(&self.planner.provider),
                self.planner.config.model.clone(),
                self.planner.explicit_model,
            )),
            coder: attach_sink(AgentRuntime::new(
                Arc::clone(&self.coder.provider),
                self.coder.config.model.clone(),
                self.coder.explicit_model,
            )),
            reviewer: attach_sink(AgentRuntime::new(
                Arc::clone(&self.reviewer.provider),
                self.reviewer.config.model.clone(),
                self.reviewer.explicit_model,
            )),
            fast: AgentRuntime::new(
                Arc::clone(&self.fast.provider),
                self.fast.config.model.clone(),
                self.fast.explicit_model,
            ),
        }
    }

    fn reviewer_runtime(
        &self,
        event_sink: Option<Arc<dyn Fn(GraphEvent) + Send + Sync>>,
    ) -> AgentRuntime {
        let runtime = AgentRuntime::new(
            Arc::clone(&self.reviewer.provider),
            self.reviewer.config.model.clone(),
            self.reviewer.explicit_model,
        );
        if let Some(event_sink) = event_sink {
            runtime.with_event_sink(event_sink)
        } else {
            runtime
        }
    }
}

fn apply_project_llm_overrides(config: &mut LlmConfig, project_config: &ProjectConfig) {
    if let Some(llm) = &project_config.llm {
        if let Some(provider) = &llm.provider {
            config.provider = provider.clone();
        }
        if let Some(model) = &llm.model {
            config.model = model.clone();
        }
        if let Some(fast_model) = &llm.fast_model {
            config.fast_model = Some(fast_model.clone());
        }
        if let Some(base_url) = &llm.base_url {
            config.base_url = Some(base_url.clone());
        }
        if let Some(api_key) = &llm.api_key {
            config.api_key = Some(api_key.clone());
        }
        if let Some(api_key_env) = &llm.api_key_env {
            config.api_key_env = Some(api_key_env.clone());
        }
        if let Some(temperature) = llm.temperature {
            config.temperature = temperature;
        }
        if let Some(max_tokens) = llm.max_tokens {
            config.max_tokens = max_tokens;
        }
    }
}

fn parse_provider_model(value: &str) -> Result<(&str, &str)> {
    let (provider, model) = value.split_once('/').ok_or_else(|| {
        ZcodeError::ConfigError(format!(
            "agent model `{}` must use `provider/model` format",
            value
        ))
    })?;
    if provider.trim().is_empty() || model.trim().is_empty() {
        return Err(ZcodeError::ConfigError(format!(
            "agent model `{}` must include both provider and model",
            value
        )));
    }
    Ok((provider.trim(), model.trim()))
}

fn resolve_agent_config(
    agent: &str,
    default_config: &LlmConfig,
    project_config: &ProjectConfig,
) -> Result<(LlmConfig, bool)> {
    let Some(value) = project_config.agent_models.get(agent) else {
        return Ok((default_config.clone(), false));
    };
    let (provider_id, model) = parse_provider_model(value)?;
    let provider = project_config
        .llm
        .as_ref()
        .and_then(|llm| llm.providers.get(provider_id))
        .ok_or_else(|| {
            ZcodeError::ConfigError(format!(
                "agent model `{}` references unknown provider `{}`",
                value, provider_id
            ))
        })?;

    let mut config = default_config.clone();
    config.provider = provider_id.to_string();
    config.model = model.to_string();
    config.fast_model = None;
    if let Some(base_url) = &provider.base_url {
        config.base_url = Some(base_url.clone());
    }
    if let Some(api_key) = &provider.api_key {
        config.api_key = Some(api_key.clone());
    } else {
        config.api_key = None;
    }
    if let Some(api_key_env) = &provider.api_key_env {
        config.api_key_env = Some(api_key_env.clone());
    } else {
        config.api_key_env = None;
    }
    if let Some(temperature) = provider.temperature {
        config.temperature = temperature;
    }
    if let Some(max_tokens) = provider.max_tokens {
        config.max_tokens = max_tokens;
    }

    Ok((config, true))
}

fn build_agent_providers(
    settings: &Settings,
    project_config: &ProjectConfig,
) -> Result<AgentProviders> {
    let mut default_config = llm_config_from_settings(settings);
    apply_project_llm_overrides(&mut default_config, project_config);
    let fast_config = fast_llm_config(&default_config);

    let fast = AgentLlm {
        provider: make_llm_provider(&fast_config),
        config: fast_config,
        explicit_model: false,
    };

    let make_agent = |agent: &str| -> Result<AgentLlm> {
        let (config, explicit_model) =
            resolve_agent_config(agent, &default_config, project_config)?;
        Ok(AgentLlm {
            provider: make_llm_provider(&config),
            config,
            explicit_model,
        })
    };

    Ok(AgentProviders {
        fast,
        planner: make_agent("planner")?,
        coder: make_agent("coder")?,
        reviewer: make_agent("reviewer")?,
        investigator: make_agent("investigator")?,
        docs: make_agent("docs")?,
    })
}

fn print_loop_event(agent: &str, event: LoopEvent) {
    match event {
        LoopEvent::ToolStart { tool_name, command } => {
            println!("🔧 {} {}: {}", agent, tool_name, command);
        }
        LoopEvent::ToolComplete { tool_name, success } => {
            println!(
                "{} {} {}",
                if success { "✅" } else { "❌" },
                agent,
                tool_name
            );
        }
    }
}

fn print_task_models(models: &AgentModelLabels) {
    println!("🔎 Investigator model: {}", models.investigator);
    println!("🧠 Planner model: {}", models.planner);
    println!("🛠 Coder model: {}", models.coder);
    println!("🔍 Reviewer model: {}", models.reviewer);
    println!("⚡ Fast model: {}", models.fast);
}

fn print_cli_graph_event(prefix: &str, event: GraphEvent) {
    match event {
        GraphEvent::ToolStart {
            agent,
            tool_name,
            command,
        } => {
            println!("🔧 {} {}: {}", agent, tool_name, command);
        }
        GraphEvent::StepStart { id, title, agent } => {
            println!("▶ {} {}: {}", agent, id, title);
        }
        GraphEvent::StepComplete {
            id,
            title,
            agent,
            success,
        } => {
            println!(
                "{} {} {}: {}",
                if success { "✅" } else { "❌" },
                agent,
                id,
                title
            );
        }
        GraphEvent::ToolComplete {
            agent,
            tool_name,
            success,
        } => {
            println!(
                "{} {} {}",
                if success { "✅" } else { "❌" },
                agent,
                tool_name
            );
        }
        other => println!("{} {}", prefix, other),
    }
}

fn make_tui_tool_event_sink(
    tx: mpsc::Sender<TaskUiEvent>,
) -> Arc<dyn Fn(GraphEvent) + Send + Sync> {
    Arc::new(move |event| match event {
        GraphEvent::ToolStart {
            agent,
            tool_name,
            command,
        } => {
            let _ = tx.send(TaskUiEvent::ToolStart {
                agent,
                tool_name,
                command,
            });
        }
        GraphEvent::ToolComplete {
            agent,
            tool_name,
            success,
        } => {
            let _ = tx.send(TaskUiEvent::ToolComplete {
                agent,
                tool_name,
                success,
            });
        }
        _ => {}
    })
}

#[cfg(not(test))]
fn make_llm_provider(config: &LlmConfig) -> Arc<dyn LlmProvider> {
    Arc::new(RigProvider::new(config.clone()))
}

#[cfg(test)]
fn make_llm_provider(_config: &LlmConfig) -> Arc<dyn LlmProvider> {
    Arc::new(MockLlmProvider::new("PASS"))
}

fn project_config_for(cwd: &Path) -> ProjectConfig {
    ProjectConfig::load(cwd).unwrap_or_else(|_| {
        ProjectConfig::new(
            cwd.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("project")
                .to_string(),
        )
    })
}

fn build_tool_registry(cwd: &Path, settings: &Settings, args: &Args) -> Result<Arc<ToolRegistry>> {
    let mut registry = ToolRegistry::new();
    register_workspace_tools(&mut registry, cwd)?;

    let project_config = project_config_for(cwd);

    for mcp_cfg in &settings.mcp_servers {
        if !mcp_cfg.auto_start {
            continue;
        }
        let exec_args: Vec<&str> = mcp_cfg.args.iter().map(|s| s.as_str()).collect();
        info!(
            "Starting global MCP server: {} {:?}",
            mcp_cfg.command, exec_args
        );
        let client = McpClient::connect_stdio(&mcp_cfg.name, &mcp_cfg.command, &exec_args)?;
        for adapter in Arc::new(client).create_adapters() {
            registry.register(adapter);
        }
    }

    for mcp_cfg in project_config.mcp_servers {
        if !mcp_cfg.auto_start {
            continue;
        }
        let exec_args: Vec<&str> = mcp_cfg.args.iter().map(|s| s.as_str()).collect();
        info!(
            "Starting workspace MCP server: {} {:?}",
            mcp_cfg.command, exec_args
        );
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
pub async fn execute_default(args: &Args) -> Result<()> {
    execute_chat(args).await
}

/// Handle `zcode task {list|show|clean}` commands.
async fn execute_task(action: &TaskAction, args: &Args) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let store = TaskStore::new(&cwd)?;
    match action {
        TaskAction::List => {
            let tasks = store.list()?;
            if tasks.is_empty() {
                println!("No saved tasks. Run `zcode task sync` to import from docs/tasks/.");
            } else {
                println!("{:<10} {:<12} {:<5} {}", "ID", "STATUS", "ITER", "TASK");
                println!("{}", "-".repeat(70));
                for t in &tasks {
                    let snippet = if t.task.chars().count() > 45 {
                        let truncated: String = t.task.chars().take(45).collect();
                        format!("{}…", truncated)
                    } else {
                        t.task.clone()
                    };
                    println!(
                        "{:<10} {:<12} {:<5} {}",
                        t.id,
                        t.status.to_string(),
                        t.state.iteration,
                        snippet
                    );
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
        TaskAction::Run {
            task_or_id,
            max_iterations,
        } => {
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
        TaskAction::RunAll {
            concurrency,
            max_iterations,
        } => {
            let tasks = store.list()?;
            let pending: Vec<_> = tasks
                .into_iter()
                .filter(|t| {
                    matches!(
                        t.status,
                        TaskStatus::Running | TaskStatus::Interrupted | TaskStatus::Failed
                    )
                })
                .collect();

            if pending.is_empty() {
                println!("No pending tasks. Run `zcode task sync` to import from docs/tasks/.");
                return Ok(());
            }

            println!(
                "🚀 Running {} pending task(s) with concurrency={}",
                pending.len(),
                concurrency
            );
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
                    println!(
                        "\n▶ [{}/{}] Starting: {} [id={}]",
                        i + 1,
                        total_count,
                        task_desc,
                        task_id
                    );
                    // execute_run_task_only runs the per-task orchestrator graph.
                    let result =
                        execute_run_task_only(&task_desc, Some(&task_id), max_iter, &args_clone)
                            .await;
                    match &result {
                        Ok(_) => println!("✅ [{}] Completed: {}", task_id, task_desc),
                        Err(e) => println!("❌ [{}] Failed: {} — {}", task_id, task_desc, e),
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
            println!(
                "📊 Task Results: {}/{} succeeded, {} failed",
                succeeded, total, failed
            );

            // ── Global Reviewer: runs once after ALL tasks complete ────────────
            println!("\n🔍 All tasks done — starting global code review...");
            let combined_report = task_reports.join("\n\n---\n\n");

            // Re-build shared infra for the reviewer
            let mut settings = Settings::load().unwrap_or_default();
            if let Some(model) = &args.model {
                settings.llm.model = model.clone();
            }
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let project_config = project_config_for(&cwd);
            let agent_providers = build_agent_providers(&settings, &project_config)?;
            let registry = build_tool_registry(&cwd, &settings, args)?;

            let skills = SkillsLoader::load(&cwd, &settings.skill_dirs);
            let skills_prompt = SkillsLoader::build_system_prompt("", &skills);

            let reviewer_graph = build_reviewer_pipeline_with_runtime(
                agent_providers.reviewer_runtime(Some(Arc::new(|event| {
                    print_cli_graph_event("reviewer", event)
                }))),
                registry,
                skills_prompt,
            )
            .compile()?;

            let mut review_state = DefaultState::default();
            review_state
                .messages
                .push(ConversationMessage::user(format!(
                    "Combined Task Completion Report:\n\n{}",
                    combined_report
                )));

            match reviewer_graph
                .execute_with_events(&mut review_state, |e| println!("🔍 {}", e))
                .await
            {
                Ok(_) => {
                    let review_output = review_state
                        .messages
                        .last()
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
                Err(ZcodeError::InternalError(format!(
                    "{} task(s) failed",
                    failed
                )))
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
                println!(
                    "Synced {} new task(s). Run `zcode task list` to view them.",
                    added
                );
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
async fn execute_feed(
    path: &str,
    investigate: bool,
    max_iterations: usize,
    args: &Args,
) -> Result<()> {
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

    let project_config = project_config_for(&cwd);
    let agent_providers = build_agent_providers(&settings, &project_config)?;

    let registry = build_tool_registry(&cwd, &settings, args)?;

    println!("🧠 Docs model: {}", agent_providers.docs.config.model);
    if investigate {
        println!(
            "🔎 Investigator model: {}",
            agent_providers.investigator.config.model
        );
    }

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
                agent_providers.investigator.config.model
            ),
        };
        let user_task = format!(
            "The user has provided raw requirement documents at: `{}`.\n\
             1. Read and analyze the raw requirement documents.\n\
             2. Use connected tools to research any missing contexts, architectural patterns, or API best practices.\n\
             3. Generate a comprehensive Research Report with your findings.",
            path
        );
        let p = Arc::clone(&agent_providers.investigator.provider);
        let agent_loop = AgentLoop::new(config, Arc::clone(&registry));
        let result = agent_loop
            .run_with_events(
                &user_task,
                &[],
                &[],
                move |msgs, tools| {
                    let p = Arc::clone(&p);
                    async move { feed_call_llm(p, msgs, tools).await }
                },
                |event| print_loop_event("investigator", event),
            )
            .await?;
        println!("✅ Investigator Agent complete.");
        investigator_context = format!("\n💡 [Investigator Agent Report]\n{}\n", result.answer);
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
            agent_providers.docs.config.model
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
    let docs_provider = Arc::clone(&agent_providers.docs.provider);
    let result = agent_loop
        .run_with_events(
            &user_task,
            &[],
            &[],
            move |msgs, tools| {
                let p = Arc::clone(&docs_provider);
                async move { feed_call_llm(p, msgs, tools).await }
            },
            |event| print_loop_event("docs", event),
        )
        .await?;
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
    let llm_messages: Vec<Message> = msgs
        .iter()
        .filter_map(|v| {
            let role_str = v.get("role")?.as_str()?;
            match role_str {
                "system" => {
                    let content = v
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some(Message::system(content))
                }
                "assistant" => {
                    if let Some(tool_calls) = v.get("tool_calls").and_then(|tc| tc.as_array()) {
                        if !tool_calls.is_empty() {
                            let content = v
                                .get("content")
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string();
                            let reasoning_content = v
                                .get("reasoning_content")
                                .and_then(|c| c.as_str())
                                .map(|s| s.to_string());
                            Some(Message::assistant_with_tool_calls_and_reasoning(
                                content,
                                tool_calls.clone(),
                                reasoning_content,
                            ))
                        } else {
                            let content = v
                                .get("content")
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string();
                            Some(Message::assistant(content))
                        }
                    } else {
                        let content = v
                            .get("content")
                            .and_then(|c| c.as_str())
                            .unwrap_or("")
                            .to_string();
                        Some(Message::assistant(content))
                    }
                }
                "tool" => {
                    let tool_call_id = v
                        .get("tool_call_id")
                        .and_then(|id| id.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = v
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    let content = v
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some(Message::tool_result(tool_call_id, name, content))
                }
                _ => {
                    let content = v
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
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
async fn execute_run(
    task: &str,
    resume_id: Option<&str>,
    max_iterations: usize,
    args: &Args,
) -> Result<String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // ── Task Store ────────────────────────────────────────────────────
    let store = TaskStore::new(&cwd)?;
    let mut task_record: TaskRecord = if let Some(id) = resume_id {
        let mut record = store.load(id)?;
        if record.status == TaskStatus::Completed {
            println!("⚠  Task '{}' already completed. Starting fresh.", id);
            record = store.create(task);
        } else {
            println!(
                "▶  Resuming task '{}' from iteration {}.",
                record.id, record.state.iteration
            );
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
    let project_config = project_config_for(&cwd);
    let agent_providers = build_agent_providers(&settings, &project_config)?;
    let task_models = agent_providers.task_models();

    // ── Capability registry (workspace tools + MCP) ───────────────────
    let registry = build_tool_registry(&cwd, &settings, args)?;

    let skills_prompt = SkillsLoader::build_system_prompt("", &skills);

    // ── Graph Engine (orchestrator → planner → coder(ReAct) → reviewer) ──
    let graph = build_task_pipeline_with_limit(
        agent_providers.task_runtimes(Some(Arc::new(|event| print_cli_graph_event("task", event)))),
        Arc::clone(&registry),
        skills_prompt.clone(),
        max_iterations,
    )
    .compile()?;

    println!("🤖 zcode Task Agent starting...");
    println!("📋 Task: {} [id={}]", task, task_record.id);
    print_task_models(&task_models);
    println!("💾 Progress saved to .zcode/tasks/{}.json", task_record.id);
    println!();

    // Save initial state
    let _ = store.save(&mut task_record);

    let task_result = graph
        .execute_with_events(&mut task_record.state, |e| println!("🌐 {}", e))
        .await;

    // Save task-level status
    let final_answer = match task_result {
        Ok(graph_out) => {
            let test_passed = task_record
                .state
                .metadata
                .get("review_passed")
                .or_else(|| task_record.state.metadata.get("test_passed"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let answer = task_user_answer(&task_record.state.messages);

            if test_passed {
                task_record.status = TaskStatus::Completed;
                task_record.state.result =
                    Some(TaskResult::success(task_record.id.clone(), answer.clone()));
                println!(
                    "\n✅ Task complete ({} graph iterations)",
                    graph_out.total_iterations
                );
            } else {
                task_record.status = TaskStatus::Failed;
                task_record.error = Some("Tests failed after max retries".into());
                task_record.state.result = Some(TaskResult::failure(
                    task_record.id.clone(),
                    "Tests failed after max retries",
                ));
                println!(
                    "\n❌ Task failed: Tests failed after max retries ({} iterations)",
                    graph_out.total_iterations
                );
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
    let reviewer_graph = build_reviewer_pipeline_with_runtime(
        agent_providers.reviewer_runtime(Some(Arc::new(|event| {
            print_cli_graph_event("reviewer", event)
        }))),
        registry,
        skills_prompt,
    )
    .compile()?;

    let mut review_state = DefaultState::default();
    review_state
        .messages
        .push(ConversationMessage::user(format!(
            "Task: {}\n\nTask Report:\n{}",
            task, final_answer
        )));

    match reviewer_graph
        .execute_with_events(&mut review_state, |e| println!("🔍 {}", e))
        .await
    {
        Ok(_) => {
            let review_output = review_state
                .messages
                .last()
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
async fn execute_run_task_only(
    task: &str,
    resume_id: Option<&str>,
    max_iterations: usize,
    args: &Args,
) -> Result<String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let store = TaskStore::new(&cwd)?;
    let mut task_record: TaskRecord = if let Some(id) = resume_id {
        let mut record = store.load(id)?;
        if record.status == TaskStatus::Completed {
            println!("⚠  Task '{}' already completed. Starting fresh.", id);
            record = store.create(task);
        } else {
            println!(
                "▶  Resuming task '{}' from iteration {}.",
                record.id, record.state.iteration
            );
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
    let project_config = project_config_for(&cwd);
    let agent_providers = build_agent_providers(&settings, &project_config)?;

    let registry = build_tool_registry(&cwd, &settings, args)?;
    let skills_prompt = SkillsLoader::build_system_prompt("", &skills);

    let graph = build_task_pipeline_with_limit(
        agent_providers.task_runtimes(Some(Arc::new(|event| print_cli_graph_event("task", event)))),
        Arc::clone(&registry),
        skills_prompt,
        max_iterations,
    )
    .compile()?;

    let _ = store.save(&mut task_record);

    let task_result = graph
        .execute_with_events(&mut task_record.state, |e| println!("🌐 {}", e))
        .await;

    match task_result {
        Ok(graph_out) => {
            let test_passed = task_record
                .state
                .metadata
                .get("review_passed")
                .or_else(|| task_record.state.metadata.get("test_passed"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let answer = task_user_answer(&task_record.state.messages);

            if test_passed {
                task_record.status = TaskStatus::Completed;
                task_record.state.result =
                    Some(TaskResult::success(task_record.id.clone(), answer.clone()));
                println!(
                    "\n✅ Task [{}] complete ({} iterations)",
                    task_record.id, graph_out.total_iterations
                );
            } else {
                task_record.status = TaskStatus::Failed;
                task_record.error = Some("Tests failed after max retries".into());
                task_record.state.result = Some(TaskResult::failure(
                    task_record.id.clone(),
                    "Tests failed after max retries",
                ));
                println!(
                    "\n❌ Task [{}] failed: Tests failed after max retries ({} iterations)",
                    task_record.id, graph_out.total_iterations
                );
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
async fn execute_chat(args: &Args) -> Result<()> {
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

    // Read MCP Servers active
    let mut active_mcps = Vec::new();
    for m in &settings.mcp_servers {
        active_mcps.push(m.name.clone());
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let project_config = project_config_for(&cwd);
    for m in &project_config.mcp_servers {
        active_mcps.push(m.name.clone());
    }
    let agent_providers = build_agent_providers(&settings, &project_config)?;
    let task_models = agent_providers.task_models();

    // Read Skills active
    let skills = SkillsLoader::load(&cwd, &settings.skill_dirs);
    let active_skills: Vec<String> = skills.iter().map(|s| s.name.clone()).collect();
    let skills_prompt = SkillsLoader::build_system_prompt("", &skills);

    let registry_arc = build_tool_registry(&cwd, &settings, args)?;

    // Set up ask-user channel for agent clarification
    let (ask_tx, ask_rx) = std::sync::mpsc::channel();
    let ask_sender: AskUserSender = Arc::new(std::sync::Mutex::new(ask_tx));
    let registry = match Arc::try_unwrap(registry_arc) {
        Ok(mut reg) => {
            reg.register(AskUserTool::new(ask_sender));
            Arc::new(reg)
        }
        Err(arc) => {
            // Arc is shared — register through a new registry that wraps the existing one
            // (this path shouldn't happen in practice)
            arc
        }
    };

    let executor = build_tui_task_executor(
        cwd,
        agent_providers.task_runtimes(None),
        registry,
        skills_prompt,
        50,
    );

    // Create TUI application with the real orchestrator graph executor.
    let mut app = TuiApp::with_task_executor(executor);
    app.active_mcps = active_mcps;
    app.active_skills = active_skills;
    app.set_ask_receiver(ask_rx);

    app.chat
        .add_message(zcode_ui::tui::chat::ChatMessage::system(format!(
            "Planner: {} | Coder: {} | Reviewer: {} | Fast: {} | Press Esc or Ctrl+C to quit",
            task_models.planner, task_models.coder, task_models.reviewer, task_models.fast
        )));

    // Run the event loop
    let result = app.run(&mut terminal);

    // Restore terminal
    restore_terminal(&mut terminal)?;

    result
}

fn build_tui_task_executor(
    cwd: std::path::PathBuf,
    runtimes: TaskAgentRuntimes,
    registry: Arc<ToolRegistry>,
    skills_prompt: String,
    max_iterations: usize,
) -> TaskExecutor {
    Arc::new(
        move |request: TaskRequest, cancel: Arc<AtomicBool>, tx: mpsc::Sender<TaskUiEvent>| {
            let cwd = cwd.clone();
            let runtimes = runtimes.clone();
            let registry = Arc::clone(&registry);
            let skills_prompt = skills_prompt.clone();

            let runtime = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = tx.send(TaskUiEvent::Error(format!(
                        "Failed to create task runtime: {}",
                        error
                    )));
                    return;
                }
            };

            runtime.block_on(async move {
                run_tui_task(
                    request,
                    cwd,
                    runtimes,
                    registry,
                    skills_prompt,
                    max_iterations,
                    cancel,
                    tx,
                )
                .await;
            });
        },
    )
}

fn seed_task_history(task_record: &mut TaskRecord, history: Vec<ConversationMessage>) {
    if history.is_empty() {
        return;
    }
    task_record.state.messages.push(ConversationMessage::system(
        "Previous conversation is context only. Use it only when it helps answer the current user task. Do not repeat unrelated prior results unless the current user task asks for them.",
    ));
    task_record.state.messages.extend(history);
}

fn task_user_answer(messages: &[ConversationMessage]) -> String {
    find_report(messages, "CODER_REPORT")
        .or_else(|| find_report(messages, "INVESTIGATOR_REPORT"))
        .or_else(|| {
            messages
                .iter()
                .rev()
                .filter(|message| message.role == "assistant")
                .filter_map(|message| message.content.as_deref())
                .find(|content| !is_internal_report(content))
                .map(str::to_string)
        })
        .unwrap_or_else(|| "No output generated".to_string())
}

fn find_report(messages: &[ConversationMessage], prefix: &str) -> Option<String> {
    messages
        .iter()
        .rev()
        .filter(|message| message.role == "assistant")
        .filter_map(|message| message.content.as_deref())
        .find_map(|content| strip_report_prefix(content, prefix).map(str::to_string))
}

fn strip_report_prefix<'a>(content: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = content.strip_prefix(prefix)?;
    let rest = rest.strip_prefix(':')?;
    Some(rest.strip_prefix('\n').unwrap_or(rest).trim())
}

fn is_internal_report(content: &str) -> bool {
    [
        "INVESTIGATOR_REPORT:",
        "PLANNER_REPORT:",
        "CODER_REPORT:",
        "REVIEWER_REPORT:",
        "REVIEW_FEEDBACK:",
        "SELF_LEARNING:",
    ]
    .iter()
    .any(|prefix| content.starts_with(prefix))
}

#[allow(clippy::too_many_arguments)]
async fn run_tui_task(
    request: TaskRequest,
    cwd: std::path::PathBuf,
    runtimes: TaskAgentRuntimes,
    registry: Arc<ToolRegistry>,
    skills_prompt: String,
    max_iterations: usize,
    cancel: Arc<AtomicBool>,
    tx: mpsc::Sender<TaskUiEvent>,
) {
    if cancel.load(Ordering::SeqCst) {
        let _ = tx.send(TaskUiEvent::Cancelled);
        return;
    }

    let task = request.prompt;
    let history = request.history;
    let store = match TaskStore::new(&cwd) {
        Ok(store) => store,
        Err(error) => {
            let _ = tx.send(TaskUiEvent::Error(error.to_string()));
            return;
        }
    };
    let mut task_record = store.create(task.clone());
    seed_task_history(&mut task_record, history);
    let _ = store.save(&mut task_record);
    let _ = tx.send(TaskUiEvent::Thinking(format!(
        "Task `{}` saved to .zcode/tasks/{}.json\n",
        task, task_record.id
    )));

    let graph = match build_task_pipeline_with_limit(
        runtimes.map_event_sink(make_tui_tool_event_sink(tx.clone())),
        Arc::clone(&registry),
        skills_prompt,
        max_iterations,
    )
    .compile()
    {
        Ok(graph) => graph,
        Err(error) => {
            let _ = tx.send(TaskUiEvent::Error(error.to_string()));
            return;
        }
    };

    let graph_result = graph
        .execute_with_events_and_cancel(&mut task_record.state, |event| {
            send_graph_event(&tx, event);
            cancel.load(Ordering::SeqCst)
        })
        .await;

    if cancel.load(Ordering::SeqCst) {
        task_record.status = TaskStatus::Interrupted;
        task_record.error = Some("Interrupted by user".to_string());
        let _ = store.save(&mut task_record);
        let _ = tx.send(TaskUiEvent::Cancelled);
        return;
    }

    match graph_result {
        Ok(graph_out) => {
            let review_passed = task_record
                .state
                .metadata
                .get("review_passed")
                .or_else(|| task_record.state.metadata.get("test_passed"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let answer = task_user_answer(&task_record.state.messages);

            if review_passed {
                task_record.status = TaskStatus::Completed;
                task_record.state.result =
                    Some(TaskResult::success(task_record.id.clone(), answer.clone()));
            } else {
                task_record.status = TaskStatus::Failed;
                task_record.error = Some("Review/tests failed after max retries".to_string());
                task_record.state.result = Some(TaskResult::failure(
                    task_record.id.clone(),
                    "Review/tests failed after max retries",
                ));
            }
            let _ = store.save(&mut task_record);

            let status = if review_passed { "completed" } else { "failed" };
            let final_answer = format!(
                "{}\n\nTask `{}` {} after {} graph iteration(s).",
                answer, task_record.id, status, graph_out.total_iterations
            );
            let _ = tx.send(TaskUiEvent::Done(final_answer));
        }
        Err(error) => {
            task_record.status = TaskStatus::Failed;
            task_record.error = Some(error.to_string());
            let _ = store.save(&mut task_record);
            let _ = tx.send(TaskUiEvent::Error(error.to_string()));
        }
    }
}

fn send_graph_event(tx: &mpsc::Sender<TaskUiEvent>, event: GraphEvent) {
    match event {
        GraphEvent::NodeStart { node, iteration } => {
            let _ = tx.send(TaskUiEvent::AgentStart(node.clone()));
            let _ = tx.send(TaskUiEvent::Thinking(format!(
                "{} started (iteration {})\n",
                node, iteration
            )));
        }
        GraphEvent::StepStart { id, title, agent } => {
            let _ = tx.send(TaskUiEvent::StepStart { id, title, agent });
        }
        GraphEvent::StepComplete {
            id,
            title,
            agent,
            success,
        } => {
            let _ = tx.send(TaskUiEvent::StepComplete {
                id,
                title,
                agent,
                success,
            });
        }
        GraphEvent::NodeComplete { node, .. } => {
            let _ = tx.send(TaskUiEvent::AgentComplete(node.clone()));
            let _ = tx.send(TaskUiEvent::Thinking(format!("{} completed\n", node)));
        }
        GraphEvent::EdgeTraversed { from, to } => {
            let target = to.unwrap_or_else(|| "END".to_string());
            let _ = tx.send(TaskUiEvent::Thinking(format!("{} -> {}\n", from, target)));
        }
        GraphEvent::ToolStart {
            agent,
            tool_name,
            command,
        } => {
            let _ = tx.send(TaskUiEvent::ToolStart {
                agent: agent.clone(),
                tool_name: tool_name.clone(),
                command: command.clone(),
            });
            let _ = tx.send(TaskUiEvent::Thinking(format!(
                "{} {}: {}\n",
                agent, tool_name, command
            )));
        }
        GraphEvent::ToolComplete {
            agent,
            tool_name,
            success,
        } => {
            let _ = tx.send(TaskUiEvent::ToolComplete {
                agent: agent.clone(),
                tool_name: tool_name.clone(),
                success,
            });
            let _ = tx.send(TaskUiEvent::Thinking(format!(
                "{} {} {}\n",
                agent,
                tool_name,
                if success { "succeeded" } else { "failed" }
            )));
        }
        GraphEvent::End { reason, output } => {
            let _ = tx.send(TaskUiEvent::Thinking(format!(
                "graph ended: {} ({} iteration(s))\n",
                reason, output.total_iterations
            )));
        }
    }
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
    use std::collections::HashMap;
    use std::future::Future;
    use zcode_core::config::{AgentModelConfig, LlmConfigOverride, ProviderConfig};

    async fn in_temp_dir<F, Fut, T>(f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
        let _guard = LOCK.lock().await;
        let original = std::env::current_dir().expect("current dir");
        let temp = tempfile::tempdir().expect("temp dir");
        std::env::set_current_dir(temp.path()).expect("set temp cwd");
        let output = f().await;
        std::env::set_current_dir(original).expect("restore cwd");
        output
    }

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

    #[test]
    fn test_parse_provider_model() {
        let (provider, model) = parse_provider_model("openai/gpt-4o").unwrap();
        assert_eq!(provider, "openai");
        assert_eq!(model, "gpt-4o");
        assert!(parse_provider_model("gpt-4o").is_err());
    }

    #[test]
    fn test_resolve_agent_config_uses_provider_profile() {
        let default = LlmConfig {
            provider: "default".to_string(),
            model: "default-model".to_string(),
            base_url: Some("https://default.example/v1".to_string()),
            api_key_env: Some("DEFAULT_KEY".to_string()),
            ..Default::default()
        };
        let mut providers = HashMap::new();
        providers.insert(
            "deepseek".to_string(),
            ProviderConfig {
                base_url: Some("https://api.deepseek.com/v1".to_string()),
                api_key_env: Some("DEEPSEEK_API_KEY".to_string()),
                temperature: Some(0.2),
                max_tokens: Some(8192),
                ..Default::default()
            },
        );
        let mut project_config = ProjectConfig::new("test".to_string());
        project_config.llm = Some(LlmConfigOverride {
            providers,
            ..Default::default()
        });
        project_config.agent_models = AgentModelConfig {
            coder: Some("deepseek/deepseek-coder".to_string()),
            ..Default::default()
        };

        let (config, explicit) = resolve_agent_config("coder", &default, &project_config).unwrap();

        assert!(explicit);
        assert_eq!(config.provider, "deepseek");
        assert_eq!(config.model, "deepseek-coder");
        assert_eq!(
            config.base_url,
            Some("https://api.deepseek.com/v1".to_string())
        );
        assert_eq!(config.api_key_env, Some("DEEPSEEK_API_KEY".to_string()));
        assert_eq!(config.temperature, 0.2);
        assert_eq!(config.max_tokens, 8192);
    }

    #[test]
    fn test_resolve_agent_config_falls_back_to_default() {
        let default = LlmConfig {
            provider: "default".to_string(),
            model: "default-model".to_string(),
            ..Default::default()
        };
        let project_config = ProjectConfig::new("test".to_string());

        let (config, explicit) =
            resolve_agent_config("reviewer", &default, &project_config).unwrap();

        assert!(!explicit);
        assert_eq!(config.provider, "default");
        assert_eq!(config.model, "default-model");
    }

    #[test]
    fn test_seed_task_history_adds_context_without_current_prompt() {
        let mut record = TaskRecord::new("task-id".to_string(), "today weather");
        seed_task_history(
            &mut record,
            vec![
                ConversationMessage::user("list files"),
                ConversationMessage::assistant_text("Cargo.toml\nsrc"),
            ],
        );

        assert_eq!(record.state.messages.len(), 3);
        assert_eq!(record.state.messages[0].role, "system");
        assert!(record.state.messages[0]
            .content
            .as_deref()
            .unwrap()
            .contains("Previous conversation is context only"));
        assert_eq!(record.state.messages[1].role, "user");
        assert_eq!(
            record.state.messages[1].content.as_deref(),
            Some("list files")
        );
        assert_eq!(record.state.messages[2].role, "assistant");
        assert!(!record
            .state
            .messages
            .iter()
            .any(|message| message.content.as_deref() == Some("today weather")));
    }

    #[test]
    fn test_task_user_answer_prefers_task_result_over_reviewer_report() {
        let messages = vec![
            ConversationMessage::assistant_text("PLANNER_REPORT:\nPlan the read"),
            ConversationMessage::assistant_text("CODER_REPORT:\nCargo.toml\nsrc\nREADME.md"),
            ConversationMessage::assistant_text("REVIEWER_REPORT:\nPASS"),
        ];

        assert_eq!(task_user_answer(&messages), "Cargo.toml\nsrc\nREADME.md");
    }

    #[test]
    fn test_task_user_answer_uses_investigator_when_no_coder_report() {
        let messages = vec![
            ConversationMessage::assistant_text("INVESTIGATOR_REPORT:\nCargo.toml\nsrc"),
            ConversationMessage::assistant_text("REVIEWER_REPORT:\nPASS"),
        ];

        assert_eq!(task_user_answer(&messages), "Cargo.toml\nsrc");
    }

    #[test]
    fn test_task_user_answer_skips_internal_reports_for_plain_answer() {
        let messages = vec![
            ConversationMessage::assistant_text("The useful answer"),
            ConversationMessage::assistant_text("REVIEW_FEEDBACK:\nPASS"),
            ConversationMessage::assistant_text("SELF_LEARNING:\n# Note"),
        ];

        assert_eq!(task_user_answer(&messages), "The useful answer");
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

        if let Some(Command::Run {
            task, resume: None, ..
        }) = &args.command
        {
            let result = in_temp_dir(|| execute_run(task, None, 50, &args)).await;
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

        if let Some(Command::Run {
            task, resume: None, ..
        }) = &args.command
        {
            let result = in_temp_dir(|| execute_run(task, None, 50, &args)).await;
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

        if let Some(Command::Run {
            task, resume: None, ..
        }) = &args.command
        {
            let result = in_temp_dir(|| execute_run(task, None, 50, &args)).await;
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

        if let Some(Command::Run {
            task, resume: None, ..
        }) = &args.command
        {
            let result = in_temp_dir(|| execute_run(task, None, 50, &args)).await;
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

        if let Some(Command::Run {
            task, resume: None, ..
        }) = &args.command
        {
            let result = in_temp_dir(|| execute_run(task, None, 50, &args)).await;
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

        if let Some(Command::Run {
            task, resume: None, ..
        }) = &args.command
        {
            let result = in_temp_dir(|| execute_run(task, None, 50, &args)).await;
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

        if let Some(Command::Run {
            task, resume: None, ..
        }) = &args.command
        {
            let result = in_temp_dir(|| execute_run(task, None, 50, &args)).await;
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

        if let Some(Command::Run {
            task, resume: None, ..
        }) = &args.command
        {
            let result = in_temp_dir(|| execute_run(task, None, 50, &args)).await;
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
            let result = in_temp_dir(|| execute_command(cmd, &args)).await;
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
            let _ = in_temp_dir(|| execute_command(cmd, &args)).await;
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
        if let Command::Run {
            task, resume: None, ..
        } = cloned
        {
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

        if let Some(Command::Run {
            task, resume: None, ..
        }) = &args.command
        {
            let result = in_temp_dir(|| execute_run(task, None, 50, &args)).await;
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

        if let Some(Command::Run {
            task, resume: None, ..
        }) = &args.command
        {
            let result = in_temp_dir(|| execute_run(task, None, 50, &args)).await;
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
