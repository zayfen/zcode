//! TUI module for zcode
//!
//! This module provides a terminal user interface with chat capabilities using ratatui.

pub mod chat;

pub use chat::ChatInterface;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::executor::block_on;
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::VecDeque;
use std::io::{self, Stdout};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use zcode_core::agent::ConversationMessage;
use zcode_core::ZcodeError;
use zcode_llm_provider::{LlmProvider, LlmStreamEvent, Message, MessageRole};
use zcode_session::{Session, SessionManager};

/// Type alias for the terminal backend
pub type TuiBackend = CrosstermBackend<Stdout>;

/// Type alias for the terminal
pub type TuiTerminal = Terminal<TuiBackend>;

/// Initialize the terminal for TUI mode
pub fn init_terminal() -> zcode_core::Result<TuiTerminal> {
    enable_raw_mode()
        .map_err(|e| ZcodeError::InternalError(format!("Failed to enable raw mode: {}", e)))?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(|e| {
        ZcodeError::InternalError(format!("Failed to enter alternate screen: {}", e))
    })?;

    // Try to enable keyboard enhancement protocol (kitty protocol).
    // This lets terminals like iTerm2/Ghostty send Shift+Enter as a distinct event.
    // If unsupported, silently skip — Alt+Enter / Ctrl+J are always available.
    let _ = execute!(
        stdout,
        crossterm::event::PushKeyboardEnhancementFlags(
            crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        )
    );

    let backend = CrosstermBackend::new(io::stdout());
    Terminal::new(backend)
        .map_err(|e| ZcodeError::InternalError(format!("Failed to create terminal: {}", e)))
}

/// Restore the terminal to normal mode
pub fn restore_terminal(terminal: &mut TuiTerminal) -> zcode_core::Result<()> {
    disable_raw_mode()
        .map_err(|e| ZcodeError::InternalError(format!("Failed to disable raw mode: {}", e)))?;

    // Pop keyboard enhancement flags if we pushed them
    let _ = execute!(
        terminal.backend_mut(),
        crossterm::event::PopKeyboardEnhancementFlags
    );

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|e| ZcodeError::InternalError(format!("Failed to leave alternate screen: {}", e)))?;

    terminal
        .show_cursor()
        .map_err(|e| ZcodeError::InternalError(format!("Failed to show cursor: {}", e)))
}

/// TUI application state
pub struct TuiApp {
    /// Whether the app should quit
    pub should_quit: bool,
    /// Chat interface
    pub chat: ChatInterface,
    /// LLM provider (None in test/no-api mode)
    provider: Option<Arc<dyn LlmProvider>>,
    /// System prompt
    pub system_prompt: String,
    /// Current agents statuses [(Name, Status)]
    pub agent_statuses: Vec<(String, String)>,
    /// Loaded skill names
    pub active_skills: Vec<String>,
    /// Loaded MCP server names
    pub active_mcps: Vec<String>,
    /// Prompts queued while an LLM response is in flight.
    pending_prompts: VecDeque<String>,
    /// Whether a streaming LLM response is in flight.
    llm_in_flight: bool,
    /// Stream event receiver from the worker thread.
    stream_rx: Option<Receiver<UiStreamEvent>>,
    /// Cancellation flag for the active stream worker.
    cancel_flag: Option<Arc<AtomicBool>>,
    /// Optional session manager for slash commands.
    session_manager: Option<SessionManager>,
    /// Project root used to lazily initialize session storage.
    project_root: PathBuf,
    /// Current session id, if the visible chat came from a saved session.
    current_session_id: Option<String>,
}

enum UiStreamEvent {
    Content(String),
    Thinking(String),
    Done,
    Cancelled,
    Error(String),
}

impl TuiApp {
    /// Create a new TUI application without an LLM provider
    pub fn new() -> Self {
        Self {
            should_quit: false,
            chat: ChatInterface::new(),
            provider: None,
            system_prompt: "You are zcode, a helpful AI coding agent. \
                Use your knowledge to assist with code, architecture, and development tasks."
                .to_string(),
            agent_statuses: vec![
                ("Supervisor".to_string(), "Idle".to_string()),
                ("Planner".to_string(), "Idle".to_string()),
                ("Coder".to_string(), "Idle".to_string()),
                ("Reviewer".to_string(), "Idle".to_string()),
                ("Investigator".to_string(), "Idle".to_string()),
            ],
            active_skills: Vec::new(),
            active_mcps: Vec::new(),
            pending_prompts: VecDeque::new(),
            llm_in_flight: false,
            stream_rx: None,
            cancel_flag: None,
            session_manager: None,
            project_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            current_session_id: None,
        }
    }

