//! Chat interface component for the TUI
//!
//! This module provides a chat interface with input and message display.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use super::markdown::{markdown_to_lines, visible_line_segment};

const MESSAGE_PREFIX_WIDTH: u16 = 11;
const SCROLL_STEP: u16 = 3;

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

/// An active ask-user request awaiting user selection.
pub struct PendingAsk {
    pub question: String,
    pub options: Vec<String>,
    pub selected: usize,
    pub response_tx: std::sync::mpsc::Sender<String>,
}

impl std::fmt::Debug for PendingAsk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingAsk")
            .field("question", &self.question)
            .field("options", &self.options)
            .field("selected", &self.selected)
            .finish_non_exhaustive()
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
    /// Total rendered message lines in the last frame.
    pub rendered_message_lines: u16,
    /// Visible message area height in the last frame.
    pub visible_message_height: u16,
    /// First visible input line.
    pub input_scroll: u16,
    /// First visible display column in the input area.
    pub input_col_scroll: u16,
    /// Whether full model thinking should be shown.
    pub show_full_thinking: bool,
    /// Latest collapsed thinking text.
    pub latest_thinking: String,
    /// Full thinking transcript for the current response.
    pub thinking_log: Vec<String>,
    /// Whether an LLM response is currently in flight.
    pub loading: bool,
    /// Loading animation frame.
    pub loading_frame: usize,
    /// Number of queued prompts waiting to be sent.
    pub pending_count: usize,
    /// Active ask-user request waiting for selection.
    pub pending_ask: Option<PendingAsk>,
}

