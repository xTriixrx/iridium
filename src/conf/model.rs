use super::paths::expand_path;
use super::section::{ControlConfigSection, PersistenceConfigSection, UiConfigSection};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigurationModel {
    #[serde(default)]
    pub persistence: PersistenceConfigSection,
    #[serde(default)]
    #[allow(dead_code)]
    pub control: ControlConfigSection,
    #[serde(default)]
    #[allow(dead_code)]
    pub ui: UiConfigSection,
    #[serde(skip)]
    source_path: Option<PathBuf>,
}

impl ConfigurationModel {
    #[allow(dead_code)]
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    pub(crate) fn set_source_path(&mut self, path: PathBuf) {
        self.source_path = Some(path);
    }

    pub fn resolve_path(&self, raw: &str) -> PathBuf {
        let expanded = expand_path(raw);
        if raw == "~" || raw.starts_with("~/") || expanded.is_absolute() {
            return expanded;
        }

        if let Some(parent) = self.source_path.as_ref().and_then(|p| p.parent()) {
            return parent.join(raw);
        }

        expanded
    }
}
