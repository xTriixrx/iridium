use std::env;
use std::path::Path;

pub fn cd(args: &[String]) -> i32 {
    let path = Path::new(&args[0]);
    match env::set_current_dir(&path) {
        Ok(_) => return 0,
        Err(_e) => {
            eprintln!("cd: no such file or directory: {}", &path.to_str().unwrap());
            return 1;
        }
    }
}