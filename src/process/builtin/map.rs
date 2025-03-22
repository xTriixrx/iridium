use std::rc::Rc;
use super::Builtin;
use std::cell::RefCell;
use crate::process::cd::Cd;
use std::collections::HashMap;
use crate::process::exit::Exit;
use crate::process::help::Help;
use crate::process::alias::Alias;
use crate::process::which::Which;
use crate::process::r#type::Type;
use crate::process::history::History;
use crate::process::welcome::Welcome;

pub struct BuiltinMap {
    alias: Rc<RefCell<Alias>>,
    cd: Rc<RefCell<Cd>>,
    exit: Rc<RefCell<Exit>>,
    help: Rc<RefCell<Help>>,
    history: Rc<RefCell<History>>,
    r#type: Rc<RefCell<Type>>,
    welcome: Rc<RefCell<Welcome>>,
    which: Rc<RefCell<Which>>,
    func_map: HashMap<String, Rc<RefCell<dyn Builtin>>>,
}

impl BuiltinMap {
    pub fn new() -> Self {
        let builtin = BuiltinMap {
            alias: Rc::new(RefCell::new(Alias::new())),
            cd: Rc::new(RefCell::new(Cd::new())),
            exit: Rc::new(RefCell::new(Exit::new())),
            help: Rc::new(RefCell::new(Help::new())),
            history: Rc::new(RefCell::new(History::new())),
            r#type: Rc::new(RefCell::new(Type::new())),
            welcome: Rc::new(RefCell::new(Welcome::new())),
            which: Rc::new(RefCell::new(Which::new())),
            func_map: HashMap::new(),
        };

        builtin.which.borrow_mut().set_aliases(builtin.alias.clone());
        return builtin;
    }

    pub fn populate_func_map(&mut self) {
        // If builtin map is not empty abort inital population
        if !self.is_empty() {
            return;
        }
        
        self.add("alias", self.alias.clone());
        self.add("cd", self.cd.clone());
        self.add("exit", self.exit.clone());
        self.add("help", self.help.clone());
        self.add("history", self.history.clone());
        self.add("type", self.r#type.clone());
        self.add("welcome", self.welcome.clone());
        self.add("which", self.which.clone());
    }

    pub fn get_alias(&self) -> Rc<RefCell<Alias>> {
        self.alias.clone()
    }

    pub fn get(&mut self, func_name: &str) -> Option<&mut Rc<RefCell<dyn Builtin>>> {
        self.func_map.get_mut(func_name)
    }

    pub fn add(&mut self, func_name: &str, func_ptr: Rc<RefCell<dyn Builtin>>) {
        self.func_map.insert(func_name.to_string(), func_ptr);
    }

    pub fn contains(&self, func_name: &str) -> bool {
        self.func_map.contains_key(func_name)
    }

    pub fn is_empty(&self) -> bool {
        self.func_map.is_empty()
    }
}