use crate::process::builtin::Builtin;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

/// POSIX-compliant `cd` builtin supporting logical/physical modes and CDPATH resolution.
pub struct Cd {
    output: CdOutput,
}

impl Cd {
    /// Construct a `cd` builtin that writes path announcements to stdout.
    pub fn new() -> Self {
        Self {
            output: CdOutput::Stdout,
        }
    }

    /// Route command output into the provided buffer (useful for tests).
    pub fn capture_output_buffer(&mut self, buffer: Rc<RefCell<Vec<u8>>>) {
        self.output = CdOutput::Buffer(buffer);
    }
}

impl Builtin for Cd {
    fn call(&mut self, args: &[String]) -> Option<i32> {
        match execute_cd(args) {
            Ok(print) => {
                if let Some(path) = print {
                    self.output.println(&path);
                }
                Some(0)
            }
            Err(err) => {
                eprintln!("{err}");
                Some(1)
            }
        }
    }
}

fn execute_cd(args: &[String]) -> Result<Option<String>, String> {
    let (mode, operand) = parse_arguments(args)?;
    let mut should_print = false;
    let operand = match operand {
        Some(val) => val,
        None => env::var("HOME").map_err(|_| "cd: HOME not set".to_string())?,
    };

    let operand = if operand == "-" {
        should_print = true;
        env::var("OLDPWD").map_err(|_| "cd: OLDPWD not set".to_string())?
    } else {
        operand
    };

    let operand = expand_tilde(&operand)?;
    let cdpath_result = resolve_with_cdpath(&operand)?;

    let previous_pwd = env::var("PWD")
        .ok()
        .unwrap_or_else(|| env::current_dir().unwrap().to_string_lossy().to_string());

    if let Err(err) = env::set_current_dir(&cdpath_result.actual_path) {
        return Err(format!(
            "cd: {}: {}",
            operand,
            err.kind().to_string().replace('_', " ").to_lowercase()
        ));
    }

    let new_physical = env::current_dir()
        .map_err(|err| format!("cd: unable to determine current directory: {err}"))?;

    let new_pwd = match mode {
        ResolveMode::Logical => build_logical_path(&previous_pwd, &cdpath_result.logical_operand),
        ResolveMode::Physical => new_physical.to_string_lossy().to_string(),
    };

    unsafe {
        env::set_var("OLDPWD", previous_pwd);
        env::set_var("PWD", new_pwd.clone());
    }

    let mut print_path = should_print;
    if cdpath_result.print_on_success {
        print_path = true;
    }

    Ok(print_path.then_some(new_pwd))
}

fn parse_arguments(args: &[String]) -> Result<(ResolveMode, Option<String>), String> {
    let mut mode = ResolveMode::Logical;
    let mut operands: Vec<String> = Vec::new();
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        if arg == "--" {
            operands.extend(iter.cloned());
            break;
        }

        if arg == "-" {
            operands.push(arg.clone());
            operands.extend(iter.cloned());
            break;
        }

        if arg.starts_with('-') && arg.len() > 1 {
            for ch in arg.chars().skip(1) {
                match ch {
                    'L' => mode = ResolveMode::Logical,
                    'P' => mode = ResolveMode::Physical,
                    _ => return Err(format!("cd: invalid option -- {}", ch)),
                }
            }
            continue;
        }

        operands.push(arg.clone());
        operands.extend(iter.cloned());
        break;
    }

    if operands.len() > 1 {
        return Err("cd: too many arguments".to_string());
    }

    Ok((mode, operands.into_iter().next()))
}

fn expand_tilde(input: &str) -> Result<String, String> {
    if let Some(stripped) = input.strip_prefix("~/") {
        let home = env::var("HOME").map_err(|_| "cd: HOME not set".to_string())?;
        return Ok(format!("{home}/{stripped}"));
    }

    if input == "~" {
        let home = env::var("HOME").map_err(|_| "cd: HOME not set".to_string())?;
        return Ok(home);
    }

    Ok(input.to_string())
}

struct CdpathResolution {
    actual_path: PathBuf,
    logical_operand: String,
    print_on_success: bool,
}

