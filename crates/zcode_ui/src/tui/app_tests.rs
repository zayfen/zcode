use super::*;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use zcode_llm_provider::{LlmProvider, LlmResponse, Message};

fn drain_until_idle(app: &mut TuiApp) {
    for _ in 0..50 {
        app.drain_task_events();
        if !app.llm_in_flight {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    app.drain_task_events();
}

// ============================================================
// TuiApp creation tests
// ============================================================

#[test]
fn test_tui_app_new() {
    let app = TuiApp::new();
    assert!(!app.should_quit);
    assert_eq!(app.active_skills.len(), 0);
    assert_eq!(app.active_mcps.len(), 0);
    assert_eq!(app.agent_statuses.len(), 5);
    assert_eq!(app.agent_statuses[0].0, "Supervisor");
    assert_eq!(app.agent_statuses[0].1, "Idle");
}

#[test]
fn test_tui_app_default() {
    let app = TuiApp::default();
    assert!(!app.should_quit);
    assert_eq!(app.agent_statuses.len(), 5);
}

// ============================================================
// TuiApp should_quit tests
// ============================================================

#[test]
fn test_tui_app_should_quit_initially_false() {
    let app = TuiApp::new();
    assert!(!app.should_quit);
}

#[test]
fn test_tui_app_should_quit_can_be_set() {
    let mut app = TuiApp::new();
    app.should_quit = true;
    assert!(app.should_quit);
}

// ============================================================
// TuiApp handle_event tests
// ============================================================

#[test]
fn test_tui_app_handle_event_ctrl_c() {
    let mut app = TuiApp::new();
    let event = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    app.handle_event(event).unwrap();
    assert!(app.should_quit);
}

#[test]
fn test_tui_app_handle_event_escape() {
    let mut app = TuiApp::new();
    let event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.handle_event(event).unwrap();
    assert!(app.should_quit);
}

#[test]
fn test_tui_app_escape_interrupts_in_flight_without_quitting() {
    let mut app = TuiApp::new();
    app.llm_in_flight = true;
    app.cancel_flag = Some(Arc::new(AtomicBool::new(false)));

    let event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    assert!(!app.should_quit);
    assert!(!app.llm_in_flight);
    assert!(app
        .chat
        .messages
        .iter()
        .any(|message| message.content.contains("interrupted")));
}

#[test]
fn test_tui_app_handle_event_enter() {
    let mut app = TuiApp::new();
    app.chat.input = "Test".to_string();

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    assert!(app.chat.input.is_empty());
    assert!(!app.should_quit);
}

#[test]
fn test_tui_app_handle_event_character() {
    let mut app = TuiApp::new();

    let event = Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    assert_eq!(app.chat.input, "a");
    assert!(!app.should_quit);
}

#[test]
fn test_tui_app_handle_event_backspace() {
    let mut app = TuiApp::new();
    app.chat.input = "Hello".to_string();
    app.chat.cursor_pos = app.chat.input.len(); // cursor at end

    let event = Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    assert_eq!(app.chat.input, "Hell");
    assert!(!app.should_quit);
}

#[test]
fn test_tui_app_handle_event_other_key() {
    let mut app = TuiApp::new();

    // Test that other key combinations don't cause issues
    let event = Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    assert!(!app.should_quit);
}

#[test]
fn test_tui_app_handle_event_multiple_characters() {
    let mut app = TuiApp::new();

    for c in "Hello".chars() {
        let event = Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        app.handle_event(event).unwrap();
    }

    assert_eq!(app.chat.input, "Hello");
}

#[test]
fn test_tui_app_handle_event_non_key_event() {
    let mut app = TuiApp::new();

    // Mouse event should be ignored
    let event = Event::Mouse(crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    });
    app.handle_event(event).unwrap();

    assert!(!app.should_quit);
}

#[test]
fn test_tui_app_handle_event_scroll() {
    let mut app = TuiApp::new();
    app.chat.rendered_message_lines = 20;
    app.chat.visible_message_height = 5;

    // Scroll down
    let event = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.handle_event(event).unwrap();
    assert_eq!(app.chat.scroll, 3);

    // Scroll up
    let event = Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    app.handle_event(event).unwrap();
    assert_eq!(app.chat.scroll, 0);
}

#[test]
fn test_tui_app_mouse_scroll() {
    let mut app = TuiApp::new();
    app.chat.rendered_message_lines = 20;
    app.chat.visible_message_height = 5;

    let event = Event::Mouse(crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::ScrollDown,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    });
    app.handle_event(event).unwrap();

    assert_eq!(app.chat.scroll, 3);
}

