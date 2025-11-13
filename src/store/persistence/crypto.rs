use super::error::{PersistenceError, PersistenceResult};
use crate::conf::{ConfigurationModel, PersistenceConfigSection};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce as AesNonce};
use chacha20poly1305::{ChaCha20Poly1305, Nonce as ChaChaNonce};
use hex::FromHex;
use pbkdf2::pbkdf2_hmac;
use rand_core::{OsRng, RngCore};
use sha2::Sha256;
use std::env;
use std::fs;

pub(crate) const ENCRYPT_ENV: &str = "IRIDIUM_PERSIST_ENCRYPT";
const ENCRYPT_ALGO_ENV: &str = "IRIDIUM_PERSIST_ALGO";
const ENCRYPT_KEY_ENV: &str = "IRIDIUM_PERSIST_KEY";
const ENCRYPT_KEY_FILE_ENV: &str = "IRIDIUM_PERSIST_KEY_FILE";
const ENCRYPT_PASSPHRASE_ENV: &str = "IRIDIUM_PERSIST_PASSPHRASE";
const ENCRYPT_PBKDF_ITERS_ENV: &str = "IRIDIUM_PERSIST_PBKDF_ITERS";
const DEFAULT_PBKDF2_ITERS: u32 = 600_000;
const KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;

pub fn resolve_encryption(config: Option<&ConfigurationModel>) -> EncryptionMode {
    if let Ok(val) = env::var(ENCRYPT_ENV) {
        if is_truthy(&val) {
            return EncryptionMode::from_env().unwrap_or_else(|err| {
                eprintln!("Warning: encryption disabled due to configuration error: {err}");
                EncryptionMode::Disabled
            });
        }
    }

    if let Some(cfg) = config {
        if cfg.persistence.encrypt.unwrap_or(false) {
            return EncryptionMode::from_config(&cfg.persistence, cfg).unwrap_or_else(|err| {
                eprintln!("Warning: encryption disabled due to configuration error: {err}");
                EncryptionMode::Disabled
            });
        }
    }

    EncryptionMode::Disabled
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[derive(Debug, Clone)]
pub enum EncryptionMode {
    Disabled,
    Enabled(EncryptionSettings),
}

impl EncryptionMode {
    pub fn from_env() -> PersistenceResult<Self> {
        let algorithm = match env::var(ENCRYPT_ALGO_ENV) {
            Ok(value) => EncryptionAlgorithm::from_str(&value)?,
            Err(_) => EncryptionAlgorithm::default(),
        };

        let key_source = parse_key_source_from_env()?;
        Ok(EncryptionMode::Enabled(EncryptionSettings {
            algorithm,
            key_source,
        }))
    }

    pub fn from_config(
        section: &PersistenceConfigSection,
        config: &ConfigurationModel,
    ) -> PersistenceResult<Self> {
        if !section.encrypt.unwrap_or(false) {
            return Ok(EncryptionMode::Disabled);
        }

        let algorithm = match section.algorithm.as_ref() {
            Some(value) => EncryptionAlgorithm::from_str(value)?,
            None => EncryptionAlgorithm::default(),
        };

        let key_source = parse_key_source_from_config(section, config)?;

        Ok(EncryptionMode::Enabled(EncryptionSettings {
            algorithm,
            key_source,
        }))
    }
}

#[derive(Debug, Clone)]
pub struct EncryptionSettings {
    pub algorithm: EncryptionAlgorithm,
    pub key_source: EncryptionKeySource,
}

#[derive(Debug, Clone)]
pub enum EncryptionAlgorithm {
    ChaCha20Poly1305,
    Aes256Gcm,
}

impl EncryptionAlgorithm {
    pub fn flag_bit(&self) -> u32 {
        match self {
            EncryptionAlgorithm::ChaCha20Poly1305 => 0x0001,
            EncryptionAlgorithm::Aes256Gcm => 0x0002,
        }
    }

    pub fn nonce_len(&self) -> usize {
        12
    }

    pub fn encrypt(
        &self,
        key: &[u8; KEY_LEN],
        nonce: &[u8],
        plaintext: &[u8],
    ) -> PersistenceResult<Vec<u8>> {
        match self {
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new(key.into());
                cipher
                    .encrypt(ChaChaNonce::from_slice(nonce), plaintext)
                    .map_err(|_| PersistenceError::Crypto("ChaCha20-Poly1305 encryption failure"))
            }
            EncryptionAlgorithm::Aes256Gcm => {
                let cipher = Aes256Gcm::new(key.into());
                cipher
                    .encrypt(AesNonce::from_slice(nonce), plaintext)
                    .map_err(|_| PersistenceError::Crypto("AES-256-GCM encryption failure"))
            }
        }
    }

    pub fn decrypt(
        &self,
        key: &[u8; KEY_LEN],
        nonce: &[u8],
        ciphertext: &[u8],
    ) -> PersistenceResult<Vec<u8>> {
        match self {
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new(key.into());
                cipher
                    .decrypt(ChaChaNonce::from_slice(nonce), ciphertext)
                    .map_err(|_| PersistenceError::Crypto("ChaCha20-Poly1305 decryption failure"))
            }
            EncryptionAlgorithm::Aes256Gcm => {
                let cipher = Aes256Gcm::new(key.into());
                cipher
                    .decrypt(AesNonce::from_slice(nonce), ciphertext)
                    .map_err(|_| PersistenceError::Crypto("AES-256-GCM decryption failure"))
            }
        }
    }

    fn from_str(value: &str) -> PersistenceResult<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "aes256gcm" | "aes-256-gcm" => Ok(EncryptionAlgorithm::Aes256Gcm),
            "chacha20poly1305" | "chacha20" | "chacha" | "default" => {
                Ok(EncryptionAlgorithm::ChaCha20Poly1305)
            }
            other => Err(PersistenceError::InvalidEncryptionConfig(format!(
                "unknown algorithm '{other}'"
            ))),
        }
    }
}