    /// Create a TUI application with a real LLM provider
    pub fn with_provider(provider: Arc<dyn LlmProvider>) -> Self {
        let mut app = Self::new();
        app.provider = Some(provider);
        app.chat.add_message(chat::ChatMessage::system(
            "zcode agent ready. Connected to LLM provider.",
        ));
        app
    }

    /// Handle a terminal event
    pub fn handle_event(&mut self, event: Event) -> zcode_core::Result<()> {
        match event {
            Event::Key(key) => match (key.modifiers, key.code) {
                // Quit
                (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                    self.should_quit = true;
                }
                (KeyModifiers::NONE, KeyCode::Esc) => {
                    if self.llm_in_flight {
                        self.cancel_active_stream();
                    } else {
                        self.should_quit = true;
                    }
                }
                // --- Newline insertion ---
                // Shift+Enter (requires keyboard enhancement / kitty protocol)
                (KeyModifiers::SHIFT, KeyCode::Enter) => {
                    self.chat.input_newline();
                }
                // Alt+Enter — works in most terminals without keyboard enhancement
                (KeyModifiers::ALT, KeyCode::Enter) => {
                    self.chat.input_newline();
                }
                // Ctrl+J — fallback for terminals that map Ctrl+J to \n
                (KeyModifiers::CONTROL, KeyCode::Char('j')) => {
                    self.chat.input_newline();
                }
                (KeyModifiers::CONTROL, KeyCode::Char('o')) => {
                    self.chat.toggle_thinking();
                }
                // Ctrl+Enter is sometimes sent as Ctrl+M
                (KeyModifiers::CONTROL, KeyCode::Enter) => {
                    self.chat.input_newline();
                }
                // Plain Enter: send message
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    if let Some(user_text) = self.chat.take_current_input() {
                        if user_text.trim_start().starts_with('/') {
                            self.handle_slash_command(&user_text);
                        } else if self.provider.is_some() {
                            self.pending_prompts.push_back(user_text);
                            self.chat.pending_count = self.pending_prompts.len();
                            self.start_next_prompt_if_idle();
                        } else {
                            self.chat.add_message(chat::ChatMessage::user(user_text));
                            self.chat.add_message(chat::ChatMessage::assistant(
                                "⚠ No LLM provider configured. \
                                Set ZCODE_API_KEY environment variable and restart.",
                            ));
                        }
                    }
                }
                // --- Cursor movement ---
                (KeyModifiers::NONE, KeyCode::Left) => {
                    self.chat.cursor_left();
                }
                (KeyModifiers::NONE, KeyCode::Right) => {
                    self.chat.cursor_right();
                }
                // --- Typing ---
                (KeyModifiers::NONE, KeyCode::Char(c))
                | (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                    self.chat.input_char(c);
                }
                (KeyModifiers::NONE, KeyCode::Backspace)
                | (KeyModifiers::SHIFT, KeyCode::Backspace) => {
                    self.chat.backspace();
                }
                (KeyModifiers::NONE, KeyCode::Up) => {
                    self.chat.scroll_up();
                }
                (KeyModifiers::NONE, KeyCode::Down) => {
                    self.chat.scroll_down();
                }
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => self.chat.scroll_up(),
                MouseEventKind::ScrollDown => self.chat.scroll_down(),
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    /// Run the main event loop.
    pub fn run(&mut self, terminal: &mut TuiTerminal) -> zcode_core::Result<()> {
        while !self.should_quit {
            self.drain_stream_events();
            self.start_next_prompt_if_idle();
            self.chat.tick_loading();

            terminal
                .draw(|f| {
                    self.chat.render(
                        f,
                        &self.agent_statuses,
                        &self.active_skills,
                        &self.active_mcps,
                    )
                })
                .map_err(|e| ZcodeError::InternalError(format!("Failed to draw: {}", e)))?;

            if event::poll(std::time::Duration::from_millis(100))
                .map_err(|e| ZcodeError::InternalError(format!("Poll error: {}", e)))?
            {
                let event = event::read()
                    .map_err(|e| ZcodeError::InternalError(format!("Read error: {}", e)))?;
                self.handle_event(event)?;
            }
        }
        Ok(())
    }

    fn start_next_prompt_if_idle(&mut self) {
        if self.llm_in_flight {
            return;
        }
        let Some(user_text) = self.pending_prompts.pop_front() else {
            self.chat.pending_count = 0;
            return;
        };
        let Some(provider) = self.provider.as_ref().cloned() else {
            return;
        };

        self.chat
            .add_message(chat::ChatMessage::user(user_text.clone()));
        self.chat.pending_count = self.pending_prompts.len();
        self.chat.start_loading();
        if let Some(agent) = self.agent_statuses.iter_mut().find(|a| a.0 == "Supervisor") {
            agent.1 = "Thinking...".to_string();
        }
        self.llm_in_flight = true;

        let messages = self.build_messages_for_prompt(&user_text);
        let (tx, rx) = mpsc::channel();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let worker_cancel_flag = Arc::clone(&cancel_flag);
        self.cancel_flag = Some(cancel_flag);
        self.stream_rx = Some(rx);

        std::thread::spawn(move || {
            let result = (|| {
                let mut stream = provider.stream_chat(&messages, &[])?;
                while let Some(event) = block_on(stream.next()) {
                    if worker_cancel_flag.load(Ordering::SeqCst) {
                        let _ = tx.send(UiStreamEvent::Cancelled);
                        return Ok(());
                    }
                    match event {
                        Ok(LlmStreamEvent::Content(text)) => {
                            let _ = tx.send(UiStreamEvent::Content(text));
                        }
                        Ok(LlmStreamEvent::Thinking(text)) => {
                            let _ = tx.send(UiStreamEvent::Thinking(text));
                        }
                        Err(e) => return Err(e),
                    }
                }
                Ok(())
            })();

            match result {
                Ok(()) => {
                    let _ = tx.send(UiStreamEvent::Done);
                }
                Err(e) => {
                    let _ = tx.send(UiStreamEvent::Error(e.to_string()));
                }
            }
        });
    }

    fn drain_stream_events(&mut self) {
        let Some(rx) = self.stream_rx.take() else {
            return;
        };
        let mut keep_rx = true;
        loop {
            match rx.try_recv() {
                Ok(UiStreamEvent::Content(text)) => {
                    self.chat.append_assistant_delta(&text);
                }
                Ok(UiStreamEvent::Thinking(text)) => {
                    self.chat.append_thinking(&text);
                }
                Ok(UiStreamEvent::Done) => {
                    self.finish_stream(false);
                    keep_rx = false;
                    break;
                }
                Ok(UiStreamEvent::Cancelled) => {
                    self.chat
                        .add_message(chat::ChatMessage::system("Request interrupted."));
                    self.finish_stream(true);
                    keep_rx = false;
                    break;
                }
                Ok(UiStreamEvent::Error(error)) => {
                    self.chat.add_message(chat::ChatMessage::assistant(format!(
                        "⚠ LLM error: {}",
                        error
                    )));
                    self.finish_stream(true);
                    keep_rx = false;
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.finish_stream(true);
                    keep_rx = false;
                    break;
                }
            }
        }
        if keep_rx {
            self.stream_rx = Some(rx);
        }
    }

    fn finish_stream(&mut self, had_error: bool) {
        self.llm_in_flight = false;
        self.cancel_flag = None;
        self.chat.stop_loading();
        self.chat.pending_count = self.pending_prompts.len();
        if let Some(agent) = self.agent_statuses.iter_mut().find(|a| a.0 == "Supervisor") {
            agent.1 = if had_error {
                "Idle (Error)".to_string()
            } else {
                "Idle".to_string()
            };
        }
    }

    fn build_messages_for_prompt(&self, user_text: &str) -> Vec<Message> {
        let mut messages = vec![Message::system(&self.system_prompt)];
        for msg in &self.chat.messages {
            if msg.role == "system" {
                continue;
            }
            if msg.role == "user" {
                messages.push(Message::user(&msg.content));
            } else if msg.role == "assistant" && !msg.content.is_empty() {
                messages.push(Message::assistant(&msg.content));
            }
        }
        if messages.last().map(|m| &m.role) != Some(&MessageRole::User) {
            messages.push(Message::user(user_text));
        }
        messages
    }

    fn cancel_active_stream(&mut self) {
        if let Some(flag) = &self.cancel_flag {
            flag.store(true, Ordering::SeqCst);
        }
        self.stream_rx = None;
        self.pending_prompts.clear();
        self.finish_stream(true);
        self.chat
            .add_message(chat::ChatMessage::system("Request interrupted."));
        if let Some(agent) = self.agent_statuses.iter_mut().find(|a| a.0 == "Supervisor") {
            agent.1 = "Idle (Cancelled)".to_string();
        }
    }

    fn handle_slash_command(&mut self, input: &str) {
        let trimmed = input.trim();
        let mut parts = trimmed.split_whitespace();
        let command = parts.next().unwrap_or("");
        match command {
            "/undo" => {
                self.pending_prompts.pop_back();
                self.chat.pending_count = self.pending_prompts.len();
                if self.chat.undo_last_turn() {
                    self.chat
                        .add_message(chat::ChatMessage::system("Undid the last turn."));
                } else {
                    self.chat
                        .add_message(chat::ChatMessage::system("Nothing to undo."));
                }
            }
            "/compact" => {
                if self.chat.compact_messages(20, 8_000) {
                    self.chat
                        .add_message(chat::ChatMessage::system("Conversation compacted."));
                } else {
                    self.chat.add_message(chat::ChatMessage::system(
                        "Conversation is already compact enough.",
                    ));
                }
                self.save_current_session();
            }
            "/resume" => {
                let requested = parts.next();
                match self.resume_session(requested) {
                    Ok(message) => self.chat.add_message(chat::ChatMessage::system(message)),
                    Err(error) => self.chat.add_message(chat::ChatMessage::system(format!(
                        "Resume failed: {}",
                        error
                    ))),
                }
            }
            "/help" => {
                self.chat.add_message(chat::ChatMessage::system(
                    "Commands: `/resume [id]`, `/undo`, `/compact`, `/help`.",
                ));
            }
            _ => {
                self.chat.add_message(chat::ChatMessage::system(
                    "Unknown command. Try `/help`, `/resume`, `/undo`, or `/compact`.",
                ));
            }
        }
    }

    fn resume_session(&mut self, requested_id: Option<&str>) -> zcode_core::Result<String> {
        let project_root = self.project_root.clone();
        let manager = self
            .session_manager
            .get_or_insert(SessionManager::new(project_root)?);
        let session = if let Some(id) = requested_id {
            manager.load(id)?
        } else {
            manager
                .list()?
                .into_iter()
                .next()
                .ok_or_else(|| ZcodeError::FileNotFound {
                    path: ".zcode/sessions/*.json".to_string(),
                })?
        };

        let id = session.id.clone();
        let message_count = session.messages.len();
        self.current_session_id = Some(id.clone());
        self.chat
            .replace_messages(chat_messages_from_session(session));
        Ok(format!(
            "Resumed session `{}` ({} messages).",
            id, message_count
        ))
    }

    fn save_current_session(&mut self) {
        let Some(id) = self.current_session_id.clone() else {
            return;
        };
        let project_root = self.project_root.clone();
        if self.session_manager.is_none() {
            let Ok(manager) = SessionManager::new(project_root) else {
                return;
            };
            self.session_manager = Some(manager);
        }

        let Some(manager) = self.session_manager.as_ref() else {
            return;
        };
        let mut session = Session::new(id);
        for message in &self.chat.messages {
            session.push(conversation_message_from_chat(message));
        }
        let _ = manager.save(&mut session);
    }
}

fn chat_messages_from_session(session: Session) -> Vec<chat::ChatMessage> {
    let mut messages = Vec::new();
    if let Some(summary) = session.summary {
        if !summary.trim().is_empty() {
            messages.push(chat::ChatMessage::system(summary));
        }
    }
    messages.extend(session.messages.into_iter().filter_map(|message| {
        let content = message.content.unwrap_or_default();
        match message.role.as_str() {
            "user" => Some(chat::ChatMessage::user(content)),
            "assistant" => Some(chat::ChatMessage::assistant(content)),
            "system" => Some(chat::ChatMessage::system(content)),
            _ => None,
        }
    }));
    messages
}

fn conversation_message_from_chat(message: &chat::ChatMessage) -> ConversationMessage {
    match message.role.as_str() {
        "user" => ConversationMessage::user(&message.content),
        "assistant" => ConversationMessage::assistant_text(&message.content),
        _ => ConversationMessage::system(&message.content),
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use zcode_llm_provider::LlmResponse;

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
        ) -> std::result::Result<zcode_llm_provider::provider::StreamingResponse, ZcodeError>
        {
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
}