// ============================================================
// TuiApp chat integration tests
// ============================================================

#[test]
fn test_tui_app_chat_initially_empty() {
    let app = TuiApp::new();
    assert!(app.chat.input.is_empty());
    assert!(app.chat.messages.is_empty());
}

#[test]
fn test_tui_app_chat_can_add_messages() {
    let mut app = TuiApp::new();
    app.chat.add_message(chat::ChatMessage::system("Welcome"));
    assert_eq!(app.chat.messages.len(), 1);
}

// ============================================================
// TuiApp type alias tests
// ============================================================

#[test]
fn test_tui_backend_type() {
    // Verify type alias compiles
    fn _check_type(_: TuiBackend) {}
    // This function is just for compile-time checking
}

#[test]
fn test_tui_terminal_type() {
    // Verify type alias compiles
    fn _check_type(_: TuiTerminal) {}
    // This function is just for compile-time checking
}

// ============================================================
// TuiApp edge cases
// ============================================================

#[test]
fn test_tui_app_handle_event_empty_input_enter() {
    let mut app = TuiApp::new();

    // Enter with empty input should not add messages
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    assert!(app.chat.messages.is_empty());
}

#[test]
fn test_tui_app_handle_event_backspace_empty() {
    let mut app = TuiApp::new();

    // Backspace on empty input should not panic
    let event = Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    assert!(app.chat.input.is_empty());
}

#[test]
fn test_tui_app_handle_event_unicode_character() {
    let mut app = TuiApp::new();

    let event = Event::Key(KeyEvent::new(KeyCode::Char('你'), KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    assert_eq!(app.chat.input, "你");
}

#[test]
fn test_tui_app_full_typing_sequence() {
    let mut app = TuiApp::new();

    // Type "Hi"
    let event = Event::Key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::NONE));
    app.handle_event(event).unwrap();
    let event = Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    // Send
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    // Without a provider, the user message is preserved and a warning is shown.
    assert_eq!(app.chat.messages.len(), 2);
    assert_eq!(app.chat.messages[0].role, "user");
    assert_eq!(app.chat.messages[1].role, "assistant");
}

// ============================================================
// Event handling edge cases
// ============================================================

#[test]
fn test_tui_app_ctrl_other_keys() {
    let mut app = TuiApp::new();

    // Ctrl+A should not quit
    let event = Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    app.handle_event(event).unwrap();
    assert!(!app.should_quit);

    // Ctrl+D should not quit
    let event = Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
    app.handle_event(event).unwrap();
    assert!(!app.should_quit);
}

#[test]
fn test_tui_app_shift_modifiers() {
    let mut app = TuiApp::new();

    // Shift+Char should be handled as character
    let event = Event::Key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT));
    app.handle_event(event).unwrap();

    assert_eq!(app.chat.input, "A");
}

#[test]
fn test_tui_app_alt_modifiers() {
    let mut app = TuiApp::new();

    // Alt+Char should be ignored (no handler)
    let event = Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::ALT));
    app.handle_event(event).unwrap();

    assert!(!app.should_quit);
    assert!(app.chat.input.is_empty());
}

// ============================================================
// Mock provider for testing
// ============================================================

struct MockProvider {
    should_fail: bool,
}

impl LlmProvider for MockProvider {
    fn complete(&self, _prompt: &str) -> std::result::Result<String, ZcodeError> {
        Ok("mock response".to_string())
    }

