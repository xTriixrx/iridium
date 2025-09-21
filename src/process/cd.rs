use crate::process::builtin::Builtin;
use crate::process::pwd::Pwd;
use normalize_path::NormalizePath;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

/// The cd built-in command follows the IEEE 1003.1-2017 standard.
///
/// # Name
/// cd - change the working directory
///
/// # Synopsis
/// cd [-L|-P] [directory]
/// cd -
///
/// Implements the POSIX `cd` builtin and tracks the `pwd` dependency when provided.
pub struct Cd {
    pwd: Option<Rc<RefCell<Pwd>>>,
}

impl Builtin for Cd {
    /// Resolve command arguments and update process working directory state.
    fn call(&mut self, args: &[String]) -> Option<i32> {
        // Get HOME env variable
        let mut home = match env::var("HOME") {
            Ok(env) => env,
            Err(_e) => String::from(""),
        };

        // If no arguments have been provided and HOME env is empty or undefined, return with 0
        if args.len() == 0 && home.is_empty() {
            return Some(0);
        }

        // If no arguments have been provided and HOME env is set, cd to HOME and return with 0
        if args.len() == 0 && !home.is_empty() {
            return set_dir(&mut home);
        }

        if args[0] == "-" {
            let pwd_cmd = match self.pwd.as_ref() {
                Some(pwd) => pwd.borrow(),
                None => panic!("Pwd is none!"),
            };

            let mut oldpwd = match env::var("OLDPWD") {
                Ok(oldpwd) => oldpwd,
                Err(_e) => String::from(""),
            };

            let ret = set_dir(&mut oldpwd);
            println!("{}\n", pwd_cmd.get_pwd());
            return ret;
        }

        let mut path = args[0].clone();
        set_dir(&mut path)
    }
}

impl Cd {
    /// Construct a new `cd` builtin with no pwd dependency wired yet.
    pub fn new() -> Self {
        Cd { pwd: None }
    }

    /// Provide the shared `pwd` builtin so `cd` can print the previous directory.
    pub fn set_pwd(&mut self, pwd: Rc<RefCell<Pwd>>) {
        self.pwd = Some(pwd);
    }
}

/// Resolve the requested target directory and update environment state.
fn set_dir(path: &mut String) -> Option<i32> {
    // Clone provided path before attempting to replace '~'
    let mut modpath = path.clone();

    // If provided path contains "~", replace with home value.
    if path.contains("~") {
        modpath = path.replace(
            "~",
            &env::var("HOME").expect("Expected HOME environment variable to be set, aborting now."),
        );
    }

    // Create new path based off cwd and path provided
    let mut new_path = match env::current_dir() {
        Ok(cur_dir) => cur_dir.join(modpath),
        Err(e) => panic!("Unknown current path: '{}', {}", path, e),
    };

    // If path is a symbolic link, normalize path
    if new_path.is_symlink() {
        new_path.normalize();
    } else {
        // Otherwise, get full canonical path of new directory
        new_path = match fs::canonicalize(new_path) {
            Ok(canonical) => canonical,
            Err(_e) => {
                eprintln!("cd: no such file or directory: {}", path);
                return Some(1);
            }
        };
    }
    //cd: no such file or directory: blah
    // Change current directory using canonical path
    update_path(new_path)
}

/// Apply the directory change and make sure `PWD` and `OLDPWD` mirror the change.
fn update_path(new_path: PathBuf) -> Option<i32> {
    // Get current path before changing directories
    let cur_path = match env::current_dir() {
        Ok(cur) => cur,
        Err(e) => panic!("Unknown current path: {}", e),
    };

    // Set new path and update OLDPWD & PWD env vars
    match env::set_current_dir(&new_path) {
        Ok(_) => {
            return {
                unsafe {
                    env::set_var("OLDPWD", cur_path);
                    env::set_var("PWD", new_path);
                }

                Some(0)
            };
        }
        Err(_e) => {
            eprintln!(
                "cd: no such file or directory: {}",
                &new_path.to_str().unwrap()
            );
            return None;
        }
    }
}
