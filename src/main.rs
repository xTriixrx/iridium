mod config;
mod control;
mod process;
use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let empty_slice: [String; 0] = [];

    // Load config files, if any.
    process::welcome::welcome(&empty_slice);

    // Run command loop.
    control::control_loop()?;

    return Ok(())
}

