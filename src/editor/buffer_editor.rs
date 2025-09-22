use core::cmp::min;
use std::io::Error;
use crate::store::buffer::BufferStore;
use crate::editor::terminal::{Terminal, Size, Position};
use crossterm::event::{read, Event, Event::Key, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

const NAME: &str = "IBE";
const RETURN: &str = "\r\n";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Default)]
pub struct BufferEditor {
    quit: bool,
    name: String,
    mode: EditorMode,
    location: Location,
    buffer_store: BufferStore,
}

#[derive(Debug, Copy, Clone, Default)]
struct Location {
    x: usize,
    y: usize,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum EditorMode {
    #[default]
    Insert,
    Command,
}

#[derive(Debug, Clone)]
pub enum EditorAction {
    Append(String),
    Clear,
    DeleteLast,
    Show,
    ListBuffers,
    Quit { write: bool },
    SwitchMode(EditorMode),
    UnknownCommand(String),
    NoOp,
}

impl BufferEditor {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            quit: false,
            name: name.into(),
            mode: EditorMode::default(),
            location: Location::default(),
            buffer_store: BufferStore::default(),
        }
    }

    pub fn run(&mut self) {
        Terminal::initialize().unwrap();
        let result = self.repl();
        Terminal::terminate().unwrap();
        result.unwrap();
    }

    fn repl(&mut self) -> Result<(), Error> {
        loop {
            self.refresh_screen()?;

            if self.quit {
                break;
            }

            let event = read()?;
            self.evaluate_event(&event);
        }
        
        Ok(())
    }

    fn move_point(&mut self, key_code: KeyCode) -> Result<(), Error> {
        let Location { mut x, mut y} = self.location;
        let Size { width, height} = Terminal::size()?;
        match key_code {
            KeyCode::Up => {
                y = y.saturating_sub(1);
            },
            KeyCode::Down => {
                y = min(height.saturating_sub(1), y.saturating_add(1));
            },
            KeyCode::Left => {
                x = x.saturating_sub(1);
            },
            KeyCode::Right => {
                x = min(width.saturating_sub(1), x.saturating_add(1))
            },
            KeyCode::PageUp => {
                y = 0;
            }
            KeyCode::PageDown => {
                y = height.saturating_sub(1);
            }
            KeyCode::Home => {
                x = 0;
            }
            KeyCode::End => {
                x = width.saturating_sub(1)
            }
            _ => (),
        }

        self.location = Location { x, y };
        Ok(())
    }

    fn evaluate_event(&mut self, event: &Event) -> Result<(), Error> {
        if let Key(KeyEvent {
            code, modifiers, kind: KeyEventKind::Press, state
        }) = event
        {
            match code {
                KeyCode::Char('c') if *modifiers == KeyModifiers::CONTROL => {
                    self.quit = true;
                },
                KeyCode::Up
                | KeyCode::Down
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::Home
                | KeyCode::End => {
                    self.move_point(*code)?;
                }
                _ => (),
            }
        }

        Ok(())
    }

    fn refresh_screen(&self) -> Result<(), Error> {
        Terminal::hide_caret()?;
        Terminal::move_caret_to(Position::default())?;

        if self.quit {
            Terminal::clear_screen()?;
            let _ = Terminal::print("Closed editor.\r\n");
        }
        else {
            Self::draw_rows()?;
            let _ = Terminal::move_caret_to(Position {
                col: self.location.x,
                row: self.location.y,
            })?;
        }
        Terminal::show_caret()?;
        Terminal::execute()?;
        Ok(())
    }

    fn draw_banner() -> Result<(), Error> {
        let purple_text = "\u{1b}[35m";
        let end_color_text = "\u{1b}[39m";

        let iridium_msg = [
            "   ██▓ ██▀███   ██▓▓█████▄  ██▓ █    ██  ███▄ ▄███▓",
            "  ▓██▒▓██ ▒ ██▒▓██▒▒██▀ ██▌▓██▒ ██  ▓██▒▓██▒▀█▀ ██▒",
            "  ▒██▒▓██ ░▄█ ▒▒██▒░██   █▌▒██▒▓██  ▒██░▓██    ▓██░",
            "  ░██░▒██▀▀█▄  ░██░░▓█▄   ▌░██░▓▓█  ░██░▒██    ▒██ ",
            "  ░██░░██▓ ▒██▒░██░░▒████▓ ░██░▒▒█████▓ ▒██▒   ░██▒",
            "  ░▓  ░ ▒▓ ░▒▓░░▓   ▒▒▓  ▒ ░▓  ░▒▓▒ ▒ ▒ ░ ▒░   ░  ░",
            "   ▒ ░  ░▒ ░ ▒░ ▒ ░ ░ ▒  ▒  ▒ ░░░▒░ ░ ░ ░  ░      ░",
            "   ▒ ░  ░░   ░  ▒ ░ ░ ░  ░  ▒ ░ ░░░ ░ ░ ░      ░   ",
            "  ░     ░      ░     ░     ░     ░            ░    ",
            "                    ░                              ",
        ];

        let buffer_msg = [
            "▄▄▄▄    █    ██   █████▒ █████▒▓█████  ██▀███",
            "▓█████▄  ██  ▓██▒▓██   ▒▓██   ▒ ▓█   ▀ ▓██ ▒ ██▒",
            "▒██▒ ▄██▓██  ▒██░▒████ ░▒████ ░ ▒███   ▓██ ░▄█ ▒",
            "▒██░█▀  ▓▓█  ░██░░▓█▒  ░░▓█▒  ░ ▒▓█  ▄ ▒██▀▀█▄  ",
            "░▓█  ▀█▓▒▒█████▓ ░▒█░   ░▒█░    ░▒████▒░██▓ ▒██▒",
            "░▒▓███▀▒░▒▓▒ ▒ ▒  ▒ ░    ▒ ░    ░░ ▒░ ░░ ▒▓ ░▒▓░",
            "▒░▒   ░ ░░▒░ ░ ░  ░      ░       ░ ░  ░  ░▒ ░ ▒░",
            " ░    ░  ░░░ ░ ░  ░ ░    ░ ░       ░     ░░   ░ ",
            " ░         ░                       ░  ░   ░     ",
            "      ░                                         ",
        ];

        let editor_msg = [
            "▓█████ ▓█████▄  ██▓▄▄▄█████▓ ▒█████   ██▀███  ",
            "▓█   ▀ ▒██▀ ██▌▓██▒▓  ██▒ ▓▒▒██▒  ██▒▓██ ▒ ██▒",
            "▒███   ░██   █▌▒██▒▒ ▓██░ ▒░▒██░  ██▒▓██ ░▄█ ▒",
            "▒▓█  ▄ ░▓█▄   ▌░██░░ ▓██▓ ░ ▒██   ██░▒██▀▀█▄  ",
            "░▒████▒░▒████▓ ░██░  ▒██▒ ░ ░ ████▓▒░░██▓ ▒██▒",
            "░░ ▒░ ░ ▒▒▓  ▒ ░▓    ▒ ░░   ░ ▒░▒░▒░ ░ ▒▓ ░▒▓░",
            " ░ ░  ░ ░ ▒  ▒  ▒ ░    ░      ░ ▒ ▒░   ░▒ ░ ▒░",
            "   ░    ░ ░  ░  ▒ ░  ░      ░ ░ ░ ▒    ░░   ░ ",
            "   ░  ░   ░     ░               ░ ░     ░     ",
            "        ░                                     ",
        ];

        let width = Terminal::size()?.width;
        let mut len = iridium_msg.len();
        let mut padding = (width.saturating_sub(len)) / 2;
        let mut spaces = " ".repeat(padding.saturating_sub(1));

        for line in iridium_msg {
            let formatted_line = format!("~{spaces}{purple_text}{line}{end_color_text}{RETURN}");
            let _ = Terminal::print(formatted_line);
        }
        let _ = Terminal::print(format!("~{RETURN}"));
        
        len = buffer_msg.len();
        padding = (width.saturating_sub(len)) / 2;
        spaces = " ".repeat(padding.saturating_sub(1));

        for line in buffer_msg {
            let formatted_line = format!("~{spaces}{purple_text}{line}{end_color_text}{RETURN}");
            let _ = Terminal::print(formatted_line);
        }
        let _ = Terminal::print(format!("~{RETURN}"));

        len = editor_msg.len();
        padding = (width.saturating_sub(len)) / 2;
        spaces = " ".repeat(padding.saturating_sub(1));

        for line in editor_msg {
            let formatted_line = format!("~{spaces}{purple_text}{line}{end_color_text}{RETURN}");
            let _ = Terminal::print(formatted_line);
        }
        let _ = Terminal::print(format!("~{RETURN}"));

        let mut welcome_msg = format!("{NAME} editor -- version {VERSION}");
        len = welcome_msg.len();
        padding = (width.saturating_sub(len)) / 2;
        spaces = " ".repeat(padding.saturating_sub(1));
        welcome_msg = format!("~{spaces}{welcome_msg}");
        welcome_msg.truncate(width);
        Terminal::print(welcome_msg)?;
        Ok(())
    }

    fn draw_empty_row() -> Result<(), Error> {
        let _ = Terminal::print("~");
        Ok(())
    }

    fn draw_rows() -> Result<(), std::io::Error> {
        let Size {width, height} = Terminal::size()?;
        for current_row in 0..height {
            Terminal::clear_line()?;
            if current_row == height / 3 {
                Self::draw_banner()?;
            }
            else {
                Self::draw_empty_row()?;
            }

            if current_row.saturating_add(1) < height {
                Terminal::print(RETURN)?;
            }
        }
        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn prompt(&self) -> String {
        match self.mode {
            EditorMode::Command => format!("[buffer:{}] ", self.name),
            EditorMode::Insert => format!("[buffer:{}] -- INSERT -- ", self.name),
        }
    }

    pub fn handle_input(&mut self, line: &str) -> EditorAction {
        let trimmed = line.trim_end();
        if let Some(cmd) = trimmed.strip_prefix(':') {
            return self.handle_colon_command(cmd.trim());
        }

        match self.mode {
            EditorMode::Command => match trimmed {
                "" => EditorAction::NoOp,
                "i" | "a" => self.switch_mode(EditorMode::Insert),
                "dd" => EditorAction::DeleteLast,
                other => EditorAction::UnknownCommand(other.to_string()),
            },
            EditorMode::Insert => EditorAction::Append(line.to_string()),
        }
    }

    fn handle_colon_command(&mut self, cmd: &str) -> EditorAction {
        match cmd {
            "" => EditorAction::NoOp,
            "w" | "show" => EditorAction::Show,
            "clear" => EditorAction::Clear,
            "ls" => EditorAction::ListBuffers,
            "q" => EditorAction::Quit { write: false },
            "wq" => EditorAction::Quit { write: true },
            "i" => self.switch_mode(EditorMode::Insert),
            "esc" => self.switch_mode(EditorMode::Command),
            other => EditorAction::UnknownCommand(other.to_string()),
        }
    }

    fn switch_mode(&mut self, mode: EditorMode) -> EditorAction {
        if self.mode != mode {
            self.mode = mode;
            EditorAction::SwitchMode(mode)
        } else {
            EditorAction::NoOp
        }
    }
}


// pub struct BufferEditor {

// }

// impl BufferEditor {
//     pub fn new() -> Self {
//         Self {

//         }
//     }

// }
