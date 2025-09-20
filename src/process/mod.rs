pub mod alias;
pub mod builtin;
pub mod cd;
pub mod exit;
pub mod help;
pub mod history;
pub mod pushd;
pub mod pwd;
pub mod r#type;
pub mod welcome;
pub mod which;
use crate::process::builtin::map::BuiltinMap;
use std::process::Command;

pub fn execute(builtin_map: &mut BuiltinMap, args: &Vec<String>) -> Option<i32> {
    if args.len() == 0 {
        return Some(0);
    }

    // Determine if command is builtin, and call function
    if builtin_map.contains(&args[0]) {
        return builtin_map
            .get(&args[0])
            .unwrap()
            .borrow_mut()
            .call(&args[1..]);
    }

    // Attempt to exec external process
    launch(&args)
}

fn launch(args: &Vec<String>) -> Option<i32> {
    let res = Command::new(&args[0]).args(&args[1..]).spawn();

    let mut child = match res {
        Ok(child) => child,
        Err(_e) => {
            eprintln!("iridium: command not found: {}", &args[0]);
            return None;
        }
    };

    let ecode = child
        .wait()
        .expect("Failed to wait on child process, aborting now.");
    Some(
        ecode
            .code()
            .expect("Expected an exit code from spawned child process, aborting now."),
    )
}
