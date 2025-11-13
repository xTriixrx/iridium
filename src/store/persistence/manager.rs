use super::binary::BinaryBufferDb;
use super::config::PersistenceConfig;
use super::crypto::EncryptionMode;
use super::error::PersistenceResult;
use super::pipeline::{CompressionLayer, EncryptionLayer, PersistencePipeline};
use crate::store::buffer_snapshot::BufferSnapshot;

pub struct PersistenceManager {
    config: PersistenceConfig,
    pipeline: PersistencePipeline,
}

impl PersistenceManager {
    pub fn new(config: PersistenceConfig) -> Self {
        let mut pipeline = PersistencePipeline::new();
        pipeline.push_layer(Box::new(CompressionLayer::new(config.compression())));
        if let EncryptionMode::Enabled(settings) = config.encryption().clone() {
            pipeline.push_layer(Box::new(EncryptionLayer::new(settings)));
        }
        Self { config, pipeline }
    }

    pub fn load(&self) -> PersistenceResult<Vec<BufferSnapshot>> {
        match self.config.path() {
            Some(path) => BinaryBufferDb::load(path, &self.pipeline),
            None => Ok(Vec::new()),
        }
    }

    pub fn store(&self, snapshots: &[BufferSnapshot]) -> PersistenceResult<()> {
        match self.config.path() {
            Some(path) => BinaryBufferDb::store(path, &self.pipeline, snapshots),
            None => Ok(()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.is_enabled()
    }
}