    fn chat(
        &self,
        _messages: &[Message],
        _tools: &[serde_json::Value],
    ) -> std::result::Result<LlmResponse, ZcodeError> {
        if self.should_fail {
            Err(ZcodeError::InternalError("mock failure".to_string()))
        } else {
            Ok(LlmResponse {
                content: "mock response".to_string(),
                model: "mock-model".to_string(),
                usage: None,
                raw_response: serde_json::Value::Null,
            })
        }
    }

    fn stream_complete(
        &self,
        _prompt: &str,
    ) -> std::result::Result<zcode_llm_provider::provider::StreamingResponse, ZcodeError> {
        unimplemented!()
    }

    fn stream_chat(
        &self,
        _messages: &[Message],
        _tools: &[serde_json::Value],
    ) -> std::result::Result<zcode_llm_provider::ChatStreamingResponse, ZcodeError> {
        Ok(Box::pin(futures::stream::iter(vec![Ok(
            zcode_llm_provider::LlmStreamEvent::Content("mock response".to_string()),
        )])))
    }
}

// ============================================================
// TuiApp with_provider tests
// ============================================================

#[test]
fn test_tui_app_with_provider() {
    let provider = Arc::new(MockProvider { should_fail: false });
    let app = TuiApp::with_provider(provider);
    assert!(!app.should_quit);
    assert!(app.chat.messages.iter().any(|m| m.role == "system"));
}

#[test]
fn test_tui_app_enter_queues_prompt_while_in_flight() {
    let provider = Arc::new(MockProvider { should_fail: false });
    let mut app = TuiApp::with_provider(provider);
    app.llm_in_flight = true;
    app.chat.input = "queued prompt".to_string();
    app.chat.cursor_pos = app.chat.input.len();

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    assert_eq!(app.pending_prompts.len(), 1);
    assert_eq!(app.chat.pending_count, 1);
    assert!(app.chat.input.is_empty());
    assert!(!app
        .chat
        .messages
        .iter()
        .any(|m| m.content == "queued prompt"));
}

#[test]
fn test_tui_app_done_starts_new_assistant_turn() {
    let mut app = TuiApp::new();
    app.llm_in_flight = true;
    app.task_rx = None;
    app.chat.add_message(chat::ChatMessage::user("list files"));
    app.chat
        .add_message(chat::ChatMessage::assistant("file list"));

    let (tx, rx) = mpsc::channel();
    tx.send(TaskUiEvent::Done("new answer".to_string()))
        .unwrap();
    app.task_rx = Some(rx);

    app.drain_task_events();

    assert_eq!(app.chat.messages.len(), 3);
    assert_eq!(app.chat.messages[1].content, "file list");
    assert_eq!(app.chat.messages[2].role, "assistant");
    assert_eq!(app.chat.messages[2].content, "new answer");
}

#[test]
fn test_tui_app_step_events_update_agent_status() {
    let mut app = TuiApp::new();
    let (tx, rx) = mpsc::channel();
    tx.send(TaskUiEvent::StepStart {
        id: "step-2".to_string(),
        title: "Apply the changes".to_string(),
        agent: "coder".to_string(),
    })
    .unwrap();
    tx.send(TaskUiEvent::StepComplete {
        id: "step-2".to_string(),
        title: "Apply the changes".to_string(),
        agent: "coder".to_string(),
        success: true,
    })
    .unwrap();
    app.task_rx = Some(rx);

    app.drain_task_events();

    let coder = app
        .agent_statuses
        .iter()
        .find(|(name, _)| name == "Coder")
        .unwrap();
    assert_eq!(coder.1, "Done");
    assert!(app
        .chat
        .thinking_log
        .iter()
        .any(|line| line.contains("Apply the changes")));
}

