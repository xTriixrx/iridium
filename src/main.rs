mod cmd;
mod complete;
mod config;
mod control;
mod control_state;
mod editor;
mod process;
mod store;

use rustyline::Result;

/// Entry point that starts the control loop.
fn main() -> Result<()> {
    control::control_loop()
}
