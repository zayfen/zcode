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
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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

fn markdown_to_lines(content: &str, width: usize) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let mut in_code = false;
    let raw_lines: Vec<&str> = content.lines().collect();
    let mut index = 0;

    while index < raw_lines.len() {
        let line = raw_lines[index].trim_end();
        if let Some(language) = code_fence_language(line) {
            in_code = !in_code;
            out.push(code_fence_line(language));
            index += 1;
            continue;
        }

        if in_code {
            out.extend(code_block_lines(line, width));
            index += 1;
            continue;
        }

        if line.trim().is_empty() {
            out.push(Line::from(""));
            index += 1;
            continue;
        }

        if let Some((consumed, table)) = parse_markdown_table(&raw_lines, index) {
            out.extend(table_to_lines(&table, width));
            index += consumed;
            continue;
        }

        let (marker, text, style) = markdown_line_parts(line);
        let wrap_width = width.saturating_sub(marker.width()).max(8);
        let wrapped = textwrap::wrap(text, wrap_width);
        if wrapped.is_empty() {
            out.push(Line::from(Span::styled(marker.to_string(), style)));
            index += 1;
            continue;
        }

        for (idx, part) in wrapped.iter().enumerate() {
            let current_marker = if idx == 0 {
                marker.to_string()
            } else {
                " ".repeat(marker.width())
            };
            let mut spans = vec![Span::styled(current_marker, style)];
            spans.extend(inline_markdown_spans(part, style));
            out.push(Line::from(spans));
        }
        index += 1;
    }

    if out.is_empty() && !content.is_empty() {
        out.push(Line::from(""));
    }
    out
}

fn code_fence_language(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix("```")?;
    Some(rest.trim())
}

fn code_fence_line(language: &str) -> Line<'static> {
    let mut spans = vec![Span::styled("```", Style::default().fg(Color::DarkGray))];
    if !language.is_empty() {
        spans.push(Span::styled(
            language.to_string(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }
    Line::from(spans)
}

fn code_block_lines(line: &str, width: usize) -> Vec<Line<'static>> {
    let width = width.max(1).min(u16::MAX as usize) as u16;
    let visual_width = line.width();
    if visual_width == 0 {
        return vec![Line::from(Span::styled(
            String::new(),
            Style::default().fg(Color::Green),
        ))];
    }

    let mut lines = Vec::new();
    let mut start = 0u16;
    while (start as usize) < visual_width {
        lines.push(Line::from(Span::styled(
            visible_line_segment(line, start, width),
            Style::default().fg(Color::Green),
        )));
        start = start.saturating_add(width);
    }
    lines
}

fn parse_markdown_table(lines: &[&str], start: usize) -> Option<(usize, Vec<Vec<String>>)> {
    if start + 1 >= lines.len() {
        return None;
    }

    let header = parse_table_row(lines[start].trim_end())?;
    let separator = parse_table_row(lines[start + 1].trim_end())?;
    if header.len() < 2 || separator.len() != header.len() || !is_table_separator_row(&separator) {
        return None;
    }

    let column_count = header.len();
    let mut table = vec![normalize_table_row(header, column_count)];
    let mut index = start + 2;
    while index < lines.len() {
        let line = lines[index].trim_end();
        if line.trim().is_empty() {
            break;
        }
        let Some(row) = parse_table_row(line) else {
            break;
        };
        if is_table_separator_row(&row) {
            break;
        }
        table.push(normalize_table_row(row, column_count));
        index += 1;
    }

    Some((index - start, table))
}

fn parse_table_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.contains('|') {
        return None;
    }
    let inner = trimmed
        .strip_prefix('|')
        .unwrap_or(trimmed)
        .strip_suffix('|')
        .unwrap_or_else(|| trimmed.strip_prefix('|').unwrap_or(trimmed));
    let cells = inner
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect::<Vec<_>>();
    (cells.len() >= 2).then_some(cells)
}

fn is_table_separator_row(row: &[String]) -> bool {
    row.iter().all(|cell| {
        let trimmed = cell.trim();
        let body = trimmed.trim_matches(':');
        body.len() >= 3 && body.chars().all(|ch| ch == '-')
    })
}

