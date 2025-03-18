pub mod cd;
pub mod exit;
pub mod help;
pub mod which;
pub mod history;
pub mod welcome;

use std::process::Command;
use std::collections::HashMap;
type BuiltInFuncIntf = fn(&[String]) -> Option<i32>;

// Would like to move this to the heap but keep static for commands
// that need to reference built in commands..
const BUILTIN_NAMES: [&str; 6] = [
    "cd",
    "exit",
    "help",
    "history",
    "welcome",
    "which",
];

pub struct BuiltInMap {
    func_map: HashMap<String, BuiltInFuncIntf>,
}

impl BuiltInMap {
    pub fn new() -> Self {
        BuiltInMap {
            func_map: HashMap::new(),
        }
    }

    pub fn call(&self, func_name: &str, args: &[String]) -> Option<i32> {
        self.func_map[func_name](args)
    }

    pub fn add(&mut self, func_name: &str, func_ptr: BuiltInFuncIntf) {
        self.func_map.insert(func_name.to_string(), func_ptr);
    }

    pub fn contains(&self, func_name: &str) -> bool {
        self.func_map.contains_key(func_name)
    }

    pub fn is_empty(&self) -> bool {
        self.func_map.is_empty()
    }
}

pub fn populate_func_map(builtin_map: &mut BuiltInMap) {
    // If builtin map is not empty abort inital population
    if !builtin_map.is_empty() {
        return;
    }

    builtin_map.add("cd", cd::cd);
    builtin_map.add("exit", exit::exit);
    builtin_map.add("help", help::help);
    builtin_map.add("history", history::history);
    builtin_map.add("welcome", welcome::welcome);
    builtin_map.add("which", which::which);
}

pub fn execute(builtin_map: &BuiltInMap, args: &Vec<String>) -> Option<i32> {
    if args.len() == 0 {
        return Some(0);
    }

    // Determine if command is builtin, and call function
    if builtin_map.contains(&args[0]) {
        return builtin_map.call(&args[0], &args[1..]);
    }

    // Attempt to exec external process
    launch(&args)
}

fn launch(args: &Vec<String>) -> Option<i32> {
    let res = Command::new(&args[0])
        .args(&args[1..])
        .spawn();
    
    let mut child = match res {
        Ok(child) => child,
        Err(_e) => {
            eprintln!("iridium: command not found: {}", &args[0]);
            return None;
        }
    };

    let ecode = child.wait().expect("Failed to wait on child process, aborting now.");
    Some(ecode.code().expect("Expected an exit code from spawned child process, aborting now."))
}