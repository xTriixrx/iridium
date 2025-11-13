use super::model::ConfigurationModel;
use super::paths::resolve_config_path;
use std::fs;

/// Load the user's configuration file, falling back to defaults when absent or invalid.
pub fn load() -> ConfigurationModel {
    let path = resolve_config_path();
    if let Some(path) = path {
        match fs::read_to_string(&path) {
            Ok(contents) => match serde_yaml::from_str::<ConfigurationModel>(&contents) {
                Ok(mut cfg) => {
                    cfg.set_source_path(path);
                    return cfg;
                }
                Err(err) => {
                    eprintln!(
                        "Warning: unable to parse config file '{}': {err}",
                        path.display()
                    );
                }
            },
            Err(err) => {
                eprintln!(
                    "Warning: unable to read config file '{}': {err}",
                    path.display()
                );
            }
        }
    }

    ConfigurationModel::default()
}
