use std::env;
use std::rc::Rc;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use crate::process::alias::Alias;
use crate::process::builtin::Builtin;
use crate::process::builtin::BUILTIN_NAMES;

pub struct Which {
    aliases: Option<Rc<RefCell<Alias>>>,
}

impl Builtin for Which {
    fn call(&mut self, args: &[String]) -> Option<i32> {
        let aliases = match self.aliases.as_ref() {
            Some(aliases) => aliases.borrow(),
            None => panic!("Aliases is none!"),
        };

        // Check if command is an alias
        if aliases.contains_alias(&args[0]) {
            let expansion = aliases.get_alias_expansion(&args[0]).unwrap();
            println!("{}: aliased to {}", args[0], expansion);
            return Some(0);
        }

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
}

impl Which {
    pub fn new() -> Self {
        Which {
            aliases: None,
        }
    }

    pub fn set_aliases(&mut self, aliases: Rc<RefCell<Alias>>) {
        self.aliases = Some(aliases);
    }
}