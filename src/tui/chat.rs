//! Chat interface component for the TUI
//!
//! This module provides a chat interface with input and message display.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// A chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Role (user, assistant, system)
    pub role: String,
    /// Message content
    pub content: String,
}

impl ChatMessage {
    /// Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }

    /// Create a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }
}

/// Chat interface state
#[derive(Debug, Default)]
pub struct ChatInterface {
    /// Current input text
    pub input: String,
    /// Chat messages
    pub messages: Vec<ChatMessage>,
    /// Scroll position
    pub scroll: u16,
    /// Whether input should be sent to the agent
    pub send_to_agent: bool,
    /// Pending input text for the agent
    pub pending_input: Option<String>,
    /// Whether a streaming response is active
    pub is_streaming: bool,
    /// Accumulated streaming text
    pub streaming_text: String,
    /// Status bar text
    pub status: String,
}

impl ChatInterface {
    /// Create a new chat interface
    pub fn new() -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
            scroll: 0,
            send_to_agent: false,
            pending_input: None,
            is_streaming: false,
            streaming_text: String::new(),
            status: "Ready".to_string(),
        }
    }

    /// Add a character to the input
    pub fn input_char(&mut self, c: char) {
        self.input.push(c);
    }

    /// Remove the last character from input
    pub fn backspace(&mut self) {
        self.input.pop();
    }

    /// Send the current input as a message
    pub fn send_current_input(&mut self) {
        if !self.input.is_empty() {
            self.pending_input = Some(self.input.clone());
            self.send_to_agent = true;
            self.input.clear();
        }
    }

    /// Add an assistant response to the chat
    pub fn add_assistant_response(&mut self, content: &str) {
        self.messages.push(ChatMessage::assistant(content));
        self.scroll_to_bottom();
    }

    /// Add a message to the chat
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        self.scroll_to_bottom();
    }

    /// Set the status bar text
    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    /// Scroll up by N lines
    pub fn scroll_up(&mut self, lines: u16) {
        self.scroll = self.scroll.saturating_sub(lines);
    }

    /// Scroll down by N lines
    pub fn scroll_down(&mut self, lines: u16) {
        self.scroll = self.scroll.saturating_add(lines);
    }

    /// Auto-scroll to bottom (called after new messages)
    pub fn scroll_to_bottom(&mut self) {
        self.scroll = u16::MAX;
    }

    /// Render the chat interface
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.size();

        // Create layout: messages on top, input at bottom
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3), Constraint::Length(1)])
            .split(area);

        // Render messages area
        let messages_widget = self.render_messages(chunks[0]);
        frame.render_widget(messages_widget, chunks[0]);

        // Render input area
        let input_widget = self.render_input(chunks[1]);
        frame.render_widget(input_widget, chunks[1]);

        // Render status bar
        let status_widget = self.render_status(chunks[2]);
        frame.render_widget(status_widget, chunks[2]);
    }

    /// Render the messages area
    fn render_messages(&self, area: Rect) -> Paragraph<'_> {
        let mut lines = Vec::new();

        for message in &self.messages {
            let (style, prefix) = match message.role.as_str() {
                "user" => (Style::default().fg(Color::Cyan), "You: "),
                "assistant" => (Style::default().fg(Color::Green), "Assistant: "),
                "system" => (Style::default().fg(Color::Yellow), "System: "),
                _ => (Style::default(), ""),
            };

            // Word wrap the content
            let max_width = area.width.saturating_sub(2) as usize;
            let wrapped = textwrap::wrap(&message.content, max_width);

            for (i, line) in wrapped.iter().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled(prefix, style.add_modifier(Modifier::BOLD)),
                        Span::styled(line.to_string(), style),
                    ]));
                } else {
                    lines.push(Line::from(Span::styled(line.to_string(), style)));
                }
            }
        }

        // Show streaming text if active
        if self.is_streaming && !self.streaming_text.is_empty() {
            let style = Style::default().fg(Color::Green);
            let prefix = "Assistant: ";
            let max_width = area.width.saturating_sub(2) as usize;
            let wrapped = textwrap::wrap(&self.streaming_text, max_width);

            for (i, line) in wrapped.iter().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled(prefix, style.add_modifier(Modifier::BOLD)),
                        Span::styled(line.to_string(), style),
                    ]));
                } else {
                    lines.push(Line::from(Span::styled(line.to_string(), style)));
                }
            }
            // Blinking cursor indicator
            lines.push(Line::from(Span::styled(
                "\u{258A}",
                Style::default().fg(Color::Green).add_modifier(Modifier::SLOW_BLINK),
            )));
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "No messages yet. Start typing to chat!",
                Style::default().fg(Color::DarkGray),
            )));
        }

        let total_lines = lines.len() as u16;
        let view_height = area.height.saturating_sub(2);
        let max_scroll = total_lines.saturating_sub(view_height);
        let scroll = if self.scroll == u16::MAX {
            max_scroll
        } else {
            self.scroll.min(max_scroll)
        };

        Paragraph::new(Text::from(lines))
            .scroll((scroll, 0))
            .block(Block::default().borders(Borders::ALL).title("Chat"))
    }

    /// Render the input area
    fn render_input(&self, _area: Rect) -> Paragraph<'_> {
        let input_text = if self.input.is_empty() {
            Text::from(Span::styled(
                "Type a message... (Enter to send, Esc to quit)",
                Style::default().fg(Color::DarkGray),
            ))
        } else {
            Text::from(self.input.as_str())
        };

        Paragraph::new(input_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input")
                .style(Style::default()),
        )
    }

    /// Render the status bar
    fn render_status(&self, _area: Rect) -> Paragraph<'_> {
        let scroll_info = if self.scroll > 0 {
            format!(" | Scroll: {}", self.scroll)
        } else {
            String::new()
        };
        let msg_count = format!("Messages: {}", self.messages.len());
        let status_text = format!("{}{} | {}", self.status, scroll_info, msg_count);

        Paragraph::new(status_text).style(
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_creation() {
        let user_msg = ChatMessage::user("Hello");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = ChatMessage::assistant("Hi there!");
        assert_eq!(assistant_msg.role, "assistant");

        let system_msg = ChatMessage::system("Welcome");
        assert_eq!(system_msg.role, "system");
    }

    #[test]
    fn test_chat_interface_input() {
        let mut chat = ChatInterface::new();
        chat.input_char('H');
        chat.input_char('i');
        assert_eq!(chat.input, "Hi");

        chat.backspace();
        assert_eq!(chat.input, "H");
    }

    #[test]
    fn test_chat_interface_send() {
        let mut chat = ChatInterface::new();
        chat.input = "Hello".to_string();
        chat.send_current_input();

        assert!(chat.input.is_empty());
        assert!(chat.send_to_agent);
        assert_eq!(chat.pending_input, Some("Hello".to_string()));
    }

    #[test]
    fn test_chat_interface_add_message() {
        let mut chat = ChatInterface::new();
        chat.add_message(ChatMessage::system("Welcome to zcode!"));

        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].role, "system");
    }

    #[test]
    fn test_scroll_up_down() {
        let mut chat = ChatInterface::new();
        chat.scroll_down(10);
        assert_eq!(chat.scroll, 10);
        chat.scroll_up(5);
        assert_eq!(chat.scroll, 5);
        chat.scroll_up(10); // saturating
        assert_eq!(chat.scroll, 0);
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut chat = ChatInterface::new();
        chat.scroll_to_bottom();
        assert_eq!(chat.scroll, u16::MAX);
    }

    #[test]
    fn test_set_status() {
        let mut chat = ChatInterface::new();
        assert_eq!(chat.status, "Ready");
        chat.set_status("Thinking...");
        assert_eq!(chat.status, "Thinking...");
    }
}
