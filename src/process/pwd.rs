use super::builtin::Builtin;
use std::env;
use std::path::Path;

/// The 'pwd' built-in command follows the IEEE 1003.1-2017 standard.
///
/// # Name
/// pwd - return working directory name
///
/// # Synopsis
/// pwd [-L|-P]
///
/// # Description
/// The pwd utility shall write to the standard output an absolute pathname of the current working
/// directory, which does not contain the filenames dot or dot-dot.
///
/// # Options
/// The pwd utility shall conform to the XBD Utility Syntax Guidelines.
/// The following options shall be supported by the implementation:
///
/// #### -L
/// If the PWD environment variable contains an absolute pathname of the current directory and the
/// pathname does not contain any components that are dot or dot-dot, pwd shall write this pathname
/// to standard output, except that if the PWD environment variable is longer than {PATH_MAX} bytes
/// including the terminating null, it is unspecified whether pwd writes this pathname to standard
/// output or behaves as if the -P option had been specified. Otherwise, the -L option shall behave
/// as the -P option.
///
/// #### -P
/// The pathname written to standard output shall not contain any components that refer to files of
/// type symbolic link. If there are multiple pathnames that the pwd utility could write to standard
/// output, one beginning with a single <slash> character and one or more beginning with two <slash>
/// characters, then it shall write the pathname beginning with a single <slash> character. The
/// pathname shall not contain any unnecessary <slash> characters after the leading one or two
/// <slash> characters.
///
/// If both -L and -P are specified, the last one shall apply. If neither -L nor -P is specified,
/// the pwd utility shall behave as if -L had been specified.
pub struct Pwd {}

impl Builtin for Pwd {
    fn call(&mut self, args: &[String]) -> Option<i32> {
        let mut options: Vec<&String> = Vec::new();

        // Iterate through all arguments and categorize references into options and arguments
        for arg in args {
            // If argument is provided that isn't an option abort
            if !arg.starts_with("-") {
                eprintln!("pwd: too many arguments");
                return None;
            }

            // If an option is provided that is not -L or -P abort
            if arg.starts_with("-") && arg != "-L" && arg != "-P" {
                eprintln!("pwd: bad option: {}", arg);
                return None;
            }

            options.push(arg);
        }

        if options.iter().any(|&option| option == "-P") {
            let pwd_val = self.get_pwd();
            let path = Path::new(&pwd_val);
            let pwd = match path.canonicalize() {
                Ok(pwd) => pwd,
                Err(e) => panic!("Error canonicalizing path: {}, {}", pwd_val, e),
            };

            println!("{}", pwd.to_str().unwrap());
            return Some(0);
        }

        let pwd = self.get_pwd();
        println!("{}", pwd);
        Some(0)
    }
}

impl Pwd {
    pub fn new() -> Self {
        Pwd {}
    }

    pub fn get_pwd(&self) -> String {
        get_pwd()
    }
}

fn get_pwd() -> String {
    match env::var("PWD") {
        Ok(pwd) => pwd,
        Err(_e) => String::from(""),
    }
}
