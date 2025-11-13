use std::env;
use std::path::PathBuf;

pub const CONFIG_PATH_ENV: &str = "IRIDIUM_CONFIG";

pub fn resolve_config_path() -> Option<PathBuf> {
    if let Ok(env_path) = env::var(CONFIG_PATH_ENV) {
        if !env_path.trim().is_empty() {
            return Some(expand_path(&env_path));
        }
    }

    home_dir()
        .map(|home| home.join(".iridiumrc"))
        .filter(|path| path.exists())
}

pub fn expand_path(input: &str) -> PathBuf {
    if input == "~" {
        if let Some(home) = home_dir() {
            return home;
        }
    } else if let Some(rest) = input.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(input)
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from).or_else(|| {
        #[cfg(target_os = "windows")]
        {
            env::var_os("USERPROFILE").map(PathBuf::from)
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    })
}
