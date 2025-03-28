use std::fs;
use std::env;
use std::rc::Rc;
use std::cell::RefCell;
use crate::process::pwd::Pwd;
use crate::process::builtin::Builtin;

/// The cd built-in command follows the IEEE 1003.1-2017 standard.
/// 
/// # Name
/// cd - change the working directory
/// 
/// # Synopsis
/// cd [-L|-P] [directory]
/// cd -
pub struct Cd {
    pwd: Option<Rc<RefCell<Pwd>>>,
}

///
/// 
impl Builtin for Cd {
    ///
    /// 
    fn call(&mut self, args: &[String]) -> Option<i32> {
        // Get HOME env variable
        let home = match env::var("HOME") {
            Ok(env) => env,
            Err(_e) => String::from(""),
        };

        // If no arguments have been provided and HOME env is empty or undefined, return with 0
        if args.len() == 0 && home.is_empty() {
            return Some(0);
        }

        // If no arguments have been provided and HOME env is set, cd to HOME and return with 0
        if args.len() == 0 && !home.is_empty() {
            return set_dir(&home);
        }

        if args[0] == "-" {
            let pwd_cmd = match self.pwd.as_ref() {
                Some(pwd) => pwd.borrow(),
                None => panic!("Pwd is none!"),
            };

            let oldpwd = match env::var("OLDPWD") {
                Ok(oldpwd) => oldpwd,
                Err(_e) => String::from(""),
            };

            let ret = set_dir(&oldpwd);
            println!("{}\n", pwd_cmd.get_pwd());
            return ret;
        }

        set_dir(&args[0])
    }
}

///
/// 
impl Cd {
    pub fn new() -> Self {
        Cd {
            pwd: None,
        }
    }

    pub fn set_pwd(&mut self, pwd: Rc<RefCell<Pwd>>) {
        self.pwd = Some(pwd);
    }
}

///
/// 
pub fn set_dir(path: &String) -> Option<i32> {
    // Get current path before changing directories
    let cur_path = match env::current_dir() {
        Ok(cur) => cur,
        Err(e) => panic!("Unknown current path: {}", e),
    };

    // Get full canonical path of new directory
    let canonical_path = match fs::canonicalize(path) {
        Ok(canonical) => canonical,
        Err(e) => panic!("Unknown canonical path: {}, {}", path, e),
    };

    // Change current directory using canonical path
    match env::set_current_dir(&canonical_path) {
        Ok(_) => return {
            unsafe {
                env::set_var("OLDPWD", cur_path);
                env::set_var("PWD", canonical_path);
            }

            Some(0)
        },
        Err(_e) => {
            eprintln!("cd: no such file or directory: {}", &path.as_str());
            return None;
        }
    }
}