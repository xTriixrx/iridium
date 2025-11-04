use crate::editor::input::{InputAction, InputHandler};
use crate::editor::terminal::{Position, Size, Terminal};
use crate::editor::view::View;
use core::cmp::min;
use crossterm::event::{KeyCode, read};
use std::io::Error;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone)]
pub struct BufferEditor {
    quit: bool,
    name: String,
    mode: EditorMode,
    prev_mode: EditorMode,
    term: &'static Terminal,
    location: Location,
    input: InputHandler,
    command_input: String,
    scroll_offset: usize,
}

#[derive(Debug, Copy, Clone, Default)]
struct Location {
    x: usize,
    y: usize,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum EditorMode {
    #[default]
    Read,
    Insert,
    Command,
}

impl BufferEditor {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            quit: false,
            term: Terminal::instance(),
            name: name.into(),
            mode: EditorMode::default(),
            prev_mode: EditorMode::default(),
            location: Location::default(),
            input: InputHandler::new(),
            command_input: String::new(),
            scroll_offset: 0,
        }
    }

    pub fn instance() -> &'static Mutex<BufferEditor> {
        static INSTANCE: OnceLock<Mutex<BufferEditor>> = OnceLock::new();
        INSTANCE.get_or_init(|| Mutex::new(BufferEditor::new(String::new())))
    }

    pub fn open(&mut self, name: impl Into<String>) {
        self.name = name.into();
        self.quit = false;
        self.mode = EditorMode::Read;
        self.prev_mode = EditorMode::Read;
        self.location = Location::default();
        self.command_input.clear();
        self.scroll_offset = 0;
    }

    pub fn run(&mut self) {
        self.quit = false;
        self.term
            .enter()
            .expect("failed to prepare terminal session");
        let result = self.repl();
        Terminal::terminate().unwrap();
        result.unwrap();
    }

    fn repl(&mut self) -> Result<(), Error> {
        self.ensure_cursor_visible()?;
        loop {
            self.refresh_screen()?;

            if self.quit {
                break;
            }

            let event = read()?;
            if let Some(action) = self.input.process(&event, &self.mode, self.mode == EditorMode::Insert) {
                self.apply_input_action(action)?;
            }
        }

        Ok(())
    }

    fn move_point(&mut self, key_code: KeyCode) -> Result<(), Error> {
        let Location { mut x, mut y } = self.location;
        let Size { width, height } = Terminal::size()?;
        let content_height = height.saturating_sub(1);

        let buffer_view = View::snapshot(&self.name);
        let mut line_lengths = if buffer_view.line_count() == 0 {
            vec![0]
        } else {
            (0..buffer_view.line_count())
                .map(|row| buffer_view.char_count(row))
                .collect::<Vec<_>>()
        };
        let mut line_count = line_lengths.len();
        if line_count == 0 {
            line_lengths.push(0);
            line_count = 1;
        }

        let store_handle = self.term.store_handle();
        let mut store = store_handle.lock().expect("buffer store lock poisoned");
        if store.get(self.name.as_str()).is_none() {
            store.open(self.name.clone());
        }

        let line_length = |row: usize| -> usize { line_lengths.get(row).copied().unwrap_or(0) };

        match key_code {
            KeyCode::Up => {
                if y > 0 {
                    y -= 1;
                    x = min(x, line_length(y));
                }
            }
            KeyCode::Down => {
                if y + 1 < line_count {
                    y += 1;
                    x = min(x, line_length(y));
                } else if self.mode == EditorMode::Insert {
                    let last_row = line_count.saturating_sub(1);
                    let last_col = line_length(last_row);
                    let target_x = x;
                    let (new_row, _) = store.insert_newline(self.name.as_str(), last_row, last_col);
                    store.pad_line(self.name.as_str(), new_row, target_x);
                    line_lengths.push(target_x);
                    y = new_row;
                    x = target_x;
                }
            }
            KeyCode::Left => {
                if x > 0 {
                    x -= 1;
                } else if y > 0 {
                    y -= 1;
                    x = line_length(y);
                }
            }
            KeyCode::Right => {
                if x < line_length(y) {
                    x += 1;
                } else if self.mode == EditorMode::Insert {
                    let current_len = line_length(y);
                    store.insert_char(self.name.as_str(), y, current_len, ' ');
                    line_lengths[y] = current_len + 1;
                    x += 1;
                }
            }
            KeyCode::PageUp => {
                if content_height > 0 {
                    y = y.saturating_sub(content_height);
                } else {
                    y = 0;
                }
                x = min(x, line_length(y));
            }
            KeyCode::PageDown => {
                if content_height > 0 {
                    y = min(
                        line_count.saturating_sub(1),
                        y.saturating_add(content_height),
                    );
                }
                x = min(x, line_length(y));
            }
            KeyCode::Home => {
                x = 0;
            }
            KeyCode::End => {
                x = line_length(y);
                if width > 0 {
                    x = min(x, width.saturating_sub(1));
                }
            }
            _ => (),
        }

        drop(store);

        self.location = Location { x, y };
        self.ensure_cursor_visible()?;
        Ok(())
    }

    fn apply_input_action(&mut self, action: InputAction) -> Result<(), Error> {
        let mut redraw = false;

        match action {
            InputAction::Quit => {
                self.quit = true;
                self.command_input.clear();
                self.ensure_cursor_visible()?;
                redraw = true;
            }
            InputAction::MoveCursor(key) => {
                self.move_point(key)?;
                redraw = true;
            }
            InputAction::EnterCommandMode => {
                self.command_input = ":".to_string();
                self.enter_command_mode();
                self.ensure_cursor_visible()?;
                redraw = true;
            }
            InputAction::EnterInsertMode => {
                self.command_input.clear();
                self.enter_insert_mode();
                self.ensure_cursor_visible()?;
                redraw = true;
            }
            InputAction::EnterPreviousMode => {
                self.command_input.clear();
                self.enter_last_mode();
                self.ensure_cursor_visible()?;
                redraw = true;
            }
            InputAction::ExitInsertMode => {
                self.command_input.clear();
                self.enter_last_mode();
                self.ensure_cursor_visible()?;
                redraw = true;
            }
            InputAction::InsertChar(ch) => {
                if self.mode == EditorMode::Insert {
                    let position = Position {
                        col: self.location.x,
                        row: self.location.y,
                    };
                    let new_position = self.term.insert_char(self.name.as_str(), position, ch)?;
                    self.location = Location {
                        x: new_position.col,
                        y: new_position.row,
                    };
                    self.ensure_cursor_visible()?;
                    redraw = true;
                }
            }
            InputAction::InsertNewLine => {
                if self.mode == EditorMode::Insert {
                    let position = Position {
                        col: self.location.x,
                        row: self.location.y,
                    };
                    let new_position = self.term.insert_newline(self.name.as_str(), position)?;
                    self.location = Location {
                        x: new_position.col,
                        y: new_position.row,
                    };
                    self.ensure_cursor_visible()?;
                    redraw = true;
                }
            }
            InputAction::DeleteChar => {
                if self.mode == EditorMode::Insert {
                    let position = Position {
                        col: self.location.x,
                        row: self.location.y,
                    };
                    if let Some(new_position) =
                        self.term.delete_char(self.name.as_str(), position)?
                    {
                        self.location = Location {
                            x: new_position.col,
                            y: new_position.row,
                        };
                        self.ensure_cursor_visible()?;
                        redraw = true;
                    }
                }
            }
            InputAction::UpdateCommandBuffer(buffer) => {
                self.command_input = format!(":{}", buffer);
                redraw = true;
            }
            InputAction::ExecuteCommand(command) => {
                let command = command.trim();
                
                if command.is_empty() {
                    self.restore_after_command();
                }
                if command == "q" || command == "q!" {
                    self.quit = true;
                }
                else if command == "i" {
                    self.enter_insert_mode();
                }
                else if command == "r" {
                    self.enter_read_mode();
                }

                self.command_input.clear();
                self.ensure_cursor_visible()?;
                redraw = true;
            }
        }

        if redraw {
            self.refresh_screen()?;
        }

        Ok(())
    }

    fn refresh_screen(&self) -> Result<(), Error> {
        Terminal::hide_caret()?;
        Terminal::move_caret_to(Position::default())?;

        if self.quit {
            Terminal::clear_screen()?;
            let _ = Terminal::print("Closed editor.\r\n");
        } else {
            let buffer_view = View::snapshot(&self.name);
            View::render(&buffer_view, &self.mode, &self.command_input, self.scroll_offset)?;
            let Size { width, height } = Terminal::size()?;
            let cursor_position = if !self.command_input.is_empty() {
                let column = self
                    .command_input
                    .chars()
                    .count()
                    .min(width.saturating_sub(1));
                Position {
                    col: column,
                    row: height.saturating_sub(1),
                }
            } else {
                let content_height = height.saturating_sub(1);
                let screen_row = self.location.y.saturating_sub(self.scroll_offset);
                Position {
                    col: self.location.x.min(width.saturating_sub(1)),
                    row: screen_row.min(content_height.saturating_sub(1)),
                }
            };

            Terminal::move_caret_to(cursor_position)?;

            // Draw custom cursor glyph (U+2038: ‸) at the caret position.
            let cursor_glyph = '\u{2038}';
            let glyph = cursor_glyph.to_string();
            Terminal::print(&glyph)?;
            Terminal::move_caret_to(cursor_position)?;
        }

        Terminal::execute()?;
        Ok(())
    }

    fn ensure_cursor_visible(&mut self) -> Result<(), Error> {
        let Size { width, height } = Terminal::size()?;

        let content_height = height.saturating_sub(1);
        if content_height > 0 {
            if self.location.y < self.scroll_offset {
                self.scroll_offset = self.location.y;
            } else if self.location.y >= self.scroll_offset + content_height {
                self.scroll_offset = self.location.y + 1 - content_height;
            }
        } else {
            self.scroll_offset = self.location.y;
        }

        if width > 0 {
            self.location.x = self.location.x.min(width.saturating_sub(1));
        } else {
            self.location.x = 0;
        }

        Ok(())
    }

    fn enter_command_mode(&mut self) {
        self.prev_mode = self.mode;
        self.mode = EditorMode::Command;
    }

    fn enter_insert_mode(&mut self) {
        self.prev_mode = self.mode;
        self.mode = EditorMode::Insert;
    }

    fn enter_read_mode(&mut self) {
        self.prev_mode = self.mode;
        self.mode = EditorMode::Read;
    }

    fn enter_last_mode(&mut self) {
        let tmp = self.mode;
        self.mode = self.prev_mode;
        self.prev_mode = tmp;
    }

    fn restore_after_command(&mut self) {
        if self.mode == EditorMode::Command {
            self.mode = match self.prev_mode {
                EditorMode::Insert => EditorMode::Insert,
                EditorMode::Read => EditorMode::Read,
                _ => panic!("Unknown editor mode was entered! Editor mode: {:?}", self.mode),
            };
        }
    }

    pub fn prompt_string(&self) -> String {
        match self.mode {
            EditorMode::Read => format!("[buffer:{}] -- READ -- ", self.name),
            EditorMode::Insert => format!("[buffer:{}] -- INSERT -- ", self.name),
            EditorMode::Command => format!("[buffer:{}] ", self.name),
        }
    }
}