fn resolve_with_cdpath(dir: &str) -> Result<CdpathResolution, String> {
    let mut attempted = Vec::new();
    if eligible_for_cdpath(dir) {
        if let Ok(cdpath) = env::var("CDPATH") {
            for entry in cdpath.split(':') {
                let base = if entry.is_empty() { "." } else { entry };
                let candidate = Path::new(base).join(dir);
                if let Some(resolution) =
                    accept_candidate(&candidate, entry != "." && !entry.is_empty())
                {
                    return Ok(resolution);
                }
                attempted.push(candidate);
            }
        }
    }

    if let Some(resolution) = accept_candidate(&PathBuf::from(dir), false) {
        return Ok(resolution);
    }

    Err(format!("cd: no such file or directory: {}", dir))
}

fn accept_candidate(path: &PathBuf, print_on_success: bool) -> Option<CdpathResolution> {
    let absolute = if path.is_absolute() {
        path.clone()
    } else {
        match env::current_dir() {
            Ok(dir) => dir.join(path),
            Err(_) => return None,
        }
    };

    match fs::metadata(&absolute) {
        Ok(meta) if meta.is_dir() => Some(CdpathResolution {
            actual_path: absolute.clone(),
            logical_operand: to_string_lossy(path),
            print_on_success,
        }),
        _ => None,
    }
}

fn eligible_for_cdpath(dir: &str) -> bool {
    if dir.is_empty() {
        return false;
    }
    if dir.starts_with('/') {
        return false;
    }
    dir != "." && dir != ".." && !dir.starts_with("./") && !dir.starts_with("../")
}

fn build_logical_path(current: &str, operand: &str) -> String {
    let mut result = PathBuf::new();
    if Path::new(operand).is_absolute() {
        result.push(operand);
    } else {
        result.push(current);
        result.push(operand);
    }

    let mut stack: Vec<String> = Vec::new();
    let absolute = result.as_os_str().to_string_lossy().starts_with('/');

    for component in result.components() {
        match component {
            Component::RootDir => continue,
            Component::CurDir => continue,
            Component::ParentDir => {
                stack.pop();
            }
            Component::Normal(part) => stack.push(part.to_string_lossy().to_string()),
            Component::Prefix(_) => stack.push(component.as_os_str().to_string_lossy().to_string()),
        }
    }

    let mut normalized = if absolute {
        String::from("/")
    } else {
        String::new()
    };
    normalized.push_str(&stack.join("/"));
    if normalized.is_empty() {
        normalized.push('.');
    }
    normalized
}

