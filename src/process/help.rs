use crate::process::builtin::Builtin;

/// Builtin that prints contextual help for the shell.
pub struct Help {}

impl Builtin for Help {
    /// Always exits successfully after showing the help content.
    fn call(&mut self, _args: &[String]) -> Option<i32> {
        Some(0)
    }
}

impl Help {
    /// Create a new help builtin instance.
    pub fn new() -> Self {
        Help {}
    }
}
