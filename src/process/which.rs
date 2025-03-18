use std::env;
use std::path::{Path, PathBuf};
use crate::process::BUILTIN_NAMES;

pub fn which(args: &[String]) -> Option<i32> {
    // Check if command is a built in command
    if BUILTIN_NAMES.contains(&args[0].as_str()) {
        println!("{}: shell built-in command", args[0]);
        return Some(0);
    }

    // Create path value for prog string that was provided to 'which' and get PATH as string
    let prog = Path::new(&args[0]);
    let path_env = match env::var("PATH") {
        Ok(path_env) => path_env,
        Err(_e) => {
            eprintln!("{} not found", &args[0]);
            return None;
        },
    };

    // Split PATH string on colon to generate iterator
    let paths_str = path_env.split(":");

    // Iterate through each path defined in the PATH variable and add the program into the path
    for path_str in paths_str {
        let path = Path::new(path_str);
        let mut path_buf: PathBuf = path.into();
        path_buf.push(prog);

        // If program file has been found, report path and return
        if path_buf.is_file() {
            println!("{}", path_buf.to_str().unwrap());
            return Some(0);
        }
    }

    // Program was not found, report and return failure
    eprintln!("{} not found", &args[0]);
    return None;
}