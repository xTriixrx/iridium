use std::collections::HashMap;
use crate::process::builtin::Builtin;

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

        let parts: Vec<&str> = args[0].split("=").collect();
        self.insert_alias(parts[0], parts[1]);
    
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