#[test]
fn test_tui_app_internal_graph_nodes_do_not_override_step_status() {
    let mut app = TuiApp::new();
    let (tx, rx) = mpsc::channel();
    tx.send(TaskUiEvent::StepStart {
        id: "step-2".to_string(),
        title: "Apply the changes".to_string(),
        agent: "coder".to_string(),
    })
    .unwrap();
    tx.send(TaskUiEvent::AgentStart("execute_step".to_string()))
        .unwrap();
    tx.send(TaskUiEvent::AgentComplete("execute_step".to_string()))
        .unwrap();
    tx.send(TaskUiEvent::ToolStart {
        agent: "coder".to_string(),
        tool_name: "shell".to_string(),
        command: "cargo test".to_string(),
    })
    .unwrap();
    tx.send(TaskUiEvent::ToolComplete {
        agent: "coder".to_string(),
        tool_name: "shell".to_string(),
        success: true,
    })
    .unwrap();
    app.task_rx = Some(rx);

    app.drain_task_events();

    let coder = app
        .agent_statuses
        .iter()
        .find(|(name, _)| name == "Coder")
        .unwrap();
    assert_eq!(coder.1, "Apply the changes");
    let supervisor = app
        .agent_statuses
        .iter()
        .find(|(name, _)| name == "Supervisor")
        .unwrap();
    assert_eq!(supervisor.1, "Apply the changes");
    assert!(app
        .agent_statuses
        .iter()
        .all(|(name, _)| name != "Execute_step"));
}

#[test]
fn test_tui_app_task_request_omits_history_for_unrelated_prompt() {
    let (captured_tx, captured_rx) = mpsc::channel();
    let executor: TaskExecutor = Arc::new(move |request, _cancel, tx| {
        captured_tx.send(request).unwrap();
        let _ = tx.send(TaskUiEvent::Done("ok".to_string()));
    });
    let mut app = TuiApp::with_task_executor(executor);
    app.chat.add_message(chat::ChatMessage::system("status"));
    app.chat.add_message(chat::ChatMessage::user("list files"));
    app.chat
        .add_message(chat::ChatMessage::assistant("file list"));
    app.chat.input = "lunch recommendation".to_string();
    app.chat.cursor_pos = app.chat.input.len();

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    let request = captured_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .unwrap();
    assert_eq!(request.prompt, "lunch recommendation");
    assert!(request.history.is_empty());
}

#[test]
fn test_tui_app_task_request_includes_history_for_followup_prompt() {
    let (captured_tx, captured_rx) = mpsc::channel();
    let executor: TaskExecutor = Arc::new(move |request, _cancel, tx| {
        captured_tx.send(request).unwrap();
        let _ = tx.send(TaskUiEvent::Done("ok".to_string()));
    });
    let mut app = TuiApp::with_task_executor(executor);
    app.chat.add_message(chat::ChatMessage::system("status"));
    app.chat.add_message(chat::ChatMessage::user("list files"));
    app.chat
        .add_message(chat::ChatMessage::assistant("file list"));
    app.chat.input = "继续解释这个工程".to_string();
    app.chat.cursor_pos = app.chat.input.len();

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    let request = captured_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .unwrap();
    assert_eq!(request.prompt, "继续解释这个工程");
    assert_eq!(request.history.len(), 2);
    assert_eq!(request.history[0].role, "user");
    assert_eq!(request.history[0].content.as_deref(), Some("list files"));
    assert_eq!(request.history[1].role, "assistant");
    assert_eq!(request.history[1].content.as_deref(), Some("file list"));
    assert!(!request
        .history
        .iter()
        .any(|message| message.content.as_deref() == Some("lunch recommendation")));
}

#[test]
fn test_tui_app_task_request_treats_new_question_as_fresh_context() {
    let (captured_tx, captured_rx) = mpsc::channel();
    let executor: TaskExecutor = Arc::new(move |request, _cancel, tx| {
        captured_tx.send(request).unwrap();
        let _ = tx.send(TaskUiEvent::Done("ok".to_string()));
    });
    let mut app = TuiApp::with_task_executor(executor);
    app.chat
        .add_message(chat::ChatMessage::user("介绍 zcode 工程"));
    app.chat
        .add_message(chat::ChatMessage::assistant("工程介绍"));
    app.chat.input = "再问一下午餐推荐".to_string();
    app.chat.cursor_pos = app.chat.input.len();

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    let request = captured_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .unwrap();
    assert_eq!(request.prompt, "再问一下午餐推荐");
    assert!(request.history.is_empty());
}

