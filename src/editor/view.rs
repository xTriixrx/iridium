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
        mode: &EditorMode,
        command_input: &str,
        scroll_offset: usize,
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
        let mode_text = ";";
        let display_command = if command_input.is_empty() {
            ":"
        } else {
            command_input
        };
        let command_text: String = if width > 0 {
            display_command.chars().take(width).collect()
        } else {
            String::new()
        };
        Terminal::print(&command_text)?;

        Ok(())
    }
}