fn normalize_table_row(mut row: Vec<String>, column_count: usize) -> Vec<String> {
    row.resize(column_count, String::new());
    row.truncate(column_count);
    row
}

fn table_to_lines(table: &[Vec<String>], width: usize) -> Vec<Line<'static>> {
    let Some(header) = table.first() else {
        return Vec::new();
    };
    let widths = table_column_widths(table, width);
    let mut lines = Vec::new();
    lines.push(table_row_line(
        header,
        &widths,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));
    lines.push(table_separator_line(&widths));
    for row in table.iter().skip(1) {
        lines.push(table_row_line(
            row,
            &widths,
            Style::default().fg(Color::White),
        ));
    }
    lines
}

fn table_column_widths(table: &[Vec<String>], width: usize) -> Vec<usize> {
    let column_count = table.first().map(Vec::len).unwrap_or(0);
    if column_count == 0 {
        return Vec::new();
    }

    let mut widths = vec![1; column_count];
    for row in table {
        for (index, cell) in row.iter().enumerate().take(column_count) {
            widths[index] = widths[index].max(cell.width());
        }
    }

    let spacing = column_count.saturating_sub(1) * 2;
    let available = width.saturating_sub(spacing).max(column_count);
    let min_width = if available >= column_count * 3 { 3 } else { 1 };

    while widths.iter().sum::<usize>() > available {
        let Some((index, _)) = widths
            .iter()
            .enumerate()
            .filter(|(_, width)| **width > min_width)
            .max_by_key(|(_, width)| **width)
        else {
            break;
        };
        widths[index] -= 1;
    }

    widths
}

fn table_row_line(row: &[String], widths: &[usize], style: Style) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        let cell = row.get(index).map(String::as_str).unwrap_or("");
        spans.push(Span::styled(fit_table_cell(cell, *width), style));
    }
    Line::from(spans)
}

fn table_separator_line(widths: &[usize]) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            "─".repeat(*width),
            Style::default().fg(Color::DarkGray),
        ));
    }
    Line::from(spans)
}

fn fit_table_cell(cell: &str, width: usize) -> String {
    let fitted = truncate_to_width(cell, width);
    let padding = width.saturating_sub(fitted.width());
    format!("{}{}", fitted, " ".repeat(padding))
}

fn truncate_to_width(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }

    let suffix = if width > 1 { "…" } else { "" };
    let suffix_width = suffix.width();
    let limit = width.saturating_sub(suffix_width);
    let mut out = String::new();
    let mut current_width = 0usize;
    for ch in text.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if current_width + ch_width > limit {
            break;
        }
        out.push(ch);
        current_width += ch_width;
    }
    out.push_str(suffix);
    out
}

