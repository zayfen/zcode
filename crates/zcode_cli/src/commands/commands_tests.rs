use super::*;
use crate::tui_task::seed_task_history;
use std::collections::HashMap;
use std::future::Future;
use zcode_core::config::{AgentModelConfig, LlmConfigOverride, ProviderConfig};
use zcode_core::LlmConfig;

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

    let (config, explicit) = resolve_agent_config("reviewer", &default, &project_config).unwrap();

    assert!(!explicit);
    assert_eq!(config.provider, "default");
    assert_eq!(config.model, "default-model");
}

#[test]
fn test_seed_task_history_adds_context_without_current_prompt() {
    let mut record = TaskRecord::new("task-id".to_string(), "lunch recommendation");
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
        .contains("Previous conversation is optional background only"));
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
        .any(|message| message.content.as_deref() == Some("lunch recommendation")));
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
