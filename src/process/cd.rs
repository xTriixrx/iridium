use std::env;
use std::path::Path;

pub fn cd(args: &[String]) -> Option<i32> {
    let path = Path::new(&args[0]);
    match env::set_current_dir(&path) {
        Ok(_) => return Some(0),
        Err(_e) => {
            eprintln!("cd: no such file or directory: {}", &path.to_str().unwrap());
            return None;
        }
    }
}