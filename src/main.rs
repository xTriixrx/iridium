mod complete;
mod config;
mod control;
mod control_state;
mod editor;
mod process;
mod store;

use rustyline::Result;

use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use std::io::{self, Read};

use crate::editor::buffer_editor::BufferEditor;

/// Entry point that prints the welcome banner and starts the control loop.
fn main() -> Result<()> {
    // let mut editor = BufferEditor::new("test");
    // editor.run();
    // Ok(())

    let empty_slice: [String; 0] = [];

    // Perform welcome message
    // process::welcome::welcome(&empty_slice);

    control::control_loop()
}