#[test]
fn test_tui_app_followup_history_only_uses_recent_turn() {
    let (captured_tx, captured_rx) = mpsc::channel();
    let executor: TaskExecutor = Arc::new(move |request, _cancel, tx| {
        captured_tx.send(request).unwrap();
        let _ = tx.send(TaskUiEvent::Done("ok".to_string()));
    });
    let mut app = TuiApp::with_task_executor(executor);
    app.chat
        .add_message(chat::ChatMessage::user("old question"));
    app.chat
        .add_message(chat::ChatMessage::assistant("old answer"));
    app.chat.add_message(chat::ChatMessage::user("list files"));
    app.chat
        .add_message(chat::ChatMessage::assistant("file list"));
    app.chat.input = "继续解释这个工程".to_string();
    app.chat.cursor_pos = app.chat.input.len();

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    let request = captured_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .unwrap();
    assert_eq!(request.history.len(), 2);
    assert_eq!(request.history[0].content.as_deref(), Some("list files"));
    assert_eq!(request.history[1].content.as_deref(), Some("file list"));
    assert!(!request
        .history
        .iter()
        .any(|message| message.content.as_deref() == Some("old answer")));
}

#[test]
fn test_tui_app_persists_turns_to_one_jsonl_session_file() {
    let temp = tempfile::TempDir::new().unwrap();
    let executor: TaskExecutor = Arc::new(move |_request, _cancel, tx| {
        let _ = tx.send(TaskUiEvent::Done("project answer".to_string()));
    });
    let mut app = TuiApp::with_task_executor(executor);
    app.project_root = temp.path().to_path_buf();
    app.chat.input = "介绍 zcode 工程结构".to_string();
    app.chat.cursor_pos = app.chat.input.len();

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();
    drain_until_idle(&mut app);

    app.chat.input = "午餐推荐".to_string();
    app.chat.cursor_pos = app.chat.input.len();
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();
    drain_until_idle(&mut app);

    let sessions_dir = temp.path().join(".zcode").join("sessions");
    let jsonl_files: Vec<_> = std::fs::read_dir(sessions_dir)
        .unwrap()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
        .collect();
    assert_eq!(jsonl_files.len(), 1);

    let content = std::fs::read_to_string(&jsonl_files[0]).unwrap();
    assert!(content.contains("介绍 zcode 工程结构"));
    assert!(content.contains("午餐推荐"));
    assert!(content
        .lines()
        .all(|line| serde_json::from_str::<serde_json::Value>(line).is_ok()));
}

#[test]
fn test_tui_app_uses_session_vector_context_for_related_prompt() {
    let temp = tempfile::TempDir::new().unwrap();
    let (captured_tx, captured_rx) = mpsc::channel();
    let executor: TaskExecutor = Arc::new(move |request, _cancel, tx| {
        captured_tx.send(request).unwrap();
        let _ = tx.send(TaskUiEvent::Done("answer".to_string()));
    });
    let mut app = TuiApp::with_task_executor(executor);
    app.project_root = temp.path().to_path_buf();

    app.chat.input = "介绍 zcode 工程结构".to_string();
    app.chat.cursor_pos = app.chat.input.len();
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();
    let first = captured_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .unwrap();
    assert!(first.history.is_empty());
    app.drain_task_events();

    app.chat.input = "午餐推荐".to_string();
    app.chat.cursor_pos = app.chat.input.len();
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();
    let second = captured_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .unwrap();
    assert!(second.history.is_empty());
    app.drain_task_events();

    app.chat.input = "继续讲 zcode 工程".to_string();
    app.chat.cursor_pos = app.chat.input.len();
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();
    let third = captured_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .unwrap();

    assert_eq!(third.history.len(), 2);
    assert_eq!(
        third.history[0].content.as_deref(),
        Some("介绍 zcode 工程结构")
    );
    assert!(!third
        .history
        .iter()
        .any(|message| message.content.as_deref() == Some("午餐推荐")));
}

