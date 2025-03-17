pub mod exit;
pub mod help;
pub mod history;
pub mod welcome;

use std::process::Command;

const BUILTIN_NAMES: [&str; 4] = [
    "exit",
    "help",
    "history",
    "welcome",
];

const BUILTIN_FUNCS: [fn(&[String]) -> i32; 4] = [
    exit::exit,
    help::help,
    history::history,
    welcome::welcome,
];

pub fn execute(args: &Vec<String>) -> i32 {
    if args.len() == 0 {
        return 0;
    }

    // Determine if built in function was executed
    for i in 0..BUILTIN_NAMES.len() {
        if args[0] == BUILTIN_NAMES[i] {
            return BUILTIN_FUNCS[i](&args[1..])
        }
    }

    // Attempt to exec external process
    launch(&args)
}

fn launch(args: &Vec<String>) -> i32 {
    let mut child = Command::new(&args[0])
        .args(&args[1..])
        .spawn()
        .expect("Failed to execute child process, aborting now.");

    let ecode = child.wait().expect("Failed to wait on child process, aborting now.");
    ecode.code().expect("Expected an exit code from spawned child process, aborting now.")
}