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
}

impl ChatInterface {
    /// Create a new chat interface
    pub fn new() -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
            scroll: 0,
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
            let message = ChatMessage::user(self.input.clone());
            self.messages.push(message);
            self.input.clear();

            // Add a placeholder assistant response
            let response = ChatMessage::assistant("Response received. (Full LLM integration coming in Task 3)");
            self.messages.push(response);
        }
    }

    /// Add a message to the chat
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    /// Render the chat interface
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.size();

        // Create layout: messages on top, input at bottom
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(area);

        // Render messages area
        let messages_widget = self.render_messages(chunks[0]);
        frame.render_widget(messages_widget, chunks[0]);

        // Render input area
        let input_widget = self.render_input(chunks[1]);
        frame.render_widget(input_widget, chunks[1]);
    }

    /// Render the messages area
    fn render_messages(&self, area: Rect) -> Paragraph {
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

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "No messages yet. Start typing to chat!",
                Style::default().fg(Color::DarkGray),
            )));
        }

        Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::ALL).title("Chat"))
    }

    /// Render the input area
    fn render_input(&self, _area: Rect) -> Paragraph {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // ChatMessage creation tests
    // ============================================================

    #[test]
    fn test_chat_message_user() {
        let msg = ChatMessage::user("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_chat_message_assistant() {
        let msg = ChatMessage::assistant("Hi there!");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "Hi there!");
    }

    #[test]
    fn test_chat_message_system() {
        let msg = ChatMessage::system("Welcome");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, "Welcome");
    }

    #[test]
    fn test_chat_message_empty_content() {
        let msg = ChatMessage::user("");
        assert_eq!(msg.content, "");
    }

    #[test]
    fn test_chat_message_long_content() {
        let long_content = "x".repeat(10000);
        let msg = ChatMessage::user(long_content.clone());
        assert_eq!(msg.content.len(), 10000);
    }

    #[test]
    fn test_chat_message_unicode() {
        let msg = ChatMessage::user("Hello 你好 🎉");
        assert_eq!(msg.content, "Hello 你好 🎉");
    }

    #[test]
    fn test_chat_message_multiline() {
        let multiline = "Line 1\nLine 2\nLine 3";
        let msg = ChatMessage::user(multiline);
        assert_eq!(msg.content, multiline);
    }

    #[test]
    fn test_chat_message_from_string() {
        let content = String::from("Test message");
        let msg = ChatMessage::user(content.clone());
        assert_eq!(msg.content, content);
    }

    #[test]
    fn test_chat_message_clone() {
        let msg = ChatMessage::user("Test");
        let cloned = msg.clone();
        assert_eq!(msg.role, cloned.role);
        assert_eq!(msg.content, cloned.content);
    }

    #[test]
    fn test_chat_message_debug() {
        let msg = ChatMessage::user("Test");
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("ChatMessage"));
        assert!(debug_str.contains("user"));
        assert!(debug_str.contains("Test"));
    }

    // ============================================================
    // ChatInterface creation tests
    // ============================================================

    #[test]
    fn test_chat_interface_new() {
        let chat = ChatInterface::new();
        assert!(chat.input.is_empty());
        assert!(chat.messages.is_empty());
        assert_eq!(chat.scroll, 0);
    }

    #[test]
    fn test_chat_interface_default() {
        let chat = ChatInterface::default();
        assert!(chat.input.is_empty());
        assert!(chat.messages.is_empty());
    }

    // ============================================================
    // ChatInterface input tests
    // ============================================================

    #[test]
    fn test_chat_interface_input_char_single() {
        let mut chat = ChatInterface::new();
        chat.input_char('H');
        assert_eq!(chat.input, "H");
    }

    #[test]
    fn test_chat_interface_input_char_multiple() {
        let mut chat = ChatInterface::new();
        chat.input_char('H');
        chat.input_char('e');
        chat.input_char('l');
        chat.input_char('l');
        chat.input_char('o');
        assert_eq!(chat.input, "Hello");
    }

    #[test]
    fn test_chat_interface_input_char_unicode() {
        let mut chat = ChatInterface::new();
        chat.input_char('你');
        chat.input_char('好');
        assert_eq!(chat.input, "你好");
    }

    #[test]
    fn test_chat_interface_input_char_emoji() {
        let mut chat = ChatInterface::new();
        chat.input_char('🎉');
        assert_eq!(chat.input, "🎉");
    }

    #[test]
    fn test_chat_interface_backspace_empty() {
        let mut chat = ChatInterface::new();
        chat.backspace();
        assert!(chat.input.is_empty());
    }

    #[test]
    fn test_chat_interface_backspace_single() {
        let mut chat = ChatInterface::new();
        chat.input_char('H');
        chat.backspace();
        assert!(chat.input.is_empty());
    }

    #[test]
    fn test_chat_interface_backspace_multiple() {
        let mut chat = ChatInterface::new();
        chat.input_char('H');
        chat.input_char('i');
        chat.backspace();
        assert_eq!(chat.input, "H");
    }

    #[test]
    fn test_chat_interface_backspace_all() {
        let mut chat = ChatInterface::new();
        chat.input_char('H');
        chat.input_char('i');
        chat.backspace();
        chat.backspace();
        assert!(chat.input.is_empty());
    }

    #[test]
    fn test_chat_interface_backspace_unicode() {
        let mut chat = ChatInterface::new();
        chat.input_char('你');
        chat.input_char('好');
        chat.backspace();
        assert_eq!(chat.input, "你");
    }

    // ============================================================
    // ChatInterface send tests
    // ============================================================

    #[test]
    fn test_chat_interface_send_empty() {
        let mut chat = ChatInterface::new();
        chat.send_current_input();
        assert!(chat.messages.is_empty());
    }

    #[test]
    fn test_chat_interface_send_single_message() {
        let mut chat = ChatInterface::new();
        chat.input = "Hello".to_string();
        chat.send_current_input();

        assert!(chat.input.is_empty());
        assert_eq!(chat.messages.len(), 2); // user + assistant response
        assert_eq!(chat.messages[0].role, "user");
        assert_eq!(chat.messages[0].content, "Hello");
    }

    #[test]
    fn test_chat_interface_send_adds_assistant_response() {
        let mut chat = ChatInterface::new();
        chat.input = "Hello".to_string();
        chat.send_current_input();

        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.messages[1].role, "assistant");
        assert!(chat.messages[1].content.contains("Response received"));
    }

    #[test]
    fn test_chat_interface_send_multiple_messages() {
        let mut chat = ChatInterface::new();

        chat.input = "First".to_string();
        chat.send_current_input();

        chat.input = "Second".to_string();
        chat.send_current_input();

        chat.input = "Third".to_string();
        chat.send_current_input();

        // 3 user messages + 3 assistant responses = 6 total
        assert_eq!(chat.messages.len(), 6);
        assert_eq!(chat.messages[0].content, "First");
        assert_eq!(chat.messages[2].content, "Second");
        assert_eq!(chat.messages[4].content, "Third");
    }

    #[test]
    fn test_chat_interface_send_clears_input() {
        let mut chat = ChatInterface::new();
        chat.input = "Test".to_string();
        chat.send_current_input();
        assert!(chat.input.is_empty());
    }

    // ============================================================
    // ChatInterface add_message tests
    // ============================================================

    #[test]
    fn test_chat_interface_add_message_single() {
        let mut chat = ChatInterface::new();
        chat.add_message(ChatMessage::system("Welcome to zcode!"));

        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].role, "system");
    }

    #[test]
    fn test_chat_interface_add_message_multiple() {
        let mut chat = ChatInterface::new();
        chat.add_message(ChatMessage::system("Welcome"));
        chat.add_message(ChatMessage::user("Hi"));
        chat.add_message(ChatMessage::assistant("Hello"));

        assert_eq!(chat.messages.len(), 3);
        assert_eq!(chat.messages[0].role, "system");
        assert_eq!(chat.messages[1].role, "user");
        assert_eq!(chat.messages[2].role, "assistant");
    }

    #[test]
    fn test_chat_interface_add_message_preserves_order() {
        let mut chat = ChatInterface::new();
        chat.add_message(ChatMessage::user("First"));
        chat.add_message(ChatMessage::user("Second"));
        chat.add_message(ChatMessage::user("Third"));

        assert_eq!(chat.messages[0].content, "First");
        assert_eq!(chat.messages[1].content, "Second");
        assert_eq!(chat.messages[2].content, "Third");
    }

    // ============================================================
    // ChatInterface scroll tests
    // ============================================================

    #[test]
    fn test_chat_interface_scroll_default() {
        let chat = ChatInterface::new();
        assert_eq!(chat.scroll, 0);
    }

    #[test]
    fn test_chat_interface_scroll_can_be_modified() {
        let mut chat = ChatInterface::new();
        chat.scroll = 10;
        assert_eq!(chat.scroll, 10);
    }

    // ============================================================
    // ChatInterface debug tests
    // ============================================================

    #[test]
    fn test_chat_interface_debug() {
        let chat = ChatInterface::new();
        let debug_str = format!("{:?}", chat);
        assert!(debug_str.contains("ChatInterface"));
    }

    // ============================================================
    // Edge cases
    // ============================================================

    #[test]
    fn test_chat_interface_send_whitespace_only() {
        let mut chat = ChatInterface::new();
        chat.input = "   ".to_string();
        chat.send_current_input();
        // Whitespace is not empty, so it should send
        assert_eq!(chat.messages.len(), 2);
    }

    #[test]
    fn test_chat_interface_long_input() {
        let mut chat = ChatInterface::new();
        let long_input = "x".repeat(10000);
        chat.input = long_input.clone();
        chat.send_current_input();

        assert_eq!(chat.messages[0].content.len(), 10000);
    }

    #[test]
    fn test_chat_interface_special_characters() {
        let mut chat = ChatInterface::new();
        chat.input = "Test with \"quotes\" and 'apostrophes'".to_string();
        chat.send_current_input();

        assert!(chat.messages[0].content.contains("quotes"));
    }

    #[test]
    fn test_chat_interface_newlines_in_input() {
        let mut chat = ChatInterface::new();
        chat.input = "Line 1\nLine 2".to_string();
        chat.send_current_input();

        assert!(chat.messages[0].content.contains('\n'));
    }

    // ============================================================
    // Integration-like tests
    // ============================================================

    #[test]
    fn test_chat_interface_typical_conversation_flow() {
        let mut chat = ChatInterface::new();

        // Add system message
        chat.add_message(ChatMessage::system("You are a helpful assistant."));

        // User types and sends message
        chat.input_char('H');
        chat.input_char('i');
        chat.send_current_input();

        // More user input
        chat.input_char('H');
        chat.input_char('o');
        chat.input_char('w');
        chat.input_char(' ');
        chat.input_char('a');
        chat.input_char('r');
        chat.input_char('e');
        chat.input_char(' ');
        chat.input_char('y');
        chat.input_char('o');
        chat.input_char('u');
        chat.input_char('?');
        chat.send_current_input();

        // Should have: 1 system + 2 user + 2 assistant = 5 messages
        assert_eq!(chat.messages.len(), 5);
        assert!(chat.input.is_empty());
    }

    #[test]
    fn test_chat_interface_backspace_typing_correction() {
        let mut chat = ChatInterface::new();

        // Type "Hella"
        chat.input_char('H');
        chat.input_char('e');
        chat.input_char('l');
        chat.input_char('l');
        chat.input_char('a');

        // Correct to "Hello"
        chat.backspace();
        chat.input_char('o');

        assert_eq!(chat.input, "Hello");
    }
}
