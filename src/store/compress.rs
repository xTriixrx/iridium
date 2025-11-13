use lz4_flex::frame::{Error as Lz4FrameError, FrameDecoder, FrameEncoder};
use std::io::{Read, Write};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    Lz4,
}

impl CompressionAlgorithm {
    pub fn default() -> Self {
        CompressionAlgorithm::Lz4
    }

    pub fn flag_bit(self) -> u32 {
        match self {
            CompressionAlgorithm::Lz4 => 0x0010,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "lz4" => Some(CompressionAlgorithm::Lz4),
            _ => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum CompressionError {
    #[error("compression I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("compression frame error: {0}")]
    Frame(#[from] Lz4FrameError),
}

pub fn compress(data: &[u8], algorithm: CompressionAlgorithm) -> Result<Vec<u8>, CompressionError> {
    match algorithm {
        CompressionAlgorithm::Lz4 => {
            let mut encoder = FrameEncoder::new(Vec::new());
            encoder.write_all(data)?;
            let output = encoder.finish()?;
            Ok(output)
        }
    }
}

pub fn decompress(
    data: &[u8],
    algorithm: CompressionAlgorithm,
) -> Result<Vec<u8>, CompressionError> {
    match algorithm {
        CompressionAlgorithm::Lz4 => {
            let mut decoder = FrameDecoder::new(data);
            let mut output = Vec::new();
            decoder.read_to_end(&mut output)?;
            Ok(output)
        }
    }
}
