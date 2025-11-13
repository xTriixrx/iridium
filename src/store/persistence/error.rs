use crate::store::compress::CompressionError;
use std::io;
use std::string::FromUtf8Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("persistence I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid persistence file magic header")]
    InvalidMagic,
    #[error("unsupported persistence version {0}")]
    UnsupportedVersion(u32),
    #[error("unsupported persistence flags {0:#X}")]
    UnsupportedFlags(u32),
    #[error("buffer database contains invalid utf-8 data")]
    InvalidUtf8(#[from] FromUtf8Error),
    #[error("buffer database value overflow in {0}")]
    ValueOverflow(&'static str),
    #[error("persistence encryption key is missing")]
    MissingEncryptionKey,
    #[error("persistence encrypted payload missing salt information")]
    MissingSalt,
    #[error("invalid encryption configuration: {0}")]
    InvalidEncryptionConfig(String),
    #[error("encryption failure: {0}")]
    Crypto(&'static str),
    #[error("corrupt persistence payload: {0}")]
    CorruptPayload(&'static str),
    #[error("compression failure: {0}")]
    Compression(#[from] CompressionError),
}

pub type PersistenceResult<T> = Result<T, PersistenceError>;
