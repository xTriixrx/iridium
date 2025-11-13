use iridium::process::builtin::Builtin;
use iridium::process::cd::Cd;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};
use tempfile::TempDir;

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
        Self {
            temp_dir: tempfile::tempdir().unwrap(),
            original_dir: env::current_dir().unwrap(),
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

fn run_cd(cd: &mut Cd, args: &[&str]) -> Option<i32> {
    let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    cd.call(&owned)
}

fn capture_output(cd: &mut Cd) -> Rc<RefCell<Vec<u8>>> {
    let buffer = Rc::new(RefCell::new(Vec::new()));
    cd.capture_output_buffer(buffer.clone());
    buffer
}

fn buffer_to_string(buffer: &Rc<RefCell<Vec<u8>>>) -> String {
    String::from_utf8(buffer.borrow().clone()).unwrap()
}

#[test]
fn cd_uses_home_when_no_operands() {
    let _guard = lock_env();
    let mut env_state = TestEnv::new();
    let home = env_state.root().join("home");
    fs::create_dir_all(&home).unwrap();
    env_state.set_var("HOME", home.to_str().unwrap());
    env_state.set_var("PWD", env_state.root().to_str().unwrap());
    env_state.set_current_dir(env_state.root().as_path());

    let mut cd = Cd::new();
    assert_eq!(run_cd(&mut cd, &[]), Some(0));
    assert_paths_equal(&env::current_dir().unwrap(), &home);
    assert_path_str_equal(&env::var("PWD").unwrap(), &home);
}

#[test]
fn cd_dash_prints_and_switches_to_oldpwd() {
    let _guard = lock_env();
    let mut env_state = TestEnv::new();
    let root = env_state.root();
    let first = root.join("first");
    let second = root.join("second");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();
    env_state.set_var("PWD", first.to_str().unwrap());
    env_state.set_var("OLDPWD", second.to_str().unwrap());
    env_state.set_current_dir(&first);

    let mut cd = Cd::new();
    let buffer = capture_output(&mut cd);
    assert_eq!(run_cd(&mut cd, &["-"]), Some(0));
    assert_paths_equal(&env::current_dir().unwrap(), &second);
    let output = buffer_to_string(&buffer);
    assert_path_str_equal(output.trim_end(), &second);
}

#[test]
fn cd_honors_cdpath_and_prints_target() {
    let _guard = lock_env();
    let mut env_state = TestEnv::new();
    let root = env_state.root();
    let cdpath_dir = root.join("paths");
    let target = cdpath_dir.join("project");
    fs::create_dir_all(&target).unwrap();
    env_state.set_var("CDPATH", cdpath_dir.to_str().unwrap());
    env_state.set_var("PWD", root.to_str().unwrap());
    env_state.set_current_dir(&root);

    let mut cd = Cd::new();
    let buffer = capture_output(&mut cd);
    assert_eq!(run_cd(&mut cd, &["project"]), Some(0));
    assert_paths_equal(&env::current_dir().unwrap(), &target);
    let output = buffer_to_string(&buffer);
    assert_path_str_equal(output.trim_end(), &target);
}

#[cfg(unix)]
#[test]
fn cd_physical_option_resolves_symlinks() {
    let _guard = lock_env();
    use std::os::unix::fs::symlink;

    let mut env_state = TestEnv::new();
    let root = env_state.root();
    let real = root.join("real");
    let alias_root = root.join("linkroot");
    fs::create_dir_all(real.join("nested")).unwrap();
    fs::create_dir_all(&alias_root).unwrap();
    symlink(&real, alias_root.join("alias")).unwrap();
    env_state.set_var("PWD", alias_root.to_str().unwrap());
    env_state.set_current_dir(&alias_root);

    let mut cd = Cd::new();
    assert_eq!(run_cd(&mut cd, &["-P", "alias/nested"]), Some(0));
    assert_path_str_equal(&env::var("PWD").unwrap(), &real.join("nested"));
    assert_paths_equal(&env::current_dir().unwrap(), &real.join("nested"));
}

#[test]
fn cd_reports_invalid_option() {
    let _guard = lock_env();
    let mut cd = Cd::new();
    assert_eq!(run_cd(&mut cd, &["-Z"]), Some(1));
}

fn canonical_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn canonical_string(path: &Path) -> String {
    canonical_path(path).to_string_lossy().to_string()
}

fn assert_paths_equal(lhs: &Path, rhs: &Path) {
    assert_eq!(canonical_path(lhs), canonical_path(rhs));
}

fn assert_path_str_equal(lhs: &str, rhs: &Path) {
    assert_eq!(canonical_path(Path::new(lhs)), canonical_path(rhs));
}
