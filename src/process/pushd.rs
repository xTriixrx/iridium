use crate::process::builtin::Builtin;

/// Stub implementation of the `pushd` builtin.
pub struct Pushd {}

impl Builtin for Pushd {
    /// Currently prints a placeholder message and exits successfully.
    fn call(&mut self, _args: &[String]) -> Option<i32> {
        println!("PUSHD!");
        Some(0)
    }
}

impl Pushd {
    /// Construct a new pushd builtin instance.
    pub fn new() -> Self {
        Pushd {}
    }
}