fn markdown_line_parts(line: &str) -> (&str, &str, Style) {
    let trimmed = line.trim_start();
    let indent_len = line.len().saturating_sub(trimmed.len());
    let indent = &line[..indent_len];
    if let Some(text) = trimmed.strip_prefix("### ") {
        return (
            "",
            text,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    }
    if let Some(text) = trimmed.strip_prefix("## ") {
        return (
            "",
            text,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    }
    if let Some(text) = trimmed.strip_prefix("# ") {
        return (
            "",
            text,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    }
    if let Some(text) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
    {
        return ("• ", text, Style::default().fg(Color::White));
    }
    if let Some((marker, text)) = split_ordered_list(trimmed) {
        return (marker, text, Style::default().fg(Color::White));
    }
    (indent, trimmed, Style::default())
}

fn split_ordered_list(line: &str) -> Option<(&str, &str)> {
    let dot = line.find(". ")?;
    if dot == 0 || !line[..dot].chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((&line[..dot + 2], &line[dot + 2..]))
}

fn inline_markdown_spans(text: &str, base_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(stripped) = rest.strip_prefix("**") {
            if let Some(end) = stripped.find("**") {
                let (bold, tail) = stripped.split_at(end);
                spans.push(Span::styled(
                    bold.to_string(),
                    base_style.add_modifier(Modifier::BOLD),
                ));
                rest = &tail[2..];
                continue;
            }
        }
        if let Some(stripped) = rest.strip_prefix('`') {
            if let Some(end) = stripped.find('`') {
                let (code, tail) = stripped.split_at(end);
                spans.push(Span::styled(code.to_string(), base_style.fg(Color::Yellow)));
                rest = &tail[1..];
                continue;
            }
        }

        let next_special = rest
            .find("**")
            .into_iter()
            .chain(rest.find('`'))
            .min()
            .unwrap_or(rest.len());
        let (plain, tail) = rest.split_at(next_special.max(1).min(rest.len()));
        spans.push(Span::styled(plain.to_string(), base_style));
        rest = tail;
    }
    spans
}

fn visible_line_segment(line: &str, start_col: u16, width: u16) -> String {
    let start_col = start_col as usize;
    let width = width as usize;
    let end_col = start_col.saturating_add(width);
    let mut current_col = 0usize;
    let mut out = String::new();

    for ch in line.chars() {
        let ch_width = ch.width().unwrap_or(0);
        let next_col = current_col.saturating_add(ch_width);
        if next_col > start_col && current_col < end_col {
            out.push(ch);
        }
        current_col = next_col;
        if current_col >= end_col {
            break;
        }
    }
    out
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
mod tests {
    use super::*;

    fn lines_to_plain_text(lines: &[Line<'static>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }

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
    fn test_chat_interface_take_current_input_does_not_add_message() {
        let mut chat = ChatInterface::new();
        chat.input = "Queued".to_string();

        let returned = chat.take_current_input();

        assert_eq!(returned, Some("Queued".to_string()));
        assert!(chat.input.is_empty());
        assert!(chat.messages.is_empty());
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

    #[test]
    fn test_chat_interface_append_assistant_delta_merges_last_assistant() {
        let mut chat = ChatInterface::new();

        chat.append_assistant_delta("hel");
        chat.append_assistant_delta("lo");

        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].role, "assistant");
        assert_eq!(chat.messages[0].content, "hello");
    }

    #[test]
    fn test_chat_interface_append_thinking_keeps_recent_collapsed_text() {
        let mut chat = ChatInterface::new();

        chat.append_thinking(&"x".repeat(300));

        assert_eq!(chat.thinking_log.len(), 1);
        assert!(chat.latest_thinking.chars().count() <= 240);
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

    #[test]
    fn test_chat_interface_scroll_down() {
        let mut chat = ChatInterface::new();
        chat.rendered_message_lines = 20;
        chat.visible_message_height = 5;
        chat.scroll_down();
        assert_eq!(chat.scroll, 3);
        chat.scroll_down();
        assert_eq!(chat.scroll, 6);
    }

    #[test]
    fn test_chat_interface_scroll_up() {
        let mut chat = ChatInterface::new();
        chat.scroll = 5;
        chat.scroll_up();
        assert_eq!(chat.scroll, 2);

        // Ensure it doesn't underflow
        chat.scroll_up();
        assert_eq!(chat.scroll, 0);
    }

    #[test]
    fn test_chat_interface_scroll_down_clamps_to_content() {
        let mut chat = ChatInterface::new();
        chat.rendered_message_lines = 7;
        chat.visible_message_height = 5;

        chat.scroll_down();

        assert_eq!(chat.scroll, 2);
    }

    // ============================================================
    // ChatInterface cursor_left tests
    // ============================================================

    #[test]
    fn test_chat_interface_cursor_left() {
        let mut chat = ChatInterface::new();
        chat.input = "Hello".to_string();
        chat.cursor_pos = chat.input.len();

        chat.cursor_left();
        assert_eq!(chat.cursor_pos, 4);

        chat.cursor_left();
        assert_eq!(chat.cursor_pos, 3);
    }

    #[test]
    fn test_chat_interface_cursor_left_boundary() {
        let mut chat = ChatInterface::new();
        chat.input = "Hi".to_string();
        chat.cursor_pos = 0;

        chat.cursor_left();
        assert_eq!(chat.cursor_pos, 0); // Should not go negative
    }

    #[test]
    fn test_chat_interface_cursor_left_unicode() {
        let mut chat = ChatInterface::new();
        chat.input = "你好".to_string();
        chat.cursor_pos = chat.input.len();

        chat.cursor_left();
        assert_eq!(chat.cursor_pos, 3); // '好' is 3 bytes

        chat.cursor_left();
        assert_eq!(chat.cursor_pos, 0);
    }

    // ============================================================
    // ChatInterface cursor_right tests
    // ============================================================

    #[test]
    fn test_chat_interface_cursor_right() {
        let mut chat = ChatInterface::new();
        chat.input = "Hi".to_string();
        chat.cursor_pos = 0;

        chat.cursor_right();
        assert_eq!(chat.cursor_pos, 1);

        chat.cursor_right();
        assert_eq!(chat.cursor_pos, 2);
    }

    #[test]
    fn test_chat_interface_cursor_right_boundary() {
        let mut chat = ChatInterface::new();
        chat.input = "Hi".to_string();
        chat.cursor_pos = 2; // at end

        chat.cursor_right();
        assert_eq!(chat.cursor_pos, 2); // Should not move
    }

    #[test]
    fn test_chat_interface_cursor_right_unicode() {
        let mut chat = ChatInterface::new();
        chat.input = "你好".to_string();
        chat.cursor_pos = 0;

        chat.cursor_right();
        assert_eq!(chat.cursor_pos, 3); // '你' is 3 bytes
    }

    // ============================================================
    // ChatInterface input_newline tests
    // ============================================================

    #[test]
    fn test_chat_interface_input_newline_empty() {
        let mut chat = ChatInterface::new();
        chat.input_newline();
        assert_eq!(chat.input, "\n");
        assert_eq!(chat.cursor_pos, 1);
    }

    #[test]
    fn test_chat_interface_input_newline_mid_text() {
        let mut chat = ChatInterface::new();
        chat.input = "Hello World".to_string();
        chat.cursor_pos = 5; // after "Hello"

        chat.input_newline();
        assert_eq!(chat.input, "Hello\n World");
        assert_eq!(chat.cursor_pos, 6);
    }

    #[test]
    fn test_chat_interface_input_newline_multiple() {
        let mut chat = ChatInterface::new();
        chat.input_newline();
        chat.input_newline();
        chat.input_newline();
        assert_eq!(chat.input, "\n\n\n");
    }

    // ============================================================
    // ChatInterface cursor_row_col tests
    // ============================================================

    #[test]
    fn test_chat_interface_cursor_row_col_empty() {
        let chat = ChatInterface::new();
        let (row, col) = chat.cursor_row_col();
        assert_eq!(row, 0);
        assert_eq!(col, 0);
    }

    #[test]
    fn test_chat_interface_cursor_row_col_simple() {
        let mut chat = ChatInterface::new();
        chat.input = "Hello".to_string();
        chat.cursor_pos = 5;
        let (row, col) = chat.cursor_row_col();
        assert_eq!(row, 0);
        assert_eq!(col, 5);
    }

    #[test]
    fn test_chat_interface_cursor_row_col_with_newlines() {
        let mut chat = ChatInterface::new();
        chat.input = "Line1\nLine2\nLine3".to_string();
        chat.cursor_pos = 11; // "Line1\nLine2|" (before the second \n)
        let (row, col) = chat.cursor_row_col();
        assert_eq!(row, 1);
        assert_eq!(col, 5);
    }

    #[test]
    fn test_chat_interface_cursor_row_col_unicode() {
        let mut chat = ChatInterface::new();
        chat.input = "你好\n世界".to_string();
        chat.cursor_pos = 6; // "你好|\n世界"
        let (row, col) = chat.cursor_row_col();
        assert_eq!(row, 0);
        assert_eq!(col, 4); // 2 wide unicode chars
    }

    #[test]
    fn test_chat_interface_input_col_scroll_follows_long_input() {
        let mut chat = ChatInterface::new();
        chat.input = "abcdefghijklmnop".to_string();
        chat.cursor_pos = chat.input.len();

        chat.update_input_col_scroll(Rect::new(0, 0, 8, 3));

        assert!(chat.input_col_scroll > 0);
    }

    #[test]
    fn test_visible_line_segment_handles_wide_chars() {
        assert_eq!(visible_line_segment("你好abc", 2, 4), "好ab");
    }

    #[test]
    fn test_markdown_to_lines_renders_inline_styles() {
        let lines = markdown_to_lines("## Title\n- **bold** and `code`", 40);

        assert!(lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .any(|span| span.content.as_ref() == "Title"));
        assert!(lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .any(|span| span.content.as_ref() == "bold"));
        assert!(lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .any(|span| span.content.as_ref() == "code"));
    }

    #[test]
    fn test_markdown_to_lines_renders_pipe_table() {
        let lines = markdown_to_lines("| Name | Value |\n| --- | --- |\n| src | directory |", 40);
        let rendered = lines_to_plain_text(&lines);

        assert_eq!(rendered.len(), 3);
        assert!(rendered[0].contains("Name"));
        assert!(rendered[0].contains("Value"));
        assert!(rendered[1].contains("─"));
        assert!(rendered[2].contains("src"));
        assert!(rendered[2].contains("directory"));
    }

    #[test]
    fn test_markdown_to_lines_table_handles_wide_chars() {
        let lines = markdown_to_lines("| 名称 | 值 |\n| --- | --- |\n| 源码 | 目录 |", 40);
        let rendered = lines_to_plain_text(&lines);

        assert_eq!(rendered.len(), 3);
        assert!(rendered[0].contains("名称"));
        assert!(rendered[2].contains("源码"));
        assert!(rendered[2].contains("目录"));
    }

    #[test]
    fn test_markdown_to_lines_does_not_parse_table_inside_code_block() {
        let lines = markdown_to_lines("```text\n| Name | Value |\n| --- | --- |\n```", 40);
        let rendered = lines_to_plain_text(&lines);

        assert_eq!(rendered[0], "```text");
        assert_eq!(rendered[1], "| Name | Value |");
        assert_eq!(rendered[2], "| --- | --- |");
        assert_eq!(rendered[3], "```");
    }

    #[test]
    fn test_markdown_to_lines_renders_code_fence_language() {
        let lines = markdown_to_lines("```rust\n    fn main() {}\n```", 80);
        let rendered = lines_to_plain_text(&lines);

        assert_eq!(rendered[0], "```rust");
        assert_eq!(rendered[1], "    fn main() {}");
        assert_eq!(rendered[2], "```");
    }

    #[test]
    fn test_markdown_to_lines_code_block_wraps_without_trimming_indent() {
        let lines = markdown_to_lines("```text\n    abcdef\n```", 6);
        let rendered = lines_to_plain_text(&lines);

        assert_eq!(rendered[1], "    ab");
        assert_eq!(rendered[2], "cdef");
    }

    #[test]
    fn test_chat_interface_undo_last_turn_removes_user_and_assistant() {
        let mut chat = ChatInterface::new();
        chat.add_message(ChatMessage::user("question"));
        chat.add_message(ChatMessage::assistant("answer"));

        assert!(chat.undo_last_turn());

        assert!(chat.messages.is_empty());
    }

    #[test]
    fn test_chat_interface_compact_messages_summarizes_old_history() {
        let mut chat = ChatInterface::new();
        for index in 0..6 {
            chat.add_message(ChatMessage::user(format!("q{}", index)));
        }

        assert!(chat.compact_messages(2, 200));

        assert_eq!(chat.messages[0].role, "system");
        assert!(chat.messages[0]
            .content
            .contains("Compacted conversation summary"));
        assert_eq!(chat.messages.len(), 3);
    }

    // ============================================================
    // ChatInterface input_line_count tests
    // ============================================================

    #[test]
    fn test_chat_interface_input_line_count_empty() {
        let chat = ChatInterface::new();
        assert_eq!(chat.input_line_count(), 1); // min 1
    }

    #[test]
    fn test_chat_interface_input_line_count_single() {
        let mut chat = ChatInterface::new();
        chat.input = "Hello".to_string();
        assert_eq!(chat.input_line_count(), 1);
    }

    #[test]
    fn test_chat_interface_input_line_count_multi() {
        let mut chat = ChatInterface::new();
        chat.input = "Line1\nLine2\nLine3".to_string();
        assert_eq!(chat.input_line_count(), 3);
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
