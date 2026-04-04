//! TUI module for zcode
//!
//! This module provides a terminal user interface with chat capabilities using ratatui.

pub mod chat;

pub use chat::ChatInterface;

use crate::error::ZcodeError;
use crate::llm::{LlmProvider, Message, MessageRole};
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
pub fn init_terminal() -> crate::error::Result<TuiTerminal> {
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
pub fn restore_terminal(terminal: &mut TuiTerminal) -> crate::error::Result<()> {
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
    pub fn handle_event(&mut self, event: Event) -> crate::error::Result<()> {
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
                        self.call_llm(user_text);
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
                _ => {}
            }
        }
        Ok(())
    }

    /// Call the LLM provider and add the response to the chat.
    /// Shows a "Thinking..." indicator while waiting, falls back to an
    /// error message if no provider is configured or the call fails.
    fn call_llm(&mut self, user_text: String) {
        let Some(provider) = &self.provider else {
            self.chat.add_message(chat::ChatMessage::assistant(
                "⚠ No LLM provider configured. \
                Set ANTHROPIC_API_KEY or OPENAI_API_KEY environment variable and restart."
            ));
            return;
        };

        // Build conversation history as Message slice
        let mut messages: Vec<Message> = vec![
            Message::system(&self.system_prompt),
        ];
        for msg in &self.chat.messages {
            // Skip system messages already prepended
            if msg.role == "system" { continue; }
            if msg.role == "user" {
                messages.push(Message::user(&msg.content));
            } else if msg.role == "assistant" {
                messages.push(Message {
                    role: MessageRole::Assistant,
                    content: msg.content.clone(),
                });
            }
        }
        // Include the current user message (it was already pushed to chat)
        if messages.last().map(|m| &m.role) != Some(&MessageRole::User) {
            messages.push(Message::user(&user_text));
        }

        match provider.chat(&messages, &[]) {
            Ok(response) => {
                self.chat.add_message(chat::ChatMessage::assistant(response.content));
            }
            Err(e) => {
                self.chat.add_message(chat::ChatMessage::assistant(
                    format!("⚠ LLM error: {}", e)
                ));
            }
        }
    }

    /// Run the main event loop
    pub fn run(&mut self, terminal: &mut TuiTerminal) -> crate::error::Result<()> {
        while !self.should_quit {
            terminal
                .draw(|f| self.chat.render(f))
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

    // ============================================================
    // TuiApp creation tests
    // ============================================================

    #[test]
    fn test_tui_app_new() {
        let app = TuiApp::new();
        assert!(!app.should_quit);
    }

    #[test]
    fn test_tui_app_default() {
        let app = TuiApp::default();
        assert!(!app.should_quit);
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
}
