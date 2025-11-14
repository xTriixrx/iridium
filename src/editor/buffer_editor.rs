use crate::editor::input::{InputAction, InputHandler, NavigationCommand};
use crate::editor::terminal::{Position, Size, Terminal};
use crate::editor::view::View;
use core::cmp::min;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use crossterm::event::read;
use crossterm::event::{Event, poll};
use std::io::{Error, ErrorKind};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct BufferEditor {
    quit: bool,
    quit_all: bool,
    name: String,
    mode: EditorMode,
    prev_mode: EditorMode,
    term: &'static Terminal,
    location: Location,
    input: InputHandler,
    command_input: String,
    scroll_offset: usize,
    view_height: usize,
    pending_command: Option<PendingCommand>,
    status_message: Option<String>,
    cursor_blink_visible: bool,
    cursor_last_toggle: Instant,
}

#[derive(Debug, Copy, Clone, Default)]
struct Location {
    x: usize,
    y: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SaveIntent {
    BufferOnly,
    WriteAndQuit,
    ConditionalQuit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingCommand {
    Save(SaveIntent),
    QuitAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WordDirection {
    Left,
    Right,
}

const BUFFER_NAME_PROMPT: &str = "Buffer name: ";
const DIRTY_BUFFER_STATUS: &str = "This buffer is required to be saved.";

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum EditorMode {
    #[default]
    Read,
    Insert,
    Command,
    Navigation,
}

impl BufferEditor {
    const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(350);
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            quit: false,
            quit_all: false,
            term: Terminal::instance(),
            name: name.into(),
            mode: EditorMode::default(),
            prev_mode: EditorMode::default(),
            location: Location::default(),
            input: InputHandler::new(),
            command_input: String::new(),
            scroll_offset: 0,
            view_height: 0,
            pending_command: None,
            status_message: None,
            cursor_blink_visible: true,
            cursor_last_toggle: Instant::now(),
        }
    }

    pub fn instance() -> &'static Mutex<BufferEditor> {
        static INSTANCE: OnceLock<Mutex<BufferEditor>> = OnceLock::new();
        INSTANCE.get_or_init(|| Mutex::new(BufferEditor::new(String::new())))
    }

    pub fn open(&mut self, name: impl Into<String>) {
        self.name = name.into();
        self.quit = false;
        self.quit_all = false;
        self.mode = EditorMode::Read;
        self.prev_mode = EditorMode::Read;
        self.location = Location::default();
        self.command_input.clear();
        self.scroll_offset = 0;
        self.view_height = 0;
        self.pending_command = None;
        self.status_message = None;
        self.cursor_blink_visible = true;
        self.cursor_last_toggle = Instant::now();
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

            if let Some(event) = Self::poll_event_with_timeout(Self::CURSOR_BLINK_INTERVAL)? {
                if self.handle_prompt_input(&event)? {
                    continue;
                }

                if let Some(action) =
                    self.input
                        .process(&event, &self.mode, self.mode == EditorMode::Insert)
                {
                    self.apply_input_action(action)?;
                }
            }
        }

