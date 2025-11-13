use iridium::conf::ConfigurationModel;
use iridium::store::buffer_store::BufferStore;
use iridium::store::persistence::{
    EncryptionAlgorithm, EncryptionKeySource, EncryptionMode, EncryptionSettings,
    PersistenceConfig, PersistenceManager,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn buffer_snapshots_roundtrip_plaintext() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("buffers.db");
    let manager = PersistenceManager::new(PersistenceConfig::with_path(db_path.clone()));

    let mut store = BufferStore::new();
    store.open("alpha").append("first".into());
    store.open("beta").append("second".into());
    store.mark_closed("beta");

    let snapshots = store.snapshots();
    manager.store(&snapshots).expect("store snapshots");
    assert!(db_path.exists());

    let restored = manager.load().expect("load snapshots");
    let mut rehydrated = BufferStore::new();
    rehydrated.hydrate(restored);

    assert_eq!(
        rehydrated.get("alpha").unwrap().lines(),
        &["first".to_string()]
    );
    assert_eq!(
        rehydrated.get("beta").unwrap().lines(),
        &["second".to_string()]
    );
    assert!(!rehydrated.get("beta").unwrap().is_open());
}

#[test]
fn buffer_snapshots_roundtrip_encrypted_raw_key() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("buffers.db");
    let encryption = EncryptionMode::Enabled(EncryptionSettings {
        algorithm: EncryptionAlgorithm::ChaCha20Poly1305,
        key_source: EncryptionKeySource::RawKey([0xAA; 32]),
    });
    let config = PersistenceConfig::with_path_and_encryption(db_path.clone(), encryption);
    let manager = PersistenceManager::new(config);

    let mut store = BufferStore::new();
    store.open("alpha").append("first".into());
    let snapshots = store.snapshots();

    manager.store(&snapshots).expect("store snapshots");
    assert!(db_path.exists());

    let restored = manager.load().expect("load snapshots");
    let mut rehydrated = BufferStore::new();
    rehydrated.hydrate(restored);
    assert_eq!(
        rehydrated.get("alpha").unwrap().lines(),
        &["first".to_string()]
    );
}

#[test]
fn buffer_snapshots_roundtrip_encrypted_passphrase() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("buffers.db");
    let encryption = EncryptionMode::Enabled(EncryptionSettings {
        algorithm: EncryptionAlgorithm::ChaCha20Poly1305,
        key_source: EncryptionKeySource::Passphrase {
            passphrase: "test-passphrase".into(),
            iterations: 32,
        },
    });
    let config = PersistenceConfig::with_path_and_encryption(db_path.clone(), encryption);
    let manager = PersistenceManager::new(config);

    let mut store = BufferStore::new();
    store.open("beta").append("secret".into());
    let snapshots = store.snapshots();

    manager.store(&snapshots).expect("store snapshots");
    assert!(db_path.exists());

    let encryption = EncryptionMode::Enabled(EncryptionSettings {
        algorithm: EncryptionAlgorithm::ChaCha20Poly1305,
        key_source: EncryptionKeySource::Passphrase {
            passphrase: "test-passphrase".into(),
            iterations: 32,
        },
    });
    let config = PersistenceConfig::with_path_and_encryption(db_path.clone(), encryption);
    let manager = PersistenceManager::new(config);

    let restored = manager.load().expect("load snapshots");
    let mut rehydrated = BufferStore::new();
    rehydrated.hydrate(restored);
    assert_eq!(
        rehydrated.get("beta").unwrap().lines(),
        &["secret".to_string()]
    );
}

#[test]
fn buffer_snapshots_roundtrip_encrypted_via_config() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("buffers.db");
    let key_path = dir.path().join("key.hex");
    fs::write(
        &key_path,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .expect("write key");

    let mut cfg = ConfigurationModel::default();
    cfg.persistence.encrypt = Some(true);
    cfg.persistence.algorithm = Some("chacha20poly1305".into());
    cfg.persistence.key_file = Some(key_path.to_string_lossy().to_string());
    cfg.persistence.compression = Some("lz4".into());
    cfg.persistence.database_path = Some(db_path.to_string_lossy().to_string());

    let persistence_cfg = PersistenceConfig::from_sources(Some(&cfg));
    let manager = PersistenceManager::new(persistence_cfg);

    let mut store = BufferStore::new();
    store.open("alpha").append("first".into());
    let snapshots = store.snapshots();
    manager.store(&snapshots).expect("store snapshots");
    assert!(db_path.exists());

    let restored = manager.load().expect("load snapshots");
    let mut rehydrated = BufferStore::new();
    rehydrated.hydrate(restored);
    assert_eq!(
        rehydrated.get("alpha").unwrap().lines(),
        &["first".to_string()]
    );
}
