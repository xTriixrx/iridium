pub mod map;

// Would like to move this to the heap but keep static for commands
// that need to reference built in commands..
pub const BUILTIN_NAMES: [&str; 10] = [
    "alias", "cd", "exit", "help", "history", "pushd", "pwd", "type", "welcome", "which",
];

pub trait Builtin {
    fn call(&mut self, args: &[String]) -> Option<i32>;
}
