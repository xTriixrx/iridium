use super::super::model::ConfigurationModel;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PersistenceConfigSection {
    pub database_path: Option<String>,
    pub encrypt: Option<bool>,
    pub algorithm: Option<String>,
    pub key_file: Option<String>,
    pub passphrase: Option<String>,
    pub pbkdf2_iterations: Option<u32>,
    pub compression: Option<String>,
}

impl PersistenceConfigSection {
    pub fn resolved_database_path(&self, config: &ConfigurationModel) -> Option<PathBuf> {
        self.database_path
            .as_ref()
            .map(|raw| config.resolve_path(raw))
    }

    pub fn resolved_key_path(&self, config: &ConfigurationModel) -> Option<PathBuf> {
        self.key_file.as_ref().map(|raw| config.resolve_path(raw))
    }
}
