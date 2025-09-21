use crate::process::builtin::Builtin;

// type [-aftpP] name [name ...]
// -a print all the places that contain an executable named name
// -t Print a string describing the file type which is of:
//      alias (shell alias)
//      function (shell function)
//      buitin (shell builtin)
//      file (disk file)
//      keyword (shell reserved word)
// -f suppress shell function lookup as with the command builtin
// -p Print the path of the disk file that name would execute as a command.
//      returns nothing if 'type -t name' would not return file.
// -P Forces a PATH search for each name, even if 'type -t name' would not return file.
//      If a command is hashed, -p and -P print the hashed value, not necessarily the file that appears first in PATH.
// With no options, indicate how each name would be interpreted if used as a command name.
/// Stub implementation of the `type` builtin.
pub struct Type {}

impl Builtin for Type {
    /// Currently prints a placeholder message and exits successfully.
    fn call(&mut self, _args: &[String]) -> Option<i32> {
        println!("TYPE!");
        Some(0)
    }
}

impl Type {
    /// Construct a new type builtin instance.
    pub fn new() -> Self {
        Type {}
    }
}