impl ChatInterface {
    /// Create a new chat interface
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor_pos: 0,
            messages: Vec::new(),
            scroll: 0,
            rendered_message_lines: 0,
            visible_message_height: 0,
            input_scroll: 0,
            input_col_scroll: 0,
            show_full_thinking: false,
            latest_thinking: String::new(),
            thinking_log: Vec::new(),
            loading: false,
            loading_frame: 0,
            pending_count: 0,
            pending_ask: None,
        }
    }

    /// Insert a character at the current cursor position
    pub fn input_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
        self.ensure_cursor_visible();
    }

    /// Insert a newline at the current cursor position (Alt+Enter / Shift+Enter)
    pub fn input_newline(&mut self) {
        self.input.insert(self.cursor_pos, '\n');
        self.cursor_pos += 1;
        self.ensure_cursor_visible();
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
        self.ensure_cursor_visible();
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
        self.ensure_cursor_visible();
    }

    /// Move cursor right by one codepoint
    pub fn cursor_right(&mut self) {
        if self.cursor_pos >= self.input.len() {
            return;
        }
        let c = self.input[self.cursor_pos..].chars().next().unwrap();
        self.cursor_pos += c.len_utf8();
        self.ensure_cursor_visible();
    }

    /// Scroll chat up
    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(SCROLL_STEP);
    }

    /// Scroll chat down
    pub fn scroll_down(&mut self) {
        let max_scroll = self.max_scroll();
        self.scroll = self.scroll.saturating_add(SCROLL_STEP).min(max_scroll);
    }

    /// Scroll chat to the bottom.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll = u16::MAX;
    }

    /// Clamp scroll after rendering or message changes.
    pub fn clamp_scroll(&mut self) {
        self.scroll = self.scroll.min(self.max_scroll());
    }

    fn max_scroll(&self) -> u16 {
        if self.visible_message_height == 0 {
            return u16::MAX;
        }
        self.rendered_message_lines
            .saturating_sub(self.visible_message_height)
    }

    fn ensure_cursor_visible(&mut self) {
        let (row, _) = self.cursor_row_col();
        let visible_rows = self.input_visible_rows();
        if row < self.input_scroll {
            self.input_scroll = row;
        } else if row >= self.input_scroll.saturating_add(visible_rows) {
            self.input_scroll = row.saturating_sub(visible_rows.saturating_sub(1));
        }
    }

    fn update_input_col_scroll(&mut self, input_area: Rect) {
        let (_, col) = self.cursor_row_col();
        let visible_cols = input_area.width.saturating_sub(2).max(1);
        if col < self.input_col_scroll {
            self.input_col_scroll = col;
        } else if col >= self.input_col_scroll.saturating_add(visible_cols) {
            self.input_col_scroll = col.saturating_sub(visible_cols.saturating_sub(1));
        }
    }

    fn input_visible_rows(&self) -> u16 {
        self.input_line_count().min(8)
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
        let col = before
            .split('\n')
            .last()
            .map(UnicodeWidthStr::width)
            .unwrap_or(0) as u16;
        (row, col)
    }

    /// Take and clear the current input. Returns the user's message if non-empty.
    pub fn take_current_input(&mut self) -> Option<String> {
        if self.input.is_empty() {
            return None;
        }
        let text = self.input.clone();
        self.input.clear();
        self.cursor_pos = 0;
        self.input_scroll = 0;
        self.input_col_scroll = 0;
        Some(text)
    }

    /// Send the current input as a user message. Returns the sent message if non-empty.
    pub fn send_current_input(&mut self) -> Option<String> {
        let text = self.take_current_input()?;
        self.messages.push(ChatMessage::user(text.clone()));
        Some(text)
    }

    /// Add a message to the chat
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        self.scroll_to_bottom();
    }

    /// Start the loading/thinking UI for a new assistant response.
    pub fn start_loading(&mut self) {
        self.loading = true;
        self.loading_frame = 0;
        self.latest_thinking.clear();
        self.thinking_log.clear();
    }

    /// Stop the loading animation.
    pub fn stop_loading(&mut self) {
        self.loading = false;
        self.latest_thinking.clear();
    }

    /// Advance the loading animation frame.
    pub fn tick_loading(&mut self) {
        if self.loading {
            self.loading_frame = self.loading_frame.wrapping_add(1);
        }
    }

    /// Toggle full thinking transcript display.
    pub fn toggle_thinking(&mut self) {
        self.show_full_thinking = !self.show_full_thinking;
    }

    /// Append thinking text from the stream.
    pub fn append_thinking(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.latest_thinking.push_str(text);
        if self.latest_thinking.chars().count() > 240 {
            self.latest_thinking = self
                .latest_thinking
                .chars()
                .rev()
                .take(240)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
        }
        self.thinking_log.push(text.to_string());
        self.scroll_to_bottom();
    }

    /// Append assistant stream content, creating an assistant message if needed.
    pub fn append_assistant_delta(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.messages.last_mut() {
            if last.role == "assistant" {
                last.content.push_str(text);
                self.scroll_to_bottom();
                return;
            }
        }
        self.messages.push(ChatMessage::assistant(text.to_string()));
        self.scroll_to_bottom();
    }

    /// Remove the most recent user turn and its following assistant response.
    pub fn undo_last_turn(&mut self) -> bool {
        let Some(user_index) = self.messages.iter().rposition(|msg| msg.role == "user") else {
            return false;
        };
        self.messages.truncate(user_index);
        self.scroll_to_bottom();
        true
    }

    /// Compact older visible chat history into a deterministic summary.
    pub fn compact_messages(&mut self, retain_recent: usize, summary_max_chars: usize) -> bool {
        let non_system_count = self
            .messages
            .iter()
            .filter(|msg| msg.role != "system")
            .count();
        if non_system_count <= retain_recent {
            return false;
        }

        let split_non_system_at = non_system_count - retain_recent;
        let mut seen_non_system = 0usize;
        let mut compacted = Vec::new();
        let mut retained = Vec::new();

        for message in self.messages.drain(..) {
            if message.role == "system" {
                retained.push(message);
                continue;
            }

            if seen_non_system < split_non_system_at {
                compacted.push(message);
            } else {
                retained.push(message);
            }
            seen_non_system += 1;
        }

        let summary = summarize_chat_messages(&compacted, summary_max_chars);
        retained.insert(0, ChatMessage::system(summary));
        self.messages = retained;
        self.scroll_to_bottom();
        true
    }

    /// Replace all visible messages.
    pub fn replace_messages(&mut self, messages: Vec<ChatMessage>) {
        self.messages = messages;
        self.scroll_to_bottom();
    }

    /// Render the chat interface and position the cursor in the input area
    pub fn render(
        &mut self,
        frame: &mut Frame,
        agent_statuses: &[(String, String)],
        active_skills: &[String],
        active_mcps: &[String],
    ) {
        let area = frame.size();

        // Dynamic input height: 2 border + lines (capped at 8)
        let input_lines = if let Some(ask) = &self.pending_ask {
            ask.options.len().min(8) as u16
        } else {
            self.input_line_count().min(8)
        };
        let input_height = input_lines + 2; // +2 for borders

        // Create layout: Main Content, Input, Status Bar
        let root_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),               // Main area
                Constraint::Length(input_height), // Input area
                Constraint::Length(1),            // Bottom hotkeys
            ])
            .split(area);

        // Split Main area into Chat and Sidebar
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(75), // Chat takes 75%
                Constraint::Percentage(25), // Context sidebar takes 25%
            ])
            .split(root_chunks[0]);

        // Render messages area (Chat)
        self.update_message_scroll_metrics(main_chunks[0]);
        let messages_widget = self.render_messages(main_chunks[0]);
        frame.render_widget(messages_widget, main_chunks[0]);

        // Render Context Sidebar
        let sidebar_widget = self.render_sidebar(agent_statuses, active_skills, active_mcps);
        frame.render_widget(sidebar_widget, main_chunks[1]);

        // Render input area
        self.update_input_col_scroll(root_chunks[1]);
        let input_widget = self.render_input(root_chunks[1]);
        frame.render_widget(input_widget, root_chunks[1]);

        // Render Bottom Hotkeys Status Bar
        let status_text = Line::from(vec![
            Span::styled(
                " zcode ",
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | "),
            Span::styled("Enter:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Send | "),
            Span::styled(
                "Alt+Enter / Ctrl+J:",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Newline | "),
            Span::styled("↑/↓:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Scroll | "),
            Span::styled("Ctrl+O:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Thinking | "),
            Span::styled("Esc:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(if self.loading { " Stop | " } else { " Quit | " }),
            Span::styled("/", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Commands"),
        ]);
        frame.render_widget(
            Paragraph::new(status_text).style(Style::default().fg(Color::DarkGray)),
            root_chunks[2],
        );

        // Position the cursor: highlight selected option in ask mode, or text cursor
        if let Some(ask) = &self.pending_ask {
            let row = ask.selected.min(u16::MAX as usize) as u16;
            frame.set_cursor(root_chunks[1].x + 1, root_chunks[1].y + 1 + row);
        } else {
            let (cur_row, cur_col) = self.cursor_row_col();
            let visible_row = cur_row.saturating_sub(self.input_scroll);
            let visible_col = cur_col.saturating_sub(self.input_col_scroll);
            frame.set_cursor(
                root_chunks[1].x + 1 + visible_col,
                root_chunks[1].y + 1 + visible_row,
            );
        }
    }

    fn update_message_scroll_metrics(&mut self, area: Rect) {
        let lines = self.build_message_lines(area.width);
        self.rendered_message_lines = lines.len().min(u16::MAX as usize) as u16;
        self.visible_message_height = area.height;
        self.clamp_scroll();
    }

    /// Render the context sidebar
    fn render_sidebar<'a>(
        &self,
        agent_statuses: &'a [(String, String)],
        active_skills: &'a [String],
        active_mcps: &'a [String],
    ) -> Paragraph<'a> {
        let mut lines = Vec::new();

        // 1. Agent Statuses
        lines.push(Line::from(vec![Span::styled(
            "🤖 AI Agents ",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::White)
                .bg(Color::Rgb(60, 60, 60)),
        )]));
        for (name, status) in agent_statuses {
            let status_color = match status.as_str() {
                "Idle" => Color::DarkGray,
                s if s.starts_with("Thinking") => Color::Yellow,
                _ => Color::Red,
            };
            lines.push(Line::from(vec![
                Span::raw("   "),
                // Fixed-width formatting helps align the statuses
                Span::styled(format!("{:<15}", name), Style::default().fg(Color::White)),
                Span::styled(format!("⦾ {}", status), Style::default().fg(status_color)),
            ]));
        }
        lines.push(Line::from(""));

        // 2. Active MCPs
        lines.push(Line::from(vec![Span::styled(
            format!(" 🔌 MCP Servers ({}) ", active_mcps.len()),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::White)
                .bg(Color::Rgb(60, 60, 60)),
        )]));
        if active_mcps.is_empty() {
            lines.push(Line::from(Span::styled(
                "   No MCPs enabled",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for mcp in active_mcps {
                lines.push(Line::from(vec![
                    Span::raw("   • "),
                    Span::styled(mcp.clone(), Style::default().fg(Color::Cyan)),
                ]));
            }
        }
        lines.push(Line::from(""));

        // 3. Active Skills
        lines.push(Line::from(vec![Span::styled(
            format!(" 📚 Active Skills ({}) ", active_skills.len()),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::White)
                .bg(Color::Rgb(60, 60, 60)),
        )]));
        if active_skills.is_empty() {
            lines.push(Line::from(Span::styled(
                "   No Skills attached",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for skill in active_skills {
                lines.push(Line::from(vec![
                    Span::raw("   • "),
                    Span::styled(skill.clone(), Style::default().fg(Color::Green)),
                ]));
            }
        }

        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .wrap(Wrap { trim: false })
    }

    /// Render the messages area
    fn render_messages(&self, area: Rect) -> Paragraph<'_> {
        let mut lines = self.build_message_lines(area.width);
        if lines.is_empty() {
            let welcome = vec![
                Line::from(Span::styled(
                    "╭────────────────────────╮",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "│ Welcome to zcode agent │",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "╰────────────────────────╯",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Start typing a task to interact.",
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            lines.extend(welcome);
        }

        // We use Block::default() implicitly with NO borders, achieving the edge-to-edge look!
        Paragraph::new(Text::from(lines)).scroll((self.scroll, 0))
    }

    fn build_message_lines(&self, area_width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        for message in &self.messages {
            lines.extend(self.render_message_lines(message, area_width));
            lines.push(Line::from(""));
        }

        if self.loading
            || !self.latest_thinking.is_empty()
            || (self.show_full_thinking && !self.thinking_log.is_empty())
        {
            lines.extend(self.render_thinking_lines(area_width));
        }
        lines
    }

    fn render_message_lines(&self, message: &ChatMessage, area_width: u16) -> Vec<Line<'static>> {
        let (role_style, prefix) = match message.role.as_str() {
            "user" => (Style::default().fg(Color::Cyan), "👤 You     "),
            "assistant" => (Style::default().fg(Color::Magenta), "🤖 zcode   "),
            "system" => (Style::default().fg(Color::DarkGray), "⚙ System  "),
            _ => (Style::default(), ""),
        };
        let content_width = area_width.saturating_sub(MESSAGE_PREFIX_WIDTH + 3).max(10) as usize;
        let body_lines = markdown_to_lines(&message.content, content_width);
        let mut lines = Vec::new();

        if body_lines.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                prefix,
                role_style.add_modifier(Modifier::BOLD),
            )]));
            return lines;
        }

        for (index, mut body_line) in body_lines.into_iter().enumerate() {
            let prefix_span = if index == 0 {
                Span::styled(prefix, role_style.add_modifier(Modifier::BOLD))
            } else {
                Span::raw(" ".repeat(MESSAGE_PREFIX_WIDTH as usize))
            };
            let mut spans = vec![prefix_span];
            spans.append(&mut body_line.spans);
            lines.push(Line::from(spans));
        }
        lines
    }

    fn render_thinking_lines(&self, area_width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let spinner = ["|", "/", "-", "\\"][self.loading_frame % 4];
        let label = if self.loading {
            format!("{} thinking ", spinner)
        } else {
            "  thinking log ".to_string()
        };
        let queue = if self.pending_count > 0 {
            format!("  queued: {}", self.pending_count)
        } else {
            String::new()
        };
        let latest = if self.latest_thinking.is_empty() {
            if self.loading {
                "waiting for model...".to_string()
            } else {
                "complete".to_string()
            }
        } else {
            self.latest_thinking
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        };
        let content_width = area_width.saturating_sub(18).max(10) as usize;
        let collapsed = textwrap::wrap(&latest, content_width)
            .first()
            .map(|line| line.to_string())
            .unwrap_or(latest);

        lines.push(Line::from(vec![
            Span::styled(
                label,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(collapsed, Style::default().fg(Color::DarkGray)),
            Span::styled(queue, Style::default().fg(Color::DarkGray)),
        ]));

        if self.show_full_thinking && !self.thinking_log.is_empty() {
            let full = self.thinking_log.join("");
            for line in textwrap::wrap(&full, area_width.saturating_sub(4).max(10) as usize) {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(line.to_string(), Style::default().fg(Color::DarkGray)),
                ]));
            }
        }
        lines.push(Line::from(""));
        lines
    }

    /// Render the input area
    /// Select the next option down (wraps).
    pub fn ask_select_down(&mut self) {
        if let Some(ask) = &mut self.pending_ask {
            if ask.options.is_empty() {
                return;
            }
            ask.selected = (ask.selected + 1) % ask.options.len();
        }
    }

    /// Select the next option up (wraps).
    pub fn ask_select_up(&mut self) {
        if let Some(ask) = &mut self.pending_ask {
            if ask.options.is_empty() {
                return;
            }
            ask.selected = if ask.selected == 0 {
                ask.options.len() - 1
            } else {
                ask.selected - 1
            };
        }
    }

    /// Confirm the current selection and return the response channel + answer.
    pub fn ask_confirm(&mut self) -> Option<(std::sync::mpsc::Sender<String>, String)> {
        self.pending_ask.take().map(|ask| {
            let answer = ask.options.get(ask.selected).cloned().unwrap_or_default();
            (ask.response_tx, answer)
        })
    }

    /// Cancel the pending ask.
    pub fn ask_cancel(&mut self) -> Option<std::sync::mpsc::Sender<String>> {
        self.pending_ask.take().map(|ask| ask.response_tx)
    }

    /// Render the input area.
    fn render_input(&self, area: Rect) -> Paragraph<'_> {
        let input_text = if let Some(ask) = &self.pending_ask {
            let lines: Vec<Line<'_>> = ask
                .options
                .iter()
                .enumerate()
                .map(|(i, opt)| {
                    let marker = if i == ask.selected { " > " } else { "   " };
                    let style = if i == ask.selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    };
                    Line::from(Span::styled(format!("{}{}", marker, opt), style))
                })
                .collect();
            Text::from(lines)
        } else if self.input.is_empty() {
            Text::from(Span::styled(
                "Type a message...",
                Style::default().fg(Color::DarkGray),
            ))
        } else {
            let visible_cols = area.width.saturating_sub(2).max(1);
            let lines: Vec<Line<'_>> = self
                .input
                .split('\n')
                .skip(self.input_scroll as usize)
                .take(self.input_visible_rows() as usize)
                .map(|line| {
                    Line::from(visible_line_segment(
                        line,
                        self.input_col_scroll,
                        visible_cols,
                    ))
                })
                .collect();
            Text::from(lines)
        };

        Paragraph::new(input_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(if self.pending_ask.is_some() {
                    Line::from(Span::styled(
                        " ↑/↓ Select · Enter Confirm · Esc Cancel ",
                        Style::default().fg(Color::Yellow),
                    ))
                } else {
                    Line::from("")
                })
                .border_style(Style::default().fg(if self.pending_ask.is_some() {
                    Color::Yellow
                } else if self.input.is_empty() {
                    Color::DarkGray
                } else {
                    Color::Cyan
                })),
        )
    }
}

fn summarize_chat_messages(messages: &[ChatMessage], max_chars: usize) -> String {
    let mut out = String::from("Compacted conversation summary:\n");
    for message in messages {
        if message.content.trim().is_empty() {
            continue;
        }
        let line = format!(
            "- {}: {}\n",
            message.role,
            message.content.replace('\n', " ")
        );
        if out.len() + line.len() > max_chars {
            out.push_str("- ... summary truncated\n");
            break;
        }
        out.push_str(&line);
    }
    out
}

#[cfg(test)]
mod chat_tests;
