pub mod cd;
pub mod exit;
pub mod help;
pub mod which;
pub mod history;
pub mod welcome;

use std::process::Command;

const BUILTIN_NAMES: [&str; 6] = [
    "cd",
    "exit",
    "help",
    "history",
    "welcome",
    "which",
];

const BUILTIN_FUNCS: [fn(&[String]) -> i32; 6] = [
    cd::cd,
    exit::exit,
    help::help,
    history::history,
    welcome::welcome,
    which::which,
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
    let res = Command::new(&args[0])
        .args(&args[1..])
        .spawn();
    
    let mut child = match res {
        Ok(child) => child,
        Err(_e) => {
            eprintln!("iridium: command not found: {}", &args[0]);
            return 1;
        }
    };

    let ecode = child.wait().expect("Failed to wait on child process, aborting now.");
    ecode.code().expect("Expected an exit code from spawned child process, aborting now.")
}