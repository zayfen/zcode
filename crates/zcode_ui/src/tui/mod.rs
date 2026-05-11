//! TUI module for zcode
//!
//! This module provides a terminal user interface with chat capabilities using ratatui.

pub mod chat;

pub use chat::ChatInterface;

use zcode_core::ZcodeError;
use zcode_llm_provider::{LlmProvider, Message, MessageRole};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io::{self, Stdout};
use std::sync::Arc;

/// Type alias for the terminal backend
pub type TuiBackend = CrosstermBackend<Stdout>;

/// Type alias for the terminal
pub type TuiTerminal = Terminal<TuiBackend>;

/// Initialize the terminal for TUI mode
pub fn init_terminal() -> zcode_core::Result<TuiTerminal> {
    enable_raw_mode().map_err(|e| ZcodeError::InternalError(format!("Failed to enable raw mode: {}", e)))?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| ZcodeError::InternalError(format!("Failed to enter alternate screen: {}", e)))?;

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
    /// Set when Enter is pressed to trigger a deferred LLM call
    llm_pending: bool,
    /// Text from the pending Enter press
    pending_text: Option<String>,
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
            llm_pending: false,
            pending_text: None,
        }
    }

    /// Create a TUI application with a real LLM provider
    pub fn with_provider(provider: Arc<dyn LlmProvider>) -> Self {
        let mut app = Self::new();
        app.provider = Some(provider);
        app.chat.add_message(chat::ChatMessage::system(
            "zcode agent ready. Connected to LLM provider."
        ));
        app
    }

    /// Handle a terminal event
    pub fn handle_event(&mut self, event: Event) -> zcode_core::Result<()> {
        if let Event::Key(key) = event {
            match (key.modifiers, key.code) {
                // Quit
                (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                    self.should_quit = true;
                }
                (KeyModifiers::NONE, KeyCode::Esc) => {
                    self.should_quit = true;
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
                // Ctrl+Enter is sometimes sent as Ctrl+M
                (KeyModifiers::CONTROL, KeyCode::Enter) => {
                    self.chat.input_newline();
                }
                // Plain Enter: send message
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    if let Some(user_text) = self.chat.send_current_input() {
                        if self.provider.is_some() {
                            self.llm_pending = true;
                            self.pending_text = Some(user_text);
                        } else {
                            self.chat.add_message(chat::ChatMessage::assistant(
                                "⚠ No LLM provider configured. \
                                Set ZCODE_API_KEY environment variable and restart."
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
            }
        }
        Ok(())
    }

    /// Run the main event loop.
    ///
    /// LLM calls are deferred to the end of each iteration so the "Thinking..."
    /// status renders before the blocking provider.chat() call.
    pub fn run(&mut self, terminal: &mut TuiTerminal) -> zcode_core::Result<()> {
        while !self.should_quit {
            terminal
                .draw(|f| self.chat.render(f, &self.agent_statuses, &self.active_skills, &self.active_mcps))
                .map_err(|e| ZcodeError::InternalError(format!("Failed to draw: {}", e)))?;

            if event::poll(std::time::Duration::from_millis(100))
                .map_err(|e| ZcodeError::InternalError(format!("Poll error: {}", e)))?
            {
                let event = event::read()
                    .map_err(|e| ZcodeError::InternalError(format!("Read error: {}", e)))?;
                self.handle_event(event)?;
            }

            // Deferred LLM call — draw first so "Thinking..." is visible
            if self.llm_pending {
                self.llm_pending = false;
                let user_text = self.pending_text.take().unwrap_or_default();

                let provider = self.provider.as_ref().expect("provider checked before setting llm_pending");

                // Update Supervisor status to Thinking
                if let Some(agent) = self.agent_statuses.iter_mut().find(|a| a.0 == "Supervisor") {
                    agent.1 = "Thinking...".to_string();
                }

                // Draw "Thinking..." before blocking LLM call
                terminal
                    .draw(|f| self.chat.render(f, &self.agent_statuses, &self.active_skills, &self.active_mcps))
                    .map_err(|e| ZcodeError::InternalError(format!("Failed to draw: {}", e)))?;

                // Build conversation history
                let mut messages: Vec<Message> = vec![
                    Message::system(&self.system_prompt),
                ];
                for msg in &self.chat.messages {
                    if msg.role == "system" { continue; }
                    if msg.role == "user" {
                        messages.push(Message::user(&msg.content));
                    } else if msg.role == "assistant" {
                        messages.push(Message::assistant(&msg.content));
                    }
                }
                if messages.last().map(|m| &m.role) != Some(&MessageRole::User) {
                    messages.push(Message::user(&user_text));
                }

                match provider.chat(&messages, &[]) {
                    Ok(response) => {
                        self.chat.add_message(chat::ChatMessage::assistant(response.content));
                        if let Some(agent) = self.agent_statuses.iter_mut().find(|a| a.0 == "Supervisor") {
                            agent.1 = "Idle".to_string();
                        }
                    }
                    Err(e) => {
                        self.chat.add_message(chat::ChatMessage::assistant(
                            format!("⚠ LLM error: {}", e)
                        ));
                        if let Some(agent) = self.agent_statuses.iter_mut().find(|a| a.0 == "Supervisor") {
                            agent.1 = "Idle (Error)".to_string();
                        }
                    }
                }
            }
        }
        Ok(())
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
    use zcode_llm_provider::LlmResponse;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

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
        
        // Scroll down
        let event = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        app.handle_event(event).unwrap();
        assert_eq!(app.chat.scroll, 3);
        
        // Scroll up
        let event = Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        app.handle_event(event).unwrap();
        assert_eq!(app.chat.scroll, 0);
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

        // Should have user message and assistant response
        assert_eq!(app.chat.messages.len(), 2);
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

        fn chat(&self, _messages: &[Message], _tools: &[serde_json::Value]) -> std::result::Result<LlmResponse, ZcodeError> {
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

        fn stream_complete(&self, _prompt: &str) -> std::result::Result<zcode_llm_provider::provider::StreamingResponse, ZcodeError> {
            unimplemented!()
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
