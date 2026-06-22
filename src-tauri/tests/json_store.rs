use std::fs;
use std::path::PathBuf;

use app_lib::storage::json_store::{
    AirsendStore, PartialTransferManifest, StoreData, VerifiedChunkRecord, STORE_SCHEMA_VERSION,
};

fn unique_store_path(name: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("airsend-{name}-{nonce}.json"))
}

#[test]
fn load_or_create_initializes_missing_store_with_schema_version() {
    let path = unique_store_path("missing");

    let store =
        AirsendStore::load_or_create(path.clone()).expect("missing store should initialize");

    assert_eq!(store.data().schema_version, STORE_SCHEMA_VERSION);
    assert!(store.data().trusted_peers.is_empty());
    assert!(path.exists());

    let _ = fs::remove_file(path);
}

#[test]
fn load_or_create_recovers_corrupt_store_without_panicking() {
    let path = unique_store_path("corrupt");
    fs::write(&path, "{this is not valid json").expect("write corrupt json");

    let store = AirsendStore::load_or_create(path.clone()).expect("corrupt store should recover");

    assert_eq!(store.data().schema_version, STORE_SCHEMA_VERSION);
    assert!(store.data().transfer_history.is_empty());
    assert!(path.with_extension("json.corrupt").exists());

    let _ = fs::remove_file(path.with_extension("json.corrupt"));
    let _ = fs::remove_file(path);
}

#[test]
fn save_persists_updated_settings_atomically() {
    let path = unique_store_path("save");
    let mut store = AirsendStore::load_or_create(path.clone()).expect("store should initialize");
    store.data_mut().settings.save_directory = Some("/tmp/airsend-downloads".into());
    store.save().expect("save should succeed");

    let saved: StoreData =
        serde_json::from_slice(&fs::read(&path).expect("store file should be readable"))
            .expect("store file should be valid json");

    assert_eq!(
        saved.settings.save_directory.as_deref(),
        Some("/tmp/airsend-downloads")
    );
    assert!(!path.with_extension("json.tmp").exists());

    let _ = fs::remove_file(path);
}

#[test]
fn save_can_replace_an_existing_store_file() {
    let path = unique_store_path("replace");
    let mut store = AirsendStore::load_or_create(path.clone()).expect("store should initialize");

    store.data_mut().settings.auto_accept = true;
    store.save().expect("first update should save");
    store.data_mut().settings.auto_accept = false;
    store
        .save()
        .expect("second update should replace existing file");

    let saved: StoreData =
        serde_json::from_slice(&fs::read(&path).expect("store file should be readable"))
            .expect("store file should be valid json");

    assert!(!saved.settings.auto_accept);

    let _ = fs::remove_file(path);
}

#[test]
fn partial_transfer_persists_verified_chunks_by_file_and_chunk_index() {
    let path = unique_store_path("partial");
    let mut store = AirsendStore::load_or_create(path.clone()).expect("store should initialize");
    store
        .data_mut()
        .partial_transfers
        .push(PartialTransferManifest {
            transfer_id: "transfer-1".into(),
            root_name: "bundle".into(),
            total_bytes: 42,
            chunk_size: 8,
            verified_chunks: vec![
                VerifiedChunkRecord {
                    file_index: 0,
                    chunk_index: 0,
                },
                VerifiedChunkRecord {
                    file_index: 1,
                    chunk_index: 0,
                },
            ],
        });
    store.save().expect("partial transfer should save");

    let reloaded = AirsendStore::load_or_create(path.clone()).expect("store should reload");

    assert_eq!(
        reloaded.data().partial_transfers[0].verified_chunks.len(),
        2
    );
    assert_ne!(
        reloaded.data().partial_transfers[0].verified_chunks[0],
        reloaded.data().partial_transfers[0].verified_chunks[1]
    );

    let _ = fs::remove_file(path);
}
