use core::fmt::Display;
use crossterm::style::Print;
use crossterm::{queue, Command};
use std::io::{stdout, Error, Write};
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType};

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
pub struct Terminal {}

impl Terminal {
    pub fn terminate() -> Result<(), Error> {
        Self::execute()?;
        disable_raw_mode()?;
        Ok(())
    }

    pub fn initialize() -> Result<(), Error> {
        enable_raw_mode()?;
        Self::clear_screen()?;
        Self::execute()?;
        Ok(())
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

    pub fn print<T: Display>(string: T) -> Result<(), Error> {
        queue_command(Print(string))?;
        Ok(())
    }

    pub fn size() -> Result<(Size), Error> {
        let (width_u16, height_u16) = size()?;
        let width = width_u16 as usize;
        let height = height_u16 as usize;
        Ok(Size {width, height})
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

