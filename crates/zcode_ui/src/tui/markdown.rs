//! Markdown rendering helpers for chat messages.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub(super) fn markdown_to_lines(content: &str, width: usize) -> Vec<Line<'static>> {
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

pub(super) fn visible_line_segment(line: &str, start_col: u16, width: u16) -> String {
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
