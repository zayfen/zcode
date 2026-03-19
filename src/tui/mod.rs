//! TUI module for zcode
//!
//! This module provides a terminal user interface with chat capabilities using ratatui.

pub mod chat;

pub use chat::ChatInterface;
use chat::ChatMessage;

use crate::agent::Agent;
use crate::error::ZcodeError;
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

    let backend = CrosstermBackend::new(io::stdout());
    Terminal::new(backend)
        .map_err(|e| ZcodeError::InternalError(format!("Failed to create terminal: {}", e)))
}

/// Restore the terminal to normal mode
pub fn restore_terminal(terminal: &mut TuiTerminal) -> crate::error::Result<()> {
    disable_raw_mode()
        .map_err(|e| ZcodeError::InternalError(format!("Failed to disable raw mode: {}", e)))?;

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
    /// Agent for processing messages
    agent: Option<Agent>,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new() -> Self {
        Self {
            should_quit: false,
            chat: ChatInterface::new(),
            agent: None,
        }
    }

    /// Set the agent for the TUI
    pub fn set_agent(&mut self, agent: Agent) {
        self.agent = Some(agent);
    }

    /// Handle a terminal event
    pub fn handle_event(&mut self, event: Event) -> crate::error::Result<()> {
        if let Event::Key(key) = event {
            match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                    self.should_quit = true;
                }
                (KeyModifiers::NONE, KeyCode::Esc) => {
                    self.should_quit = true;
                }
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    self.chat.send_current_input();
                }
                (KeyModifiers::NONE, KeyCode::Char(c)) => {
                    self.chat.input_char(c);
                }
                (KeyModifiers::NONE, KeyCode::Backspace) => {
                    self.chat.backspace();
                }
                (KeyModifiers::NONE, KeyCode::PageUp) => {
                    self.chat.scroll_up(10);
                }
                (KeyModifiers::NONE, KeyCode::PageDown) => {
                    self.chat.scroll_down(10);
                }
                (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                    self.chat.scroll_up(5);
                }
                (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
                    self.chat.scroll_down(5);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Run the main event loop (synchronous, without agent support)
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

    /// Run the main event loop with async agent support
    pub async fn run_async(&mut self, terminal: &mut TuiTerminal) -> crate::error::Result<()> {
        while !self.should_quit {
            terminal
                .draw(|f| self.chat.render(f))
                .map_err(|e| ZcodeError::InternalError(format!("Failed to draw: {}", e)))?;

            // Process pending agent message
            if self.chat.send_to_agent {
                self.chat.send_to_agent = false;
                let user_input = self.chat.pending_input.take().unwrap_or_default();
                self.chat.add_message(ChatMessage::user(&user_input));

                if let Some(ref mut agent) = self.agent {
                    // Use streaming
                    self.chat.is_streaming = true;
                    self.chat.streaming_text.clear();
                    self.chat.set_status("Thinking...");

                    match agent.run_streaming(&user_input).await {
                        Ok(stream) => {
                            use futures::StreamExt;
                            let mut stream = stream;
                            while let Some(chunk_result) = stream.next().await {
                                match chunk_result {
                                    Ok(chunk) => {
                                        self.chat.streaming_text.push_str(&chunk);
                                        self.chat.set_status("Streaming...");
                                        // Re-render after each chunk
                                        terminal
                                            .draw(|f| self.chat.render(f))
                                            .map_err(|e| ZcodeError::InternalError(format!("Failed to draw: {}", e)))?;
                                    }
                                    Err(e) => {
                                        self.chat
                                            .add_message(ChatMessage::system(format!("Stream error: {}", e)));
                                        break;
                                    }
                                }
                            }
                            // Finalize streaming
                            let final_text = self.chat.streaming_text.clone();
                            self.chat.is_streaming = false;
                            self.chat.streaming_text.clear();
                            self.chat.add_assistant_response(&final_text);
                            self.chat.set_status("Ready");
                        }
                        Err(e) => {
                            self.chat.is_streaming = false;
                            self.chat.streaming_text.clear();
                            self.chat
                                .add_message(ChatMessage::system(format!("Error: {}", e)));
                            self.chat.set_status("Ready");
                        }
                    }
                } else {
                    self.chat.add_assistant_response(
                        "Agent not configured. Please set ANTHROPIC_API_KEY.",
                    );
                    self.chat.set_status("Ready");
                }
            }

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

    #[test]
    fn test_tui_app_creation() {
        let app = TuiApp::new();
        assert!(!app.should_quit);
    }

    #[test]
    fn test_chat_interface_creation() {
        let chat = ChatInterface::new();
        assert!(chat.input.is_empty());
        assert!(chat.messages.is_empty());
    }
}
