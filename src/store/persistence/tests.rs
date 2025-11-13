use super::config::PersistenceConfig;
use super::crypto::{EncryptionAlgorithm, EncryptionKeySource, EncryptionMode, EncryptionSettings};
use super::manager::PersistenceManager;
use super::pipeline::{CompressionLayer, EncryptionLayer, PersistenceLayer};
use crate::conf::ConfigurationModel;
use crate::store::buffer_snapshot::BufferSnapshot;
use crate::store::compress::CompressionAlgorithm;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn writes_and_loads_snapshots_plaintext() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("buffers.db");
    let manager = PersistenceManager::new(PersistenceConfig::with_path(path.clone()));

    let snapshots = vec![
        BufferSnapshot::new(
            "alpha".into(),
            vec!["first line".into(), "second".into()],
            false,
            true,
            true,
        ),
        BufferSnapshot::new("beta".into(), vec![], true, false, false),
    ];

    manager.store(&snapshots).unwrap();
    assert!(path.exists());

    let restored = manager.load().unwrap();
    assert_eq!(restored, snapshots);
}

#[test]
fn encryption_layer_roundtrip_with_raw_key() {
    let settings = EncryptionSettings {
        algorithm: EncryptionAlgorithm::ChaCha20Poly1305,
        key_source: EncryptionKeySource::RawKey([9u8; 32]),
    };
    let layer = EncryptionLayer::new(settings);
    let plaintext = b"secret payload".to_vec();
    let ciphertext = layer.encode(plaintext.clone()).unwrap();
    let decoded = layer.decode(ciphertext).unwrap();
    assert_eq!(decoded, plaintext);
}

#[test]
fn encryption_layer_roundtrip_with_passphrase() {
    let settings = EncryptionSettings {
        algorithm: EncryptionAlgorithm::ChaCha20Poly1305,
        key_source: EncryptionKeySource::Passphrase {
            passphrase: "hunter2".into(),
            iterations: 10,
        },
    };
    let layer = EncryptionLayer::new(settings);
    let plaintext = b"secret payload".to_vec();
    let ciphertext = layer.encode(plaintext.clone()).unwrap();
    let decoded = layer.decode(ciphertext).unwrap();
    assert_eq!(decoded, plaintext);
}

#[test]
fn encrypted_store_and_load_with_raw_key() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("encrypted.db");
    let config = PersistenceConfig::with_path_and_encryption(
        path.clone(),
        EncryptionMode::Enabled(EncryptionSettings {
            algorithm: EncryptionAlgorithm::ChaCha20Poly1305,
            key_source: EncryptionKeySource::RawKey([7u8; 32]),
        }),
    );
    let manager = PersistenceManager::new(config);

    let snapshots = vec![BufferSnapshot::new(
        "gamma".into(),
        vec!["line".into()],
        false,
        true,
        false,
    )];

    manager.store(&snapshots).unwrap();
    assert!(path.exists());

    let restored = manager.load().unwrap();
    assert_eq!(restored, snapshots);
}

#[test]
fn config_enables_encryption_when_requested() {
    let dir = tempdir().unwrap();
    let key_path = dir.path().join("cfg_key.hex");
    fs::write(
        &key_path,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .unwrap();

    let mut cfg = ConfigurationModel::default();
    cfg.persistence.encrypt = Some(true);
    cfg.persistence.algorithm = Some("aes256gcm".into());
    cfg.persistence.key_file = Some(key_path.to_string_lossy().to_string());

    let persistence_cfg = PersistenceConfig::from_sources(Some(&cfg));
    assert!(matches!(
        persistence_cfg.encryption(),
        EncryptionMode::Enabled(_)
    ));
}

#[test]
fn compression_layer_roundtrip() {
    let data =
        b"some text that compresses quite well and contains enough repeated patterns".to_vec();
    let layer = CompressionLayer::new(CompressionAlgorithm::Lz4);
    let compressed = layer.encode(data.clone()).expect("compress");
    let decompressed = layer.decode(compressed).expect("decompress");
    assert_eq!(decompressed, data);
}

#[test]
fn persistence_config_uses_default_compression() {
    let cfg = PersistenceConfig::with_path(PathBuf::from("dummy"));
    assert_eq!(cfg.compression(), CompressionAlgorithm::Lz4);
}

#[test]
fn compression_respects_config_option() {
    let mut config = ConfigurationModel::default();
    config.persistence.compression = Some("lz4".into());
    let cfg = PersistenceConfig::from_sources(Some(&config));
    assert_eq!(cfg.compression(), CompressionAlgorithm::Lz4);
}