#[test]
fn test_tui_app_slash_undo() {
    let mut app = TuiApp::new();
    app.chat.add_message(chat::ChatMessage::user("question"));
    app.chat.add_message(chat::ChatMessage::assistant("answer"));
    app.chat.input = "/undo".to_string();
    app.chat.cursor_pos = app.chat.input.len();

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    assert!(!app
        .chat
        .messages
        .iter()
        .any(|msg| msg.content == "question"));
    assert!(app
        .chat
        .messages
        .iter()
        .any(|msg| msg.content.contains("Undid")));
}

#[test]
fn test_tui_app_slash_compact() {
    let mut app = TuiApp::new();
    for index in 0..24 {
        app.chat
            .add_message(chat::ChatMessage::user(format!("message {}", index)));
    }
    app.chat.input = "/compact".to_string();
    app.chat.cursor_pos = app.chat.input.len();

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    assert!(app.chat.messages[0]
        .content
        .contains("Compacted conversation summary"));
}

#[test]
fn test_tui_app_slash_clear_resets_conversation() {
    let mut app = TuiApp::new();
    app.chat.add_message(chat::ChatMessage::user("question"));
    app.chat.add_message(chat::ChatMessage::assistant("answer"));
    app.chat.thinking_log.push("thinking".to_string());
    app.chat.latest_thinking = "thinking".to_string();
    app.chat.input = "/clear".to_string();
    app.chat.cursor_pos = app.chat.input.len();

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    assert_eq!(app.chat.messages.len(), 1);
    assert_eq!(app.chat.messages[0].role, "system");
    assert!(app.chat.messages[0].content.contains("fresh context"));
    assert!(app.chat.thinking_log.is_empty());
    assert!(app.chat.latest_thinking.is_empty());
}

#[test]
fn test_tui_app_slash_resume_missing_session_reports_error() {
    let mut app = TuiApp::new();
    app.project_root =
        std::env::temp_dir().join(format!("zcode-ui-missing-session-{}", std::process::id()));
    app.chat.input = "/resume missing".to_string();
    app.chat.cursor_pos = app.chat.input.len();

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_event(event).unwrap();

    assert!(app
        .chat
        .messages
        .iter()
        .any(|msg| msg.content.contains("Resume failed")));
}

// ============================================================
// TuiApp multi-line input tests
// ============================================================

#[test]
fn test_tui_app_shift_enter_newline() {
    let mut app = TuiApp::new();
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
    app.handle_event(event).unwrap();
    assert_eq!(app.chat.input, "\n");
}

#[test]
fn test_tui_app_ctrl_j_newline() {
    let mut app = TuiApp::new();
    let event = Event::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL));
    app.handle_event(event).unwrap();
    assert_eq!(app.chat.input, "\n");
}

#[test]
fn test_tui_app_ctrl_enter_newline() {
    let mut app = TuiApp::new();
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL));
    app.handle_event(event).unwrap();
    assert_eq!(app.chat.input, "\n");
}

#[test]
fn test_tui_app_alt_enter_newline() {
    let mut app = TuiApp::new();
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT));
    app.handle_event(event).unwrap();
    assert_eq!(app.chat.input, "\n");
}

#[test]
fn test_tui_app_cursor_left_right() {
    let mut app = TuiApp::new();
    app.chat.input = "Hello".to_string();
    app.chat.cursor_pos = 5; // end of "Hello"

    // Press left
    let event = Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    app.handle_event(event).unwrap();
    assert_eq!(app.chat.cursor_pos, 4);

    // Press right
    let event = Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    app.handle_event(event).unwrap();
    assert_eq!(app.chat.cursor_pos, 5);
}
