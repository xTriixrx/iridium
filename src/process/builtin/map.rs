use super::Builtin;
use crate::process::alias::Alias;
use crate::process::cd::Cd;
use crate::process::exit::Exit;
use crate::process::help::Help;
use crate::process::history::History;
use crate::process::pushd::Pushd;
use crate::process::pwd::Pwd;
use crate::process::r#type::Type;
use crate::process::welcome::Welcome;
use crate::process::which::Which;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Shared interface that lets [`BuiltinMap`] invoke builtins without knowing their concrete types.
trait BuiltinAdapter: Any {
    /// Execute the builtin with the provided argument list, returning its exit status when available.
    fn call(&self, args: &[String]) -> Option<i32>;
    /// Allow downcasting back to the underlying builtin wrapper when handles are needed.
    fn as_any(&self) -> &dyn Any;
}

/// Type-erased wrapper that stores builtins behind reference-counted interior mutability.
struct BuiltinWrapper<T: Builtin + 'static> {
    inner: Rc<RefCell<T>>,
}

impl<T: Builtin + 'static> BuiltinWrapper<T> {
    /// Create a new wrapper from an existing builtin handle.
    fn new(handle: Rc<RefCell<T>>) -> Self {
        Self { inner: handle }
    }

    /// Produce an adapter suitable for storage inside the builtin map.
    fn adapter(handle: Rc<RefCell<T>>) -> Rc<dyn BuiltinAdapter> {
        Rc::new(Self::new(handle))
    }

    /// Borrow the wrapped builtin handle so callers can configure dependencies.
    fn handle(&self) -> Rc<RefCell<T>> {
        self.inner.clone()
    }
}

impl<T: Builtin + 'static> BuiltinAdapter for BuiltinWrapper<T> {
    /// Forward the invocation to the wrapped builtin instance.
    fn call(&self, args: &[String]) -> Option<i32> {
        self.inner.borrow_mut().call(args)
    }

    /// Expose the wrapper as [`Any`] to enable downcasting by name.
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Helper that gathers handles for builtins requiring post-registration wiring.
#[derive(Default)]
struct BuiltinHandles {
    alias: Option<Rc<RefCell<Alias>>>,
    pwd: Option<Rc<RefCell<Pwd>>>,
    cd: Option<Rc<RefCell<Cd>>>,
    which: Option<Rc<RefCell<Which>>>,
}

/// Populate a builtin map using a set of builtin names and capture selected handles for later use.
macro_rules! register_builtins {
    ($map:expr, $names:expr) => {{
        let mut handles = BuiltinHandles::default();
        for name in $names {
            match name.as_str() {
                "alias" => handles.alias = Some(insert_builtin($map, "alias", Alias::new())),
                "pwd" => handles.pwd = Some(insert_builtin($map, "pwd", Pwd::new())),
                "cd" => handles.cd = Some(insert_builtin($map, "cd", Cd::new())),
                "exit" => {
                    insert_builtin($map, "exit", Exit::new());
                }
                "help" => {
                    insert_builtin($map, "help", Help::new());
                }
                "history" => {
                    insert_builtin($map, "history", History::new());
                }
                "pushd" => {
                    insert_builtin($map, "pushd", Pushd::new());
                }
                "type" => {
                    insert_builtin($map, "type", Type::new());
                }
                "welcome" => {
                    insert_builtin($map, "welcome", Welcome::new());
                }
                "which" => {
                    handles.which = Some(insert_builtin($map, "which", Which::new()));
                }
                other => panic!("unsupported builtin name: {}", other),
            }
        }
        handles
    }};
}

/// Concrete mapping between builtin names and runtime adapters.
pub struct BuiltinMap {
    func_map: HashMap<String, Rc<dyn BuiltinAdapter>>,
}

impl BuiltinMap {
    /// Register the default set of builtins and wire up their interdependencies.
    pub fn new() -> Self {
        let mut func_map: HashMap<String, Rc<dyn BuiltinAdapter>> = HashMap::new();

        let BuiltinHandles {
            alias,
            pwd,
            cd,
            which,
        } = register_builtins!(
            &mut func_map,
            vec![
                "alias".to_string(),
                "pwd".to_string(),
                "cd".to_string(),
                "exit".to_string(),
                "help".to_string(),
                "history".to_string(),
                "pushd".to_string(),
                "type".to_string(),
                "welcome".to_string(),
                "which".to_string(),
            ]
        );

        let alias = alias.expect("alias builtin not registered");
        let pwd = pwd.expect("pwd builtin not registered");
        let cd = cd.expect("cd builtin not registered");
        let which = which.expect("which builtin not registered");

        cd.borrow_mut().set_pwd(pwd.clone());
        which.borrow_mut().set_aliases(alias.clone());
        let builtin_names: Vec<String> = func_map.keys().cloned().collect();
        which.borrow_mut().set_builtin_names(builtin_names);

        Self { func_map }
    }

    /// Attempt to invoke a builtin by name, returning its status if the builtin exists.
    pub fn invoke(&self, func_name: &str, args: &[String]) -> Option<Option<i32>> {
        self.func_map
            .get(func_name)
            .map(|adapter| adapter.call(args))
    }

    /// Retrieve the shared alias handle so other components can mutate the alias map.
    pub fn get_alias(&self) -> Rc<RefCell<Alias>> {
        self.get_handle("alias")
            .expect("alias builtin not registered")
    }

    /// Convenience accessor that reports the current working directory tracked by the `pwd` builtin.
    pub fn get_pwd(&self) -> String {
        self.get_handle::<Pwd>("pwd")
            .map(|pwd| pwd.borrow().get_pwd())
            .unwrap_or_default()
    }

    /// Downcast the stored adapter to recover the concrete builtin handle for the requested name.
    fn get_handle<T: Builtin + 'static>(&self, name: &str) -> Option<Rc<RefCell<T>>> {
        self.func_map.get(name).and_then(|adapter| {
            adapter
                .as_any()
                .downcast_ref::<BuiltinWrapper<T>>()
                .map(|wrapper| wrapper.handle())
        })
    }
}

/// Insert a builtin into the provided map and return a handle to the stored instance.
fn insert_builtin<T: Builtin + 'static>(
    map: &mut HashMap<String, Rc<dyn BuiltinAdapter>>,
    name: &str,
    instance: T,
) -> Rc<RefCell<T>> {
    let handle = Rc::new(RefCell::new(instance));
    map.insert(name.to_string(), BuiltinWrapper::adapter(handle.clone()));
    handle
}
