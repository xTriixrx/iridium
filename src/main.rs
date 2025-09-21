mod config;
mod control;
mod process;
mod complete;
mod control_state;

use rustyline::Result;

/// Entry point that prints the welcome banner and starts the control loop.
fn main() -> Result<()> {
    let empty_slice: [String; 0] = [];

    // Perform welcome message
    process::welcome::welcome(&empty_slice);

    control::control_loop()
}
