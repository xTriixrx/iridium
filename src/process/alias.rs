use std::collections::HashMap;
use crate::process::builtin::Builtin;

// ENV variables used:
// LANG
// LC_ALL
// LC_CTYPE
// LC_MESSAGES
// NLSPATH
// man page: https://www.man7.org/linux/man-pages/man1/alias.1p.html
pub struct Alias {
    alias_map: HashMap<String, String>,
}

impl Builtin for Alias {
    fn call(&mut self, args: &[String]) -> Option<i32> {
        if args.is_empty() {
            for (alias, expansion) in self.alias_map.iter() {
                println!("{}='{}'", alias, expansion);
            }
            return Some(0);
        }

        // Iterate through each alias set and determine if it's a new alias or printing an existing one
        for arg in args {
            // If argument only contains a key, print the existing mapping if it exists.
            if !arg.contains("=") {
                if self.alias_map.contains_key(arg) {
                    let expansion = self.alias_map.get(arg).unwrap();
                    println!("{}={}", arg, expansion);
                }
                continue;
            }

            // If new alias, split on equals and insert alias and it's expansion into map.
            let parts: Vec<&str> = arg.split("=").collect();
            self.insert_alias(parts[0], parts[1]);
        }
    
        return Some(0);
    }
}

impl Alias {
    pub fn new() -> Self {
        Alias {
            alias_map: HashMap::new(),
        }
    }

    fn insert_alias(&mut self, alias_name: &str, expansion: &str) -> Option<String> {
        self.alias_map.insert(alias_name.to_string(), expansion.to_string())
    }

    pub fn contains_alias(&self, alias_name: &str) -> bool {
        self.alias_map.contains_key(alias_name)
    }

    pub fn get_alias_expansion(&self, alias_name: &str) -> Option<&String> {
        self.alias_map.get(alias_name)
    }
}