fn to_string_lossy(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[derive(Copy, Clone)]
enum ResolveMode {
    Logical,
    Physical,
}

enum CdOutput {
    Stdout,
    Buffer(Rc<RefCell<Vec<u8>>>),
}

impl CdOutput {
    fn println(&mut self, value: &str) {
        match self {
            CdOutput::Stdout => {
                println!("{value}");
            }
            CdOutput::Buffer(buffer) => {
                let mut buf = buffer.borrow_mut();
                buf.extend_from_slice(value.as_bytes());
                buf.push(b'\n');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    use once_cell::sync::Lazy;
    use std::sync::{Mutex, MutexGuard};

    static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn lock_env<'a>() -> MutexGuard<'a, ()> {
        match ENV_LOCK.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        }
    }

    struct TestEnv {
        temp_dir: TempDir,
        original_dir: PathBuf,
        saved_env: HashMap<&'static str, Option<String>>,
    }

    impl TestEnv {
        fn new() -> Self {
            let original_dir = env::current_dir().unwrap();
            Self {
                temp_dir: tempfile::tempdir().unwrap(),
                original_dir,
                saved_env: HashMap::new(),
            }
        }

        fn root(&self) -> PathBuf {
            self.temp_dir.path().to_path_buf()
        }

        fn save_var(&mut self, key: &'static str) {
            self.saved_env.insert(key, env::var(key).ok());
        }

        fn set_var(&mut self, key: &'static str, value: impl AsRef<str>) {
            self.save_var(key);
            unsafe {
                env::set_var(key, value.as_ref());
            }
        }

        fn set_current_dir(&self, path: &Path) {
            env::set_current_dir(path).unwrap();
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            env::set_current_dir(&self.original_dir).ok();
            for (key, value) in &self.saved_env {
                if let Some(val) = value {
                    unsafe {
                        env::set_var(key, val);
                    }
                } else {
                    unsafe {
                        env::remove_var(key);
                    }
                }
            }
        }
    }

    fn buffer_output(cd: &CdOutput) -> String {
        match cd {
            CdOutput::Buffer(buf) => String::from_utf8(buf.borrow().clone()).unwrap(),
            _ => String::new(),
        }
    }

    #[test]
    fn cd_defaults_to_home() {
        let _guard = lock_env();
        let mut env_state = TestEnv::new();
        let home = env_state.root().join("home");
        fs::create_dir_all(&home).unwrap();
        env_state.set_var("HOME", home.to_str().unwrap());
        env_state.set_var("PWD", env_state.root().to_str().unwrap());
        env_state.set_current_dir(&env_state.root());

        let mut cd = Cd::new();
        let status = cd.call(&[]);
        assert_eq!(status, Some(0));
        assert_paths_equal(&env::current_dir().unwrap(), &home);
        assert_path_str_equal(&env::var("PWD").unwrap(), &home);
    }

    #[test]
    fn cd_dash_switches_to_oldpwd_and_prints() {
        let _guard = lock_env();
        let mut env_state = TestEnv::new();
        let root = env_state.root();
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        env_state.set_current_dir(&first);
        env_state.set_var("OLDPWD", second.to_str().unwrap());

        let buffer = Rc::new(RefCell::new(Vec::new()));
        let mut cd = Cd::new();
        cd.capture_output_buffer(buffer.clone());
        let status = cd.call(&[String::from("-")]);
        assert_eq!(status, Some(0));
        assert_paths_equal(&env::current_dir().unwrap(), &second);
        let output = buffer_output(&CdOutput::Buffer(buffer));
        assert_path_str_equal(output.trim_end(), &second);
    }

    #[test]
    fn cd_respects_cdpath() {
        let _guard = lock_env();
        let mut env_state = TestEnv::new();
        let root = env_state.root();
        let cdpath_dir = root.join("paths");
        let target = cdpath_dir.join("project");
        fs::create_dir_all(&target).unwrap();
        env_state.set_current_dir(&root);
        env_state.set_var("CDPATH", cdpath_dir.to_str().unwrap());
        env_state.set_var("PWD", root.to_str().unwrap());

        let buffer = Rc::new(RefCell::new(Vec::new()));
        let mut cd = Cd::new();
        cd.capture_output_buffer(buffer.clone());
        let status = cd.call(&[String::from("project")]);
        assert_eq!(status, Some(0));
        assert_paths_equal(&env::current_dir().unwrap(), &target);
        let output = buffer_output(&CdOutput::Buffer(buffer));
        assert_path_str_equal(output.trim_end(), &target);
    }

    #[test]
    fn cd_physical_option_updates_pwd() {
        let _guard = lock_env();
        let mut env_state = TestEnv::new();
        let root = env_state.root();
        let real = root.join("real");
        let link_parent = root.join("links");
        fs::create_dir_all(real.join("nested")).unwrap();
        fs::create_dir_all(&link_parent).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, link_parent.join("alias")).unwrap();
        env_state.set_current_dir(&link_parent);
        env_state.set_var("PWD", link_parent.to_str().unwrap());

        let mut cd = Cd::new();
        let status = cd.call(&[String::from("-P"), String::from("alias/nested")]);
        assert_eq!(status, Some(0));
        assert_path_str_equal(&env::var("PWD").unwrap(), &real.join("nested"));
    }

    #[test]
    fn cd_invalid_option_errors() {
        let mut cd = Cd::new();
        let status = cd.call(&[String::from("-Z")]);
        assert_eq!(status, Some(1));
    }

    fn canonical_path(path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }

    fn assert_paths_equal(lhs: &Path, rhs: &Path) {
        assert_eq!(canonical_path(lhs), canonical_path(rhs));
    }

    fn assert_path_str_equal(lhs: &str, rhs: &Path) {
        assert_eq!(canonical_path(Path::new(lhs)), canonical_path(rhs));
    }
}
