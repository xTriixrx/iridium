use super::crypto::{self, EncryptionMode};
use crate::conf::ConfigurationModel;
use crate::store::compress::CompressionAlgorithm;
use std::env;
use std::path::{Path, PathBuf};

const PATH_ENV: &str = "IRIDIUM_BUFFER_DB_PATH";
const DISABLE_ENV: &str = "IRIDIUM_DISABLE_PERSISTENCE";
const COMPRESSION_ENV: &str = "IRIDIUM_PERSIST_COMPRESSION";

#[derive(Debug, Clone)]
pub enum PersistenceMode {
    Disabled,
    Enabled(PathBuf),
}

impl PersistenceMode {
    fn path(&self) -> Option<&Path> {
        match self {
            PersistenceMode::Disabled => None,
            PersistenceMode::Enabled(path) => Some(path.as_path()),
        }
    }

    fn is_enabled(&self) -> bool {
        matches!(self, PersistenceMode::Enabled(_))
    }
}

#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    mode: PersistenceMode,
    encryption: EncryptionMode,
    compression: CompressionAlgorithm,
}

impl PersistenceConfig {
    pub fn from_env() -> Self {
        Self::from_sources(None)
    }

    pub fn from_sources(config: Option<&ConfigurationModel>) -> Self {
        let mut configured_path =
            config.and_then(|cfg| cfg.persistence.resolved_database_path(cfg));

        if let Some(env_path) = env::var_os(PATH_ENV) {
            if !env_path.is_empty() {
                configured_path = Some(PathBuf::from(env_path));
            }
        }

        let mode = if env::var(DISABLE_ENV)
            .map(|val| is_truthy(&val))
            .unwrap_or(false)
        {
            PersistenceMode::Disabled
        } else {
            let path = configured_path.unwrap_or_else(default_persistence_path);
            PersistenceMode::Enabled(path)
        };

        let encryption = crypto::resolve_encryption(config);
        let compression = resolve_compression(config);

        Self {
            mode,
            encryption,
            compression,
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self {
            mode: PersistenceMode::Enabled(path),
            encryption: EncryptionMode::Disabled,
            compression: CompressionAlgorithm::default(),
        }
    }

    pub fn with_path_and_encryption(path: PathBuf, encryption: EncryptionMode) -> Self {
        Self {
            mode: PersistenceMode::Enabled(path),
            encryption,
            compression: CompressionAlgorithm::default(),
        }
    }

    pub fn disabled() -> Self {
        Self {
            mode: PersistenceMode::Disabled,
            encryption: EncryptionMode::Disabled,
            compression: CompressionAlgorithm::default(),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.mode.path()
    }

    pub fn is_enabled(&self) -> bool {
        self.mode.is_enabled()
    }

    pub fn encryption(&self) -> &EncryptionMode {
        &self.encryption
    }

    pub fn compression(&self) -> CompressionAlgorithm {
        self.compression
    }
}

fn resolve_compression(config: Option<&ConfigurationModel>) -> CompressionAlgorithm {
    if let Ok(value) = env::var(COMPRESSION_ENV) {
        if let Some(alg) = CompressionAlgorithm::from_name(&value) {
            return alg;
        } else {
            eprintln!("Warning: unknown compression algorithm '{value}', falling back to default");
        }
    }

    if let Some(cfg) = config {
        if let Some(name) = cfg.persistence.compression.as_ref() {
            if let Some(alg) = CompressionAlgorithm::from_name(name) {
                return alg;
            } else {
                eprintln!(
                    "Warning: unknown compression algorithm '{}' in config, falling back to default",
                    name
                );
            }
        }
    }

    CompressionAlgorithm::default()
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn default_persistence_path() -> PathBuf {
    let base = if cfg!(windows) {
        env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else if let Some(dir) = env::var_os("XDG_DATA_HOME") {
        Some(PathBuf::from(dir))
    } else if let Some(home) = env::var_os("HOME") {
        Some(PathBuf::from(home).join(".local/share"))
    } else {
        None
    };

    base.unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("iridium")
        .join("buffers.db")
}
