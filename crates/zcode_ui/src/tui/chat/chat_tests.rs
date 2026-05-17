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
