use super::crypto::EncryptionSettings;
use super::error::PersistenceResult;
use crate::store::compress::{self, CompressionAlgorithm};
use rand_core::{OsRng, RngCore};
use std::io::{self, Cursor, Read};

pub struct PersistencePipeline {
    layers: Vec<Box<dyn PersistenceLayer + Send + Sync>>,
}

impl PersistencePipeline {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    pub fn push_layer(&mut self, layer: Box<dyn PersistenceLayer + Send + Sync>) {
        self.layers.push(layer);
    }

    pub fn flags(&self) -> u32 {
        self.layers
            .iter()
            .fold(0u32, |acc, layer| acc | layer.flag_bit())
    }

    pub fn encode(&self, data: Vec<u8>) -> PersistenceResult<Vec<u8>> {
        let mut current = data;
        for layer in &self.layers {
            current = layer.encode(current)?;
        }
        Ok(current)
    }

    pub fn decode(&self, data: Vec<u8>) -> PersistenceResult<Vec<u8>> {
        let mut current = data;
        for layer in self.layers.iter().rev() {
            current = layer.decode(current)?;
        }
        Ok(current)
    }
}

pub trait PersistenceLayer {
    fn encode(&self, data: Vec<u8>) -> PersistenceResult<Vec<u8>>;
    fn decode(&self, data: Vec<u8>) -> PersistenceResult<Vec<u8>>;
    fn flag_bit(&self) -> u32;
}

pub struct CompressionLayer {
    algorithm: CompressionAlgorithm,
}

impl CompressionLayer {
    pub fn new(algorithm: CompressionAlgorithm) -> Self {
        Self { algorithm }
    }
}

impl PersistenceLayer for CompressionLayer {
    fn encode(&self, data: Vec<u8>) -> PersistenceResult<Vec<u8>> {
        Ok(compress::compress(&data, self.algorithm)?)
    }

    fn decode(&self, data: Vec<u8>) -> PersistenceResult<Vec<u8>> {
        Ok(compress::decompress(&data, self.algorithm)?)
    }

    fn flag_bit(&self) -> u32 {
        self.algorithm.flag_bit()
    }
}

pub struct EncryptionLayer {
    settings: EncryptionSettings,
}

impl EncryptionLayer {
    pub fn new(settings: EncryptionSettings) -> Self {
        Self { settings }
    }
}

impl PersistenceLayer for EncryptionLayer {
    fn encode(&self, data: Vec<u8>) -> PersistenceResult<Vec<u8>> {
        let material = self.settings.key_source.derive_for_encrypt()?;
        let mut nonce = vec![0u8; self.settings.algorithm.nonce_len()];
        OsRng.fill_bytes(&mut nonce);
        let ciphertext = self
            .settings
            .algorithm
            .encrypt(&material.key, &nonce, &data)?;

        let salt_len = material.salt.as_ref().map(|s| s.len()).unwrap_or(0);
        let mut output = Vec::with_capacity(2 + salt_len + nonce.len() + ciphertext.len());
        output.push(salt_len as u8);
        if let Some(salt) = &material.salt {
            output.extend_from_slice(salt);
        }
        output.push(nonce.len() as u8);
        output.extend_from_slice(&nonce);
        output.extend_from_slice(&ciphertext);
        Ok(output)
    }

    fn decode(&self, data: Vec<u8>) -> PersistenceResult<Vec<u8>> {
        let mut cursor = Cursor::new(&data);
        let salt_len = read_u8(&mut cursor)? as usize;
        let salt = if salt_len > 0 {
            let mut salt_bytes = vec![0u8; salt_len];
            cursor.read_exact(&mut salt_bytes)?;
            Some(salt_bytes)
        } else {
            None
        };

        let nonce_len = read_u8(&mut cursor)? as usize;
        let mut nonce = vec![0u8; nonce_len];
        cursor.read_exact(&mut nonce)?;
        let mut ciphertext = Vec::new();
        cursor.read_to_end(&mut ciphertext)?;

        let key = self
            .settings
            .key_source
            .derive_for_decrypt(salt.as_deref())?;
        self.settings.algorithm.decrypt(&key, &nonce, &ciphertext)
    }

    fn flag_bit(&self) -> u32 {
        self.settings.algorithm.flag_bit()
    }
}

fn read_u8(reader: &mut dyn Read) -> io::Result<u8> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}
