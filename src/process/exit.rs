use crate::process::builtin::Builtin;

/// Sentinel exit code used to signal the control loop to terminate.
pub const EXIT_CODE: i32 = 1000;

/// Implements the `exit` builtin, allowing the shell to terminate cleanly.
pub struct Exit {}

impl Builtin for Exit {
    /// Return the sentinel exit code so the caller can break out of the loop.
    fn call(&mut self, _args: &[String]) -> Option<i32> {
        Some(EXIT_CODE)
    }
}

impl Exit {
    /// Construct a new exit builtin instance.
    pub fn new() -> Self {
        Exit {}
    }
}
