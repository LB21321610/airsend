use app_lib::security::pairing_code_for_fingerprints;
use app_lib::storage::json_store::{AirsendStore, TrustedPeer};

fn unique_store_path(name: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("airsend-{name}-{nonce}.json"))
}

#[test]
fn pairing_code_is_stable_six_digits() {
    let first = pairing_code_for_fingerprints("local", "remote");
    let second = pairing_code_for_fingerprints("local", "remote");

    assert_eq!(first.code, second.code);
    assert_eq!(first.code.len(), 6);
    assert!(first
        .code
        .chars()
        .all(|character| character.is_ascii_digit()));
}

#[test]
fn trusted_peer_round_trips_through_json_store() {
    let path = unique_store_path("trusted");
    let mut store = AirsendStore::load_or_create(path.clone()).expect("store should initialize");
    store.data_mut().trusted_peers.push(TrustedPeer {
        device_id: "peer-1".into(),
        display_name: "Peer One".into(),
        fingerprint: "abc123".into(),
        trusted_at: "123456".into(),
    });
    store.save().expect("trusted peer should save");

    let reloaded = AirsendStore::load_or_create(path.clone()).expect("store should reload");

    assert_eq!(reloaded.data().trusted_peers[0].fingerprint, "abc123");

    let _ = std::fs::remove_file(path);
}
