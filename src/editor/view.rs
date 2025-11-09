use std::io::Error;

use crate::editor::buffer_editor::EditorMode;

use super::terminal::{Size, Terminal};

#[derive(Debug, Clone)]
pub struct BufferView {
    lines: Vec<String>,
}

impl BufferView {
    pub fn new(buffer_name: &str) -> Self {
        let store_handle = Terminal::instance().store_handle();
        let lines = {
            let store = store_handle.lock().expect("buffer store lock poisoned");
            store
                .get(buffer_name)
                .map(|buffer| buffer.lines().to_vec())
                .unwrap_or_default()
        };

        Self { lines }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn char_count(&self, row: usize) -> usize {
        self.lines
            .get(row)
            .map(|line| line.chars().count())
            .unwrap_or(0)
    }

    pub fn line(&self, row: usize) -> Option<&str> {
        self.lines.get(row).map(|line| line.as_str())
    }

    pub fn char_at(&self, row: usize, col: usize) -> Option<char> {
        self.line(row).and_then(|line| line.chars().nth(col))
    }
}

pub struct View;

impl View {
    pub fn snapshot(buffer_name: &str) -> BufferView {
        BufferView::new(buffer_name)
    }

    pub fn render(
        view: &BufferView,
        buffer_name: &str,
        mode: &EditorMode,
        command_input: &str,
        status_message: Option<&str>,
        scroll_offset: usize,
        cursor_position: (usize, usize),
    ) -> Result<(), Error> {
        let Size { width, height } = Terminal::size()?;
        let command_row = height.saturating_sub(1);

        let mut edge_rendered = false;

        for row in 0..command_row {
            Terminal::clear_line()?;

            if let Some(line) = view.line(scroll_offset + row) {
                let display: String = if width > 0 {
                    line.chars().take(width).collect()
                } else {
                    String::new()
                };
                Terminal::print(&display)?;
            } else if !edge_rendered {
                edge_rendered = true;
                let edge_line = "\u{2015}".repeat(width.max(1));
                Terminal::print(&edge_line)?;
            }

            Terminal::print("\r\n")?;
        }
        Terminal::clear_line()?;
        let command_line = build_command_line(
            width,
            command_input,
            buffer_name,
            mode,
            cursor_position,
            status_message,
        );
        Terminal::print(&command_line)?;

        Ok(())
    }
}

fn build_command_line(
    width: usize,
    command_input: &str,
    buffer_name: &str,
    mode: &EditorMode,
    cursor_position: (usize, usize),
    status_message: Option<&str>,
) -> String {
    if width == 0 {
        return String::new();
    }

    let mut line: Vec<char> = vec![' '; width];

    let mode_label = format!("[{}]", mode_name(mode));
    let mode_chars: Vec<char> = mode_label.chars().collect();
    let (row, col) = cursor_position;
    let cursor_label = format!("{},{}", row, col);
    let name_and_cursor = format!("{} {}", buffer_name, cursor_label);

    if let Some(message) = status_message {
        let mode_len = mode_chars.len().min(width);
        if mode_len > 0 {
            let mode_start = width - mode_len;
            let slice_start = mode_chars.len().saturating_sub(mode_len);
            for (offset, ch) in mode_chars[slice_start..].iter().enumerate() {
                line[mode_start + offset] = *ch;
            }
        }

        let available_for_combo = width.saturating_sub(mode_len);
        let combo_raw = format!(" {} ", name_and_cursor);
        let combo_chars: Vec<char> = combo_raw.chars().collect();
        let combo_len = combo_chars.len().min(available_for_combo);
        if combo_len > 0 {
            let combo_start = available_for_combo - combo_len;
            let slice_start = combo_chars.len().saturating_sub(combo_len);
            for (offset, ch) in combo_chars[slice_start..].iter().enumerate() {
                line[combo_start + offset] = *ch;
            }
        }

        let message_width = width.saturating_sub(mode_len + combo_len);
        for (idx, ch) in message.chars().take(message_width).enumerate() {
            line[idx] = ch;
        }

        return line.iter().collect();
    } else {
        let display_command = if command_input.is_empty() {
            ":"
        } else {
            command_input
        };

        for (idx, ch) in display_command.chars().take(width).enumerate() {
            line[idx] = ch;
        }
    }

    if mode_chars.len() <= width {
        let start = width - mode_chars.len();
        for (offset, ch) in mode_chars.iter().enumerate() {
            let idx = start + offset;
            line[idx] = *ch;
        }
    }

    let combo_chars: Vec<char> = name_and_cursor.chars().collect();
    if !combo_chars.is_empty() && combo_chars.len() <= width {
        let start = width.saturating_sub(combo_chars.len()) / 2;
        for (offset, ch) in combo_chars.iter().enumerate() {
            let idx = start + offset;
            if idx < width {
                line[idx] = *ch;
            }
        }
    }

    line.iter().collect()
}

fn mode_name(mode: &EditorMode) -> &'static str {
    match mode {
        EditorMode::Insert => "INSERT",
        EditorMode::Read => "READ",
        EditorMode::Command => "COMMAND",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_includes_buffer_name_cursor_and_mode() {
        let line = build_command_line(40, "", "test.rs", &EditorMode::Insert, (3, 5), None);

        assert!(line.starts_with(":"));
        assert!(line.ends_with("[INSERT]"));

        let combo_index = line.find("test.rs 3,5").expect("buffer info missing");
        let combo_center = combo_index + "test.rs 3,5".len() / 2;
        let center = 40 / 2;
        assert!((combo_center as isize - center as isize).abs() <= 2);
    }

    #[test]
    fn command_line_respects_command_input_and_mode() {
        let line = build_command_line(40, ":w", "buffer", &EditorMode::Read, (1, 1), None);

        assert!(line.starts_with(":w"));
        assert!(line.ends_with("[READ]"));
        assert!(line.contains("buffer 1,1"));
    }

    #[test]
    fn cursor_position_changes_are_reflected() {
        let first = build_command_line(30, ":", "file", &EditorMode::Command, (2, 4), None);
        let second = build_command_line(30, ":", "file", &EditorMode::Command, (5, 10), None);

        assert!(first.contains("file 2,4"));
        assert!(second.contains("file 5,10"));
        assert_ne!(first, second);
    }

    #[test]
    fn status_message_overrides_command_input() {
        let line = build_command_line(
            80,
            ":w",
            "buffer",
            &EditorMode::Command,
            (1, 1),
            Some("This buffer is required to be saved."),
        );

        assert!(line.starts_with("This buffer is required to be saved"));
        assert!(line.contains("[COMMAND]"));
        assert!(line.contains("buffer 1,1"));
    }
}
