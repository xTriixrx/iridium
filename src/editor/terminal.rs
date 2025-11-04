use crate::store::buffer::BufferStore;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode, size};
use crossterm::{Command, queue};
use std::io::{Error, Write, stdout};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Copy, Clone)]
pub struct Size {
    pub width: usize,
    pub height: usize,
}

#[derive(Copy, Clone, Default)]
pub struct Position {
    pub col: usize,
    pub row: usize,
}

#[derive(Debug, Default)]
pub struct Terminal {
    store: OnceLock<Arc<Mutex<BufferStore>>>,
}

impl Terminal {
    fn new() -> Self {
        Self {
            store: OnceLock::new(),
        }
    }

    pub fn instance() -> &'static Self {
        static INSTANCE: OnceLock<Terminal> = OnceLock::new();

        INSTANCE.get_or_init(|| {
            Terminal::initialize().expect("Terminal initialization failed. Aborting.")
        })
    }

    pub fn attach_store(&'static self, store: Arc<Mutex<BufferStore>>) {
        let _ = self.store.set(store);
    }

    pub fn store_handle(&self) -> Arc<Mutex<BufferStore>> {
        self.store
            .get()
            .cloned()
            .expect("Buffer store has not been attached to the terminal")
    }

    pub fn enter(&self) -> Result<(), Error> {
        enable_raw_mode()?;
        Self::clear_screen()?;
        Self::execute()?;
        Ok(())
    }

    fn save_buffers(&self) -> Result<(), Error> {
        let store = self.store_handle();
        let store = store.lock().expect("buffer store lock poisoned");
        store.save_all()?;
        Ok(())
    }

    pub fn insert_char(
        &self,
        buffer_name: &str,
        position: Position,
        ch: char,
    ) -> Result<Position, Error> {
        {
            let store = self.store_handle();
            let mut store = store.lock().expect("buffer store lock poisoned");
            store.insert_char(buffer_name, position.row, position.col, ch);
        }

        Self::move_caret_to(position)?;
        Self::print(&ch.to_string())?;

        let Size { width, height: _ } = Self::size()?;
        let mut next = Position {
            col: position.col.saturating_add(1),
            row: position.row,
        };

        if next.col >= width {
            next.col = 0;
            next.row = position.row.saturating_add(1);
        }

        Self::move_caret_to(next)?;
        Self::execute()?;

        Ok(next)
    }

    pub fn insert_newline(&self, buffer_name: &str, position: Position) -> Result<Position, Error> {
        let (row, col) = {
            let store = self.store_handle();
            let mut store = store.lock().expect("buffer store lock poisoned");
            store.insert_newline(buffer_name, position.row, position.col)
        };

        Self::move_caret_to(position)?;
        Self::print("\r\n")?;

        let next = Position { col, row };
        Self::move_caret_to(next)?;
        Self::execute()?;

        Ok(next)
    }

    pub fn delete_char(
        &self,
        buffer_name: &str,
        position: Position,
    ) -> Result<Option<Position>, Error> {
        if position.col == 0 {
            return Ok(None);
        }

        let new_coordinates = {
            let store = self.store_handle();
            let mut store = store.lock().expect("buffer store lock poisoned");
            store.delete_char(buffer_name, position.row, position.col)
        };

        if let Some((row, col)) = new_coordinates {
            Ok(Some(Position { col, row }))
        } else {
            Ok(None)
        }
    }

    pub fn terminate() -> Result<(), Error> {
        let terminal = Self::instance();
        terminal.save_buffers()?;
        Self::execute()?;
        disable_raw_mode()?;
        Ok(())
    }

    fn initialize() -> Result<Terminal, Error> {
        let term = Terminal::new();
        enable_raw_mode()?;
        Self::clear_screen()?;
        Self::execute()?;
        Ok(term)
    }

    pub fn clear_screen() -> Result<(), Error> {
        queue_command(Clear(ClearType::All))?;
        Ok(())
    }

    pub fn clear_line() -> Result<(), Error> {
        queue_command(Clear(ClearType::CurrentLine))?;
        Ok(())
    }

    pub fn move_caret_to(position: Position) -> Result<(), Error> {
        queue_command(MoveTo(position.col as u16, position.row as u16))?;
        Ok(())
    }

    pub fn hide_caret() -> Result<(), Error> {
        queue_command(Hide)?;
        Ok(())
    }

    pub fn show_caret() -> Result<(), Error> {
        queue_command(Show)?;
        Ok(())
    }

    pub fn print(string: &str) -> Result<(), Error> {
        queue_command(Print(string))?;
        Ok(())
    }

    pub fn size() -> Result<Size, Error> {
        let (width_u16, height_u16) = size()?;
        let width = width_u16 as usize;
        let height = height_u16 as usize;
        Ok(Size { width, height })
    }

    pub fn execute() -> Result<(), Error> {
        stdout().flush()?;
        Ok(())
    }
}

fn queue_command<T: Command>(command: T) -> Result<(), Error> {
    queue!(stdout(), command)?;
    Ok(())
}
