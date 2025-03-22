pub mod cd;
pub mod exit;
pub mod help;
pub mod alias;
pub mod which;
pub mod r#type;
pub mod history;
pub mod welcome;
pub mod builtin;
use std::process::Command;
use crate::process::builtin::map::BuiltinMap;

pub fn execute(builtin_map: &mut BuiltinMap, args: &Vec<String>) -> Option<i32> {
    if args.len() == 0 {
        return Some(0);
    }

    // Determine if command is builtin, and call function
    if builtin_map.contains(&args[0]) {
        return builtin_map.get(&args[0]).unwrap().borrow_mut().call(&args[1..]);
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