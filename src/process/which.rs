use std::env;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use crate::process::alias::Alias;
use crate::process::builtin::Builtin;

/// Implementation of the `which` builtin that searches aliases, builtins, and the PATH.
pub struct Which {
    aliases: Option<Rc<RefCell<Alias>>>,
    builtin_names: HashSet<String>,
}

impl Builtin for Which {
    /// Resolve a command name to an alias, builtin, or filesystem path.
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
        if self.builtin_names.contains(&args[0]) {
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
            }
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
    /// Construct a `which` builtin that can later be wired with dependencies.
    pub fn new() -> Self {
        Self {
            aliases: None,
            builtin_names: HashSet::new(),
        }
    }

    /// Inject the alias table so `which` can inspect defined aliases.
    pub fn set_aliases(&mut self, aliases: Rc<RefCell<Alias>>) {
        self.aliases = Some(aliases);
    }

    /// Provide the set of builtin names so they can be reported to the user.
    pub fn set_builtin_names(&mut self, names: impl IntoIterator<Item = String>) {
        self.builtin_names = names.into_iter().collect();
    }
}