        Ok(())
    }

    fn poll_event_with_timeout(timeout: Duration) -> Result<Option<Event>, Error> {
        if poll(timeout)? {
            Ok(Some(read()?))
        } else {
            Ok(None)
        }
    }

    fn handle_prompt_input(&mut self, event: &Event) -> Result<bool, Error> {
        if self.mode != EditorMode::Command {
            return Ok(false);
        }

        if self.pending_command.is_none() || self.command_input.is_empty() {
            return Ok(false);
        }

        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Enter => {
                    let input = self.command_input.clone();
                    return self.process_prompt_input(input);
                }
                KeyCode::Esc => {
                    self.pending_command = None;
                    self.command_input.clear();
                    self.refresh_screen()?;
                    return Ok(true);
                }
                KeyCode::Backspace => {
                    if self.command_input.len() > BUFFER_NAME_PROMPT.len() {
                        self.command_input.pop();
                    }
                    self.refresh_screen()?;
                    return Ok(true);
                }
                KeyCode::Char(ch) => {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        || key.modifiers.contains(KeyModifiers::ALT)
                    {
                        return Ok(false);
                    }
                    self.command_input.push(ch);
                    self.refresh_screen()?;
                    return Ok(true);
                }
                _ => {}
            }
        }

        Ok(false)
    }

    fn process_prompt_input(&mut self, input: String) -> Result<bool, Error> {
        let Some(intent) = self.pending_command.take() else {
            return Ok(true);
        };

        let provided = input
            .strip_prefix(BUFFER_NAME_PROMPT)
            .unwrap_or(input.as_str())
            .trim();

        if provided.is_empty() {
            self.pending_command = Some(intent);
            self.command_input = BUFFER_NAME_PROMPT.to_string();
            self.refresh_screen()?;
            return Ok(true);
        }

        let desired_name = provided.to_string();
        let renamed = {
            let store_handle = self.term.store_handle();
            let mut store = store_handle.lock().expect("buffer store lock poisoned");
            store.rename(self.name.as_str(), &desired_name)
        };

        if !renamed {
            println!(
                "Failed to rename buffer '{}' to '{}'",
                self.name, desired_name
            );
            self.pending_command = Some(intent);
            self.command_input = BUFFER_NAME_PROMPT.to_string();
            self.refresh_screen()?;
            return Ok(true);
        }

        self.name = desired_name;
        self.command_input.clear();
        match intent {
            PendingCommand::Save(save_intent) => self.execute_save_intent(save_intent)?,
            PendingCommand::QuitAll => self.execute_quit_all()?,
        }
        self.refresh_screen()?;
        Ok(true)
    }

    fn move_point(&mut self, key_code: KeyCode) -> Result<(), Error> {
        let Location { mut x, mut y } = self.location;
        let Size { width, height } = Terminal::size()?;
        let content_height = height.saturating_sub(1);
        self.view_height = content_height.max(1);

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

    fn navigate_line(&mut self, command: NavigationCommand) -> Result<(), Error> {
        match command {
            NavigationCommand::LineStart => self.move_point(KeyCode::Home),
            NavigationCommand::LineEnd => self.move_point(KeyCode::End),
            NavigationCommand::PageStart => self.navigate_page(PageDirection::Up),
            NavigationCommand::PageEnd => self.navigate_page(PageDirection::Down),
            NavigationCommand::WordLeft => self.navigate_word(WordDirection::Left),
            NavigationCommand::WordRight => self.navigate_word(WordDirection::Right),
        }
    }

    fn navigate_page(&mut self, direction: PageDirection) -> Result<(), Error> {
        let buffer_view = View::snapshot(&self.name);
        let mut line_lengths = if buffer_view.line_count() == 0 {
            vec![0]
        } else {
            (0..buffer_view.line_count())
                .map(|row| buffer_view.char_count(row))
                .collect::<Vec<_>>()
        };
        if line_lengths.is_empty() {
            line_lengths.push(0);
        }

        let line_count = line_lengths.len();
        let line_length = |row: usize| -> usize { line_lengths.get(row).copied().unwrap_or(0) };
        let last_row = line_count.saturating_sub(1);

        let view_height = self.view_height.max(1);
        let visible_top = self.scroll_offset.min(last_row);
        let visible_bottom = self
            .scroll_offset
            .saturating_add(view_height.saturating_sub(1))
            .min(last_row);
        let half_stride = (view_height / 2).max(1);

        let target_y = match direction {
            PageDirection::Up => {
                if self.location.y == visible_top && self.scroll_offset > 0 {
                    self.scroll_offset = self.scroll_offset.saturating_sub(half_stride);
                }
                self.scroll_offset = self.scroll_offset.min(last_row);
                self.scroll_offset
            }
            PageDirection::Down => {
                if self.location.y == visible_bottom && self.scroll_offset < last_row {
                    self.scroll_offset = (self.scroll_offset + half_stride).min(last_row);
                }
                let mut bottom = self
                    .scroll_offset
                    .saturating_add(view_height.saturating_sub(1));
                if bottom > last_row {
                    bottom = last_row;
                }
                bottom
            }
        };

        let desired_x = self.location.x;
        let mut target_x = desired_x;

        let store_handle = self.term.store_handle();
        let mut store = store_handle.lock().expect("buffer store lock poisoned");
        if store.get(self.name.as_str()).is_none() {
            store.open(self.name.clone());
        }

        if target_y == 0 {
            target_x = min(desired_x, line_length(target_y));
        } else if desired_x > line_length(target_y) {
            store.pad_line(self.name.as_str(), target_y, desired_x);
            if let Some(len) = line_lengths.get_mut(target_y) {
                *len = desired_x;
            }
        } else {
            target_x = min(desired_x, line_length(target_y));
        }

        drop(store);

        self.location = Location {
            x: target_x,
            y: target_y,
        };
        self.ensure_cursor_visible()
    }

    fn navigate_word(&mut self, direction: WordDirection) -> Result<(), Error> {
        let buffer_view = View::snapshot(&self.name);
        let line = buffer_view
            .line(self.location.y)
            .unwrap_or_default()
            .to_string();
        let chars: Vec<char> = line.chars().collect();
        let mut target_x = self.location.x.min(chars.len());

        match direction {
            WordDirection::Left => {
                if target_x == 0 {
                    target_x = 0;
                } else {
                    let mut found = None;
                    for idx in 0..target_x {
                        if chars[idx] == ' ' {
                            found = Some(idx);
                        }
                    }
                    target_x = found.unwrap_or(0);
                }
            }
            WordDirection::Right => {
                if target_x >= chars.len() {
                    target_x = chars.len();
                } else {
                    let mut found = None;
                    for idx in target_x + 1..=chars.len() {
                        if idx < chars.len() && chars[idx] == ' ' {
                            found = Some(idx);
                            break;
                        }
                    }
                    target_x = found.unwrap_or(chars.len());
                }
            }
        }

        self.location.x = target_x;
        self.cursor_last_toggle = Instant::now();
        self.ensure_cursor_visible()
    }

    fn apply_input_action(&mut self, action: InputAction) -> Result<(), Error> {
        let mut redraw = false;
        let mut keep_command_text = false;
        let mut pending_mode_restore: Option<EditorMode> = None;
        let mut pending_status_restore: Option<Option<String>> = None;

        match action {
            InputAction::Quit => {
                self.clear_status_message();
                self.quit = true;
                self.command_input.clear();
                self.ensure_cursor_visible()?;
                redraw = true;
            }
            InputAction::MoveCursor(key) => {
                self.clear_status_message();
                self.move_point(key)?;
                redraw = true;
                self.cursor_last_toggle = Instant::now();
            }
            InputAction::EnterCommandMode => {
                self.clear_status_message();
                self.command_input = ":".to_string();
                self.enter_command_mode();
                self.ensure_cursor_visible()?;
                redraw = true;
                self.cursor_last_toggle = Instant::now();
            }
            InputAction::EnterInsertMode => {
                self.clear_status_message();
                self.command_input.clear();
                self.enter_insert_mode();
                self.ensure_cursor_visible()?;
                redraw = true;
                self.cursor_last_toggle = Instant::now();
            }
            InputAction::EnterPreviousMode => {
                self.clear_status_message();
                self.command_input.clear();
                self.enter_last_mode();
                self.ensure_cursor_visible()?;
                redraw = true;
                self.cursor_last_toggle = Instant::now();
            }
            InputAction::ExitInsertMode => {
                self.clear_status_message();
                self.command_input.clear();
                self.enter_last_mode();
                self.ensure_cursor_visible()?;
                redraw = true;
                self.cursor_last_toggle = Instant::now();
            }
            InputAction::Navigation(command) => {
                let previous_mode = self.mode;
                let previous_status = self.status_message.clone();
                self.clear_status_message();
                self.set_status_message("NAVIGATION MODE");
                self.mode = EditorMode::Navigation;
                if let Err(err) = self.navigate_line(command) {
                    self.mode = previous_mode;
                    self.status_message = previous_status;
                    return Err(err);
                }
                pending_mode_restore = Some(previous_mode);
                pending_status_restore = Some(previous_status);
                redraw = true;
            }
            InputAction::InsertChar(ch) => {
                self.clear_status_message();
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
                    self.cursor_last_toggle = Instant::now();
                }
            }
            InputAction::InsertNewLine => {
                self.clear_status_message();
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
                    self.cursor_last_toggle = Instant::now();
                }
            }
            InputAction::DeleteChar => {
                self.clear_status_message();
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
                        self.cursor_last_toggle = Instant::now();
                    }
                }
            }
            InputAction::UpdateCommandBuffer(buffer) => {
                self.clear_status_message();
                self.command_input = format!(":{}", buffer);
                redraw = true;
            }
            InputAction::ExecuteCommand(command) => {
                self.clear_status_message();
                keep_command_text = self.process_colon_command(command.trim())?;

                if !keep_command_text {
                    self.command_input.clear();
                }
                self.ensure_cursor_visible()?;
                redraw = true;
            }
        }

        if redraw {
            self.refresh_screen()?;
        }

        if let Some(mode) = pending_mode_restore {
            self.mode = mode;
        }
        if let Some(status) = pending_status_restore {
            self.status_message = status;
        }

        Ok(())
    }

    fn refresh_screen(&mut self) -> Result<(), Error> {
        if std::env::var("IRIDIUM_SKIP_EDITOR").is_ok() {
            return Ok(());
        }
        Terminal::hide_caret()?;
        Terminal::move_caret_to(Position::default())?;

        if self.quit {
            Terminal::clear_screen()?;
            let _ = Terminal::print("Closed editor.\r\n");
        } else {
            let buffer_view = View::snapshot(&self.name);
            View::render(
                &buffer_view,
                &self.name,
                &self.mode,
                &self.command_input,
                self.status_message.as_deref(),
                self.scroll_offset,
                (
                    self.location.y.saturating_add(1),
                    self.location.x.saturating_add(1),
                ),
            )?;
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
            let now = Instant::now();
            if now.duration_since(self.cursor_last_toggle) >= Self::CURSOR_BLINK_INTERVAL {
                self.cursor_blink_visible = !self.cursor_blink_visible;
                self.cursor_last_toggle = now;
            }

            let glyph = if self.cursor_blink_visible {
                '\u{2038}'.to_string()
            } else {
                buffer_view
                    .char_at(self.location.y, self.location.x)
                    .map(|ch| ch.to_string())
                    .unwrap_or_else(|| " ".to_string())
            };
            Terminal::print(&glyph)?;
            Terminal::move_caret_to(cursor_position)?;
        }

        Terminal::execute()?;
        Ok(())
    }

    fn ensure_cursor_visible(&mut self) -> Result<(), Error> {
        if std::env::var("IRIDIUM_SKIP_EDITOR").is_ok() {
            return Ok(());
        }
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
                EditorMode::Navigation => EditorMode::Navigation,
                _ => panic!(
                    "Unknown editor mode was entered! Editor mode: {:?}",
                    self.mode
                ),
            };
        }
    }

    fn clear_status_message(&mut self) {
        if self.status_message.is_some() {
            self.status_message = None;
        }
    }

    fn set_status_message(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    pub fn prompt_string(&self) -> String {
        match self.mode {
            EditorMode::Read => format!("[buffer:{}] -- READ -- ", self.name),
            EditorMode::Insert => format!("[buffer:{}] -- INSERT -- ", self.name),
            EditorMode::Command => format!("[buffer:{}] ", self.name),
            EditorMode::Navigation => format!("[buffer:{}] -- NAV -- ", self.name),
        }
    }

    fn buffer_is_dirty(&self) -> bool {
        let store_handle = self.term.store_handle();
        let store = store_handle.lock().expect("buffer store lock poisoned");
        store.is_dirty(self.name.as_str())
    }

    fn save_current_buffer(&self) -> Result<(), Error> {
        let store_handle = self.term.store_handle();
        let mut store = store_handle.lock().expect("buffer store lock poisoned");
        store.save(self.name.as_str())?;
        Ok(())
    }

    fn save_current_buffer_in_memory(&self) {
        let store_handle = self.term.store_handle();
        let mut store = store_handle.lock().expect("buffer store lock poisoned");
        let _ = store.save_in_memory(self.name.as_str());
    }

    fn handle_save_command(&mut self, intent: SaveIntent) -> Result<bool, Error> {
        if self.buffer_requires_name() {
            self.pending_command = Some(PendingCommand::Save(intent));
            self.command_input = BUFFER_NAME_PROMPT.to_string();
            self.refresh_screen()?;
            return Ok(true);
        }

        self.execute_save_intent(intent)?;
        Ok(false)
    }

    fn handle_quit_all_command(&mut self) -> Result<bool, Error> {
        if self.buffer_requires_name() {
            self.pending_command = Some(PendingCommand::QuitAll);
            self.command_input = BUFFER_NAME_PROMPT.to_string();
            self.refresh_screen()?;
            return Ok(true);
        }

        self.execute_quit_all()?;
        Ok(false)
    }

    fn execute_save_intent(&mut self, intent: SaveIntent) -> Result<(), Error> {
        match intent {
            SaveIntent::BufferOnly => {
                self.save_current_buffer()?;
            }
            SaveIntent::WriteAndQuit => {
                self.save_current_buffer()?;
                self.quit = true;
            }
            SaveIntent::ConditionalQuit => {
                if self.buffer_is_dirty() {
                    println!("Buffer has unsaved changes. Use :w or :wq.");
                } else {
                    self.quit = true;
                }
            }
        }

        self.pending_command = None;
        Ok(())
    }

    fn execute_quit_all(&mut self) -> Result<(), Error> {
        self.quit = true;
        self.quit_all = true;
        self.pending_command = None;
        Ok(())
    }

    fn buffer_requires_name(&self) -> bool {
        let store_handle = self.term.store_handle();
        let store = store_handle.lock().expect("buffer store lock poisoned");
        store.requires_name(self.name.as_str())
    }

    pub fn take_quit_all_request(&mut self) -> bool {
        let requested = self.quit_all;
        if requested {
            self.quit_all = false;
        }
        requested
    }

    pub fn quit_all_now(&mut self) -> Result<(), Error> {
        if self.buffer_requires_name() {
            return Err(Error::new(
                ErrorKind::Other,
                "Buffer must be named before quitting all",
            ));
        }
        self.execute_quit_all()
    }

    pub fn jump_to_buffer(&mut self, name: &str) -> Result<(), Error> {
        self.switch_to_buffer(name)
    }

    fn cycle_buffer(&mut self, forward: bool) -> Result<(), Error> {
        let store_handle = self.term.store_handle();
        let store = store_handle.lock().expect("buffer store lock poisoned");
        let mut buffers = store.list();
        if buffers.len() <= 1 {
            return Ok(());
        }
        buffers.sort();
        let Some(idx) = buffers.iter().position(|name| name == &self.name) else {
            return Ok(());
        };
        let len = buffers.len();
        let next_idx = if forward {
            (idx + 1) % len
        } else {
            (idx + len - 1) % len
        };
        let next_name = buffers[next_idx].clone();
        drop(store);

        let previous_mode = self.mode;
        self.open(next_name);
        self.mode = previous_mode;
        self.prev_mode = previous_mode;

        if std::env::var("IRIDIUM_SKIP_EDITOR").is_err() {
            self.ensure_cursor_visible()?;
            self.refresh_screen()?;
        }

        Ok(())
    }

    fn switch_to_buffer(&mut self, name: &str) -> Result<(), Error> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            println!(":b requires a buffer name");
            return Ok(());
        }

        {
            let store_handle = self.term.store_handle();
            let mut store = store_handle.lock().expect("buffer store lock poisoned");
            store.open(trimmed);
        }

        let previous_mode = self.mode;
        self.open(trimmed.to_string());
        self.mode = previous_mode;
        self.prev_mode = previous_mode;

        if std::env::var("IRIDIUM_SKIP_EDITOR").is_err() {
            self.ensure_cursor_visible()?;
            self.refresh_screen()?;
        }

        Ok(())
    }

    fn close_current_buffer(&mut self, force: bool) -> Result<(), Error> {
        let current_name = self.name.clone();
        let store_handle = self.term.store_handle();
        let mut store = store_handle.lock().expect("buffer store lock poisoned");

        if !force && store.is_dirty(current_name.as_str()) {
            drop(store);
            self.set_status_message(DIRTY_BUFFER_STATUS);
            println!(
                "Buffer '{}' has unsaved changes. Use :q! to close without writing.",
                current_name
            );
            return Ok(());
        }

        let _ = store.mark_closed(current_name.as_str());
        let mut remaining = store.open_buffers();
        drop(store);

        if remaining.is_empty() {
            self.quit = true;
            self.quit_all = true;
            return Ok(());
        }

        remaining.sort();
        let next_name = remaining
            .iter()
            .find(|name| *name > &current_name)
            .cloned()
            .unwrap_or_else(|| remaining[0].clone());

        self.switch_to_buffer(&next_name)?;
        Ok(())
    }

    pub fn execute_colon_command(&mut self, command: &str) -> Result<(), Error> {
        self.process_colon_command(command.trim()).map(|_| ())
    }

    pub fn is_quit(&self) -> bool {
        self.quit
    }

    fn process_colon_command(&mut self, command: &str) -> Result<bool, Error> {
        let mut keep_command_text = false;
        if command.is_empty() {
            self.restore_after_command();
            return Ok(keep_command_text);
        }

        if command == "q" {
            self.close_current_buffer(false)?;
        } else if command == "q!" {
            self.close_current_buffer(true)?;
        } else if command == "i" {
            self.enter_insert_mode();
        } else if command == "r" {
            self.enter_read_mode();
        } else if let Some(rest) = command.strip_prefix('b') {
            self.jump_to_buffer(rest.trim()).ok();
        } else if command == "n" {
            self.cycle_buffer(true)?;
        } else if command == "p" {
            self.cycle_buffer(false)?;
        } else if command == "w" {
            keep_command_text = self.handle_save_command(SaveIntent::BufferOnly)?;
        } else if command == "wq" {
            keep_command_text = self.handle_save_command(SaveIntent::WriteAndQuit)?;
        } else if command == "x" {
            keep_command_text = self.handle_save_command(SaveIntent::ConditionalQuit)?;
        } else if command == "s" {
            self.save_current_buffer_in_memory();
        } else if command == "Q" {
            keep_command_text = self.handle_quit_all_command()?;
        }

        Ok(keep_command_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::buffer_store::BufferStore;
    use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

    fn test_lock() -> MutexGuard<'static, ()> {
        static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
        GUARD
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|err| err.into_inner())
    }

    fn reset_store() -> (Arc<Mutex<BufferStore>>, MutexGuard<'static, ()>) {
        let guard = test_lock();
        unsafe {
            std::env::set_var("IRIDIUM_SKIP_EDITOR", "1");
        }

        let terminal = Terminal::instance();
        let candidate = Arc::new(Mutex::new(BufferStore::new()));
        terminal.attach_store(Arc::clone(&candidate));
        let handle = terminal.store_handle();
        {
            let mut store = handle.lock().unwrap();
            *store = BufferStore::new();
        }

        (handle, guard)
    }

    fn populate_buffer(handle: &Arc<Mutex<BufferStore>>, name: &str, line_count: usize) {
        let mut store = handle.lock().unwrap();
        let buffer = store.open(name);
        buffer.clear();
        for idx in 0..line_count {
            buffer.append(format!("line {idx}"));
        }
    }

    #[test]
    fn navigation_page_up_moves_to_view_top() {
        let (handle, _guard) = reset_store();
        populate_buffer(&handle, "alpha", 20);

        let mut editor = BufferEditor::new("alpha");
        editor.open("alpha");
        editor.mode = EditorMode::Read;
        editor.location = Location { x: 3, y: 10 };
        editor.scroll_offset = 8;
        editor.view_height = 5;

        editor
            .navigate_line(NavigationCommand::PageStart)
            .expect("page up navigation");
        assert_eq!(editor.location.y, 8);
        assert_eq!(editor.scroll_offset, 8);

        editor
            .navigate_line(NavigationCommand::PageStart)
            .expect("page up scrolls");
        assert_eq!(editor.scroll_offset, 6);
        assert_eq!(editor.location.y, 6);
    }

    #[test]
    fn navigation_page_down_moves_to_view_bottom_or_buffer_end() {
        let (handle, _guard) = reset_store();
        populate_buffer(&handle, "alpha", 12);

        let mut editor = BufferEditor::new("alpha");
        editor.open("alpha");
        editor.mode = EditorMode::Read;
        editor.location = Location { x: 2, y: 8 };
        editor.scroll_offset = 7;
        editor.view_height = 6;

        editor
            .navigate_line(NavigationCommand::PageEnd)
            .expect("page down navigation");
        assert_eq!(editor.location.y, 11);
        assert_eq!(editor.scroll_offset, 7);

        editor
            .navigate_line(NavigationCommand::PageEnd)
            .expect("page down scrolls");
        assert_eq!(editor.scroll_offset, 10);
        assert_eq!(editor.location.y, 11);
    }

    #[test]
    fn navigation_page_up_preserves_horizontal_until_front() {
        let (handle, _guard) = reset_store();
        {
            let mut store = handle.lock().unwrap();
            let buffer = store.open("alpha");
            buffer.clear();
            for len in [5usize, 3, 12, 4, 2, 1, 6, 2, 3, 4, 5, 6] {
                buffer.append("x".repeat(len));
            }
        }

        let mut editor = BufferEditor::new("alpha");
        editor.open("alpha");
        editor.mode = EditorMode::Read;
        editor.location = Location { x: 10, y: 10 };
        editor.scroll_offset = 8;
        editor.view_height = 5;

        editor
            .navigate_line(NavigationCommand::PageStart)
            .expect("page up maintains x");
        assert_eq!(editor.location.y, 8);
        assert_eq!(editor.location.x, 10);

        {
            let store = handle.lock().unwrap();
            let buffer = store.get("alpha").unwrap();
            assert!(buffer.lines()[8].chars().count() >= 10);
        }

        // Move to front of buffer and ensure clamped column.
        editor.location = Location { x: 10, y: 0 };
        editor.scroll_offset = 0;
        editor
            .navigate_line(NavigationCommand::PageStart)
            .expect("page up at front");
        assert_eq!(editor.location.y, 0);
        assert_eq!(editor.location.x, 5);
    }

    #[test]
    fn navigation_word_left_moves_to_previous_space() {
        let (handle, _guard) = reset_store();
        {
            let mut store = handle.lock().unwrap();
            let buffer = store.open("alpha");
            buffer.clear();
            buffer.append("first second third".into());
        }

        let mut editor = BufferEditor::new("alpha");
        editor.open("alpha");
        editor.mode = EditorMode::Read;
        editor.location = Location { x: 12, y: 0 };

        editor
            .navigate_line(NavigationCommand::WordLeft)
            .expect("word left");
        assert_eq!(editor.location.x, 11);

        editor
            .navigate_line(NavigationCommand::WordLeft)
            .expect("word left again");
        assert_eq!(editor.location.x, 5);
    }

    #[test]
    fn navigation_word_right_moves_to_next_space_or_end() {
        let (handle, _guard) = reset_store();
        {
            let mut store = handle.lock().unwrap();
            let buffer = store.open("alpha");
            buffer.clear();
            buffer.append("first second third".into());
        }

        let mut editor = BufferEditor::new("alpha");
        editor.open("alpha");
        editor.mode = EditorMode::Read;
        editor.location = Location { x: 0, y: 0 };

        editor
            .navigate_line(NavigationCommand::WordRight)
            .expect("word right");
        assert_eq!(editor.location.x, 5);

        editor
            .navigate_line(NavigationCommand::WordRight)
            .expect("word right again");
        assert_eq!(editor.location.x, 11);
    }

    #[test]
    fn quit_all_prompts_when_buffer_is_untitled() {
        let (handle, _guard) = reset_store();
        {
            let mut store = handle.lock().unwrap();
            store.open_untitled("Untitled-1");
        }

        let mut editor = BufferEditor::new("Untitled-1");
        editor.open("Untitled-1");

        let keep_prompt = editor
            .handle_quit_all_command()
            .expect("quit all command should succeed");
        assert!(keep_prompt, "should keep command text until name provided");

        let input = format!("{}named", BUFFER_NAME_PROMPT);
        editor
            .process_prompt_input(input)
            .expect("prompt processing should succeed");

        assert!(editor.take_quit_all_request());
    }

    #[test]
    fn quit_all_sets_flag_for_named_buffer() {
        let (handle, _guard) = reset_store();
        {
            let mut store = handle.lock().unwrap();
            store.open("alpha");
        }

        let mut editor = BufferEditor::new("alpha");
        editor.open("alpha");

        let keep_prompt = editor
            .handle_quit_all_command()
            .expect("quit all command should succeed");
        assert!(!keep_prompt, "no prompt needed for named buffer");
        assert!(editor.take_quit_all_request());
    }

    #[test]
    fn cycles_forward_and_wraps() {
        let (handle, _guard) = reset_store();
        {
            let mut store = handle.lock().unwrap();
            store.open("beta");
            store.open("alpha");
            store.open("gamma");
        }

        let mut editor = BufferEditor::new("alpha");
        editor.open("alpha");

        editor.cycle_buffer(true).expect("cycle next");
        assert!(editor.prompt_string().contains("[buffer:beta]"));

        editor.cycle_buffer(true).expect("cycle next again");
        assert!(editor.prompt_string().contains("[buffer:gamma]"));

        editor.cycle_buffer(true).expect("cycle wraps to start");
        assert!(editor.prompt_string().contains("[buffer:alpha]"));
    }

    #[test]
    fn cycles_backward_and_wraps() {
        let (handle, _guard) = reset_store();
        {
            let mut store = handle.lock().unwrap();
            store.open("beta");
            store.open("alpha");
            store.open("gamma");
        }

        let mut editor = BufferEditor::new("beta");
        editor.open("beta");

        editor.cycle_buffer(false).expect("cycle prev");
        assert!(editor.prompt_string().contains("[buffer:alpha]"));

        editor.cycle_buffer(false).expect("cycle prev wraps");
        assert!(editor.prompt_string().contains("[buffer:gamma]"));
    }

    #[test]
    fn colon_command_switches_buffer() {
        let (handle, _guard) = reset_store();
        {
            let mut store = handle.lock().unwrap();
            store.open("alpha");
            store.open("beta");
        }

        let mut editor = BufferEditor::new("alpha");
        editor.open("alpha");

        editor
            .apply_input_action(InputAction::ExecuteCommand("b beta".into()))
            .expect("command should succeed");
        assert!(editor.prompt_string().contains("[buffer:beta]"));
    }

    #[test]
    fn close_current_buffer_moves_to_next() {
        let (handle, _guard) = reset_store();
        {
            let mut store = handle.lock().unwrap();
            store.open("alpha");
            store.open("beta");
        }

        let mut editor = BufferEditor::new("alpha");
        editor.open("alpha");

        editor.close_current_buffer(false).expect("close current");

        {
            let store = handle.lock().unwrap();
            let alpha = store.get("alpha").expect("alpha should remain tracked");
            assert!(!alpha.is_open(), "closed buffer should no longer be open");
            let beta = store.get("beta").expect("beta should exist");
            assert!(beta.is_open());
        }

        assert!(editor.prompt_string().contains("[buffer:beta]"));
        assert!(!editor.quit);
    }

    #[test]
    fn close_current_buffer_respects_dirty_flag() {
        let (handle, _guard) = reset_store();
        {
            let mut store = handle.lock().unwrap();
            store.open("alpha").append("dirty".into());
        }

        let mut editor = BufferEditor::new("alpha");
        editor.open("alpha");

        editor.close_current_buffer(false).expect("close current");
        {
            let store = handle.lock().unwrap();
            assert!(store.get("alpha").is_some());
        }
        assert!(!editor.quit);

        editor.close_current_buffer(true).expect("force close");
        {
            let store = handle.lock().unwrap();
            let alpha = store
                .get("alpha")
                .expect("alpha should remain tracked after force close");
            assert!(!alpha.is_open());
        }
        assert!(editor.quit);
    }

    #[test]
    fn dirty_quit_sets_status_message() {
        let (handle, _guard) = reset_store();
        {
            let mut store = handle.lock().unwrap();
            store.open("alpha").append("dirty".into());
        }

        let mut editor = BufferEditor::new("alpha");
        editor.open("alpha");
        editor.execute_colon_command("q").expect(":q should warn");

        assert_eq!(editor.status_message.as_deref(), Some(DIRTY_BUFFER_STATUS));
    }
}
