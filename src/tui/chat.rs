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
    /// Cursor byte position within input
    pub cursor_pos: usize,
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
            cursor_pos: 0,
            messages: Vec::new(),
            scroll: 0,
        }
    }

    /// Insert a character at the current cursor position
    pub fn input_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    /// Insert a newline at the current cursor position (Alt+Enter / Shift+Enter)
    pub fn input_newline(&mut self) {
        self.input.insert(self.cursor_pos, '\n');
        self.cursor_pos += 1;
    }

    /// Delete the character before the cursor (backspace)
    pub fn backspace(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        // Walk back to the start of the previous codepoint
        let mut pos = self.cursor_pos;
        loop {
            pos -= 1;
            if self.input.is_char_boundary(pos) {
                break;
            }
        }
        self.input.remove(pos);
        self.cursor_pos = pos;
    }

    /// Move cursor left by one codepoint
    pub fn cursor_left(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let mut pos = self.cursor_pos;
        loop {
            pos -= 1;
            if self.input.is_char_boundary(pos) {
                break;
            }
        }
        self.cursor_pos = pos;
    }

    /// Move cursor right by one codepoint
    pub fn cursor_right(&mut self) {
        if self.cursor_pos >= self.input.len() {
            return;
        }
        let c = self.input[self.cursor_pos..].chars().next().unwrap();
        self.cursor_pos += c.len_utf8();
    }

    /// Number of lines in the current input (min 1)
    pub fn input_line_count(&self) -> u16 {
        let count = self.input.split('\n').count().max(1);
        count as u16
    }

    /// Compute the (row, col) of cursor_pos within the input text (0-indexed)
    pub fn cursor_row_col(&self) -> (u16, u16) {
        let before = &self.input[..self.cursor_pos];
        let row = before.chars().filter(|&c| c == '\n').count() as u16;
        let col = before.split('\n').last().map(|s| s.chars().count()).unwrap_or(0) as u16;
        (row, col)
    }

    /// Send the current input as a message. Returns the user's message if non-empty.
    /// The caller is responsible for generating and adding the assistant response.
    pub fn send_current_input(&mut self) -> Option<String> {
        if self.input.is_empty() {
            return None;
        }
        let text = self.input.clone();
        let message = ChatMessage::user(text.clone());
        self.messages.push(message);
        self.input.clear();
        self.cursor_pos = 0;
        Some(text)
    }

    /// Add a message to the chat
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    /// Render the chat interface and position the cursor in the input area
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.size();

        // Dynamic input height: 2 border + lines (capped at 8)
        let input_lines = self.input_line_count().min(8);
        let input_height = input_lines + 2; // +2 for borders

        // Create layout: messages on top, input at bottom
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(input_height)])
            .split(area);

        // Render messages area
        let messages_widget = self.render_messages(chunks[0]);
        frame.render_widget(messages_widget, chunks[0]);

        // Render input area
        let input_widget = self.render_input(chunks[1]);
        frame.render_widget(input_widget, chunks[1]);

        // Position the cursor inside the input box
        // chunks[1]: x=left border, y=top border; inner starts at +1
        let (cur_row, cur_col) = self.cursor_row_col();
        // Clamp to visible area
        let max_visible_row = input_height.saturating_sub(3); // -2 border -1 for 0-index
        let visible_row = cur_row.min(max_visible_row);
        frame.set_cursor(
            chunks[1].x + 1 + cur_col,   // +1 for left border
            chunks[1].y + 1 + visible_row, // +1 for top border
        );
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
            // Add a blank line between messages for readability
            lines.push(Line::from(""));
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "No messages yet. Start typing to chat!",
                Style::default().fg(Color::DarkGray),
            )));
        }

        Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::ALL).title("Chat"))
            .scroll((self.scroll, 0))
    }

    /// Render the input area — supports multi-line (\n in input)
    fn render_input(&self, _area: Rect) -> Paragraph<'_> {
        let input_text = if self.input.is_empty() {
            Text::from(Span::styled(
                "Type a message... (Enter: send, Alt+Enter: newline, ←→: move cursor)",
                Style::default().fg(Color::DarkGray),
            ))
        } else {
            // Render each line of the input separately
            let lines: Vec<Line<'_>> = self
                .input
                .split('\n')
                .map(|l| Line::from(l.to_string()))
                .collect();
            Text::from(lines)
        };

        Paragraph::new(input_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input  [Enter: send | Alt+Enter / Ctrl+J: newline | ←→: cursor]")
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
        let result = chat.send_current_input();
        assert!(chat.messages.is_empty());
        assert!(result.is_none());
    }

    #[test]
    fn test_chat_interface_send_single_message() {
        let mut chat = ChatInterface::new();
        chat.input = "Hello".to_string();
        let returned = chat.send_current_input();

        assert!(chat.input.is_empty());
        // send_current_input only adds the user message; assistant reply is added by the caller
        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].role, "user");
        assert_eq!(chat.messages[0].content, "Hello");
        assert_eq!(returned, Some("Hello".to_string()));
    }

    #[test]
    fn test_chat_interface_send_adds_assistant_response() {
        // Verify that send_current_input returns the user's message and the
        // caller (TuiApp.call_llm) is responsible for adding the assistant reply.
        let mut chat = ChatInterface::new();
        chat.input = "Hello".to_string();
        let returned = chat.send_current_input();

        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].role, "user");
        assert_eq!(returned, Some("Hello".to_string()));
    }

    #[test]
    fn test_chat_interface_send_multiple_messages() {
        let mut chat = ChatInterface::new();

        chat.input = "First".to_string();
        chat.send_current_input();
        // Simulate TuiApp adding assistant reply each time
        chat.add_message(ChatMessage::assistant("Reply 1"));

        chat.input = "Second".to_string();
        chat.send_current_input();
        chat.add_message(ChatMessage::assistant("Reply 2"));

        chat.input = "Third".to_string();
        chat.send_current_input();
        chat.add_message(ChatMessage::assistant("Reply 3"));

        // 3 user + 3 assistant = 6 total
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
        // Whitespace is not empty, so it should send — only adds the user message now
        assert_eq!(chat.messages.len(), 1);
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

        // Should have: 1 system + 2 user = 3 messages
        // (TuiApp.call_llm is responsible for adding assistant replies)
        assert_eq!(chat.messages.len(), 3);
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