impl Default for EncryptionAlgorithm {
    fn default() -> Self {
        EncryptionAlgorithm::ChaCha20Poly1305
    }
}

#[derive(Debug, Clone)]
pub enum EncryptionKeySource {
    RawKey([u8; KEY_LEN]),
    Passphrase { passphrase: String, iterations: u32 },
}

impl EncryptionKeySource {
    pub fn derive_for_encrypt(&self) -> PersistenceResult<KeyMaterial> {
        match self {
            EncryptionKeySource::RawKey(key) => Ok(KeyMaterial {
                key: *key,
                salt: None,
            }),
            EncryptionKeySource::Passphrase {
                passphrase,
                iterations,
            } => {
                let mut salt = [0u8; SALT_LEN];
                OsRng.fill_bytes(&mut salt);
                let key = derive_key_from_passphrase(passphrase, &salt, *iterations)?;
                Ok(KeyMaterial {
                    key,
                    salt: Some(salt.to_vec()),
                })
            }
        }
    }

    pub fn derive_for_decrypt(&self, salt: Option<&[u8]>) -> PersistenceResult<[u8; KEY_LEN]> {
        match self {
            EncryptionKeySource::RawKey(key) => {
                if let Some(s) = salt {
                    if !s.is_empty() {
                        return Err(PersistenceError::InvalidEncryptionConfig(
                            "encrypted file provided salt but raw key mode was configured".into(),
                        ));
                    }
                }
                Ok(*key)
            }
            EncryptionKeySource::Passphrase {
                passphrase,
                iterations,
            } => {
                let salt = salt.ok_or(PersistenceError::MissingSalt)?;
                if salt.len() != SALT_LEN {
                    return Err(PersistenceError::InvalidEncryptionConfig(
                        "encrypted file salt length mismatch".into(),
                    ));
                }
                derive_key_from_passphrase(passphrase, salt, *iterations)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyMaterial {
    pub key: [u8; KEY_LEN],
    pub salt: Option<Vec<u8>>,
}

fn derive_key_from_passphrase(
    passphrase: &str,
    salt: &[u8],
    iterations: u32,
) -> PersistenceResult<[u8; KEY_LEN]> {
    let mut key = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), salt, iterations, &mut key);
    Ok(key)
}

fn parse_key_source_from_env() -> PersistenceResult<EncryptionKeySource> {
    if let Ok(value) = env::var(ENCRYPT_KEY_ENV) {
        let key = decode_hex_key(&value)?;
        return Ok(EncryptionKeySource::RawKey(key));
    }

    if let Ok(path) = env::var(ENCRYPT_KEY_FILE_ENV) {
        let contents = fs::read_to_string(path)?;
        let key = decode_hex_key(contents.trim())?;
        return Ok(EncryptionKeySource::RawKey(key));
    }

    if let Ok(passphrase) = env::var(ENCRYPT_PASSPHRASE_ENV) {
        if passphrase.is_empty() {
            return Err(PersistenceError::InvalidEncryptionConfig(
                "passphrase cannot be empty".into(),
            ));
        }
        let iterations = env::var(ENCRYPT_PBKDF_ITERS_ENV)
            .ok()
            .and_then(|raw| raw.parse::<u32>().ok())
            .filter(|iters| *iters > 0)
            .unwrap_or(DEFAULT_PBKDF2_ITERS);
        return Ok(EncryptionKeySource::Passphrase {
            passphrase,
            iterations,
        });
    }

    Err(PersistenceError::MissingEncryptionKey)
}

fn parse_key_source_from_config(
    section: &PersistenceConfigSection,
    config: &ConfigurationModel,
) -> PersistenceResult<EncryptionKeySource> {
    if let Some(path) = section.resolved_key_path(config) {
        let contents = fs::read_to_string(&path)?;
        let key = decode_hex_key(contents.trim())?;
        return Ok(EncryptionKeySource::RawKey(key));
    }

    if let Some(passphrase) = section.passphrase.as_ref() {
        if passphrase.is_empty() {
            return Err(PersistenceError::InvalidEncryptionConfig(
                "passphrase cannot be empty".into(),
            ));
        }
        let iterations = section.pbkdf2_iterations.unwrap_or(DEFAULT_PBKDF2_ITERS);
        return Ok(EncryptionKeySource::Passphrase {
            passphrase: passphrase.clone(),
            iterations,
        });
    }

    Err(PersistenceError::MissingEncryptionKey)
}

fn decode_hex_key(input: &str) -> PersistenceResult<[u8; KEY_LEN]> {
    let sanitized: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = <[u8; KEY_LEN]>::from_hex(&sanitized).map_err(|_| {
        PersistenceError::InvalidEncryptionConfig("invalid hex key material".into())
    })?;
    Ok(bytes)
}
