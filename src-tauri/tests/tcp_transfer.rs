use std::fs;
use std::path::PathBuf;

use app_lib::transfer::tcp::{receive_transfer_once, send_transfer_once};
use app_lib::transfer::{build_transfer_manifest, resolve_destination_path};

fn unique_temp_dir(name: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("airsend-{name}-{nonce}"));
    fs::create_dir_all(&path).expect("temp dir should be created");
    path
}

#[tokio::test]
async fn tcp_transfer_sends_manifest_and_file_bytes_over_loopback() {
    let source_root = unique_temp_dir("tcp-source");
    let destination_root = unique_temp_dir("tcp-destination");
    let file = source_root.join("hello.txt");
    fs::write(&file, b"hello over local tcp").expect("source should be written");
    let manifest = build_transfer_manifest(vec![file]).expect("manifest should build");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("loopback listener should bind");
    let addr = listener.local_addr().expect("listener should have address");

    let receiver = tokio::spawn({
        let destination_root = destination_root.clone();
        async move { receive_transfer_once(listener, destination_root).await }
    });

    send_transfer_once(addr, manifest)
        .await
        .expect("send should complete");
    let received = receiver
        .await
        .expect("receiver task should finish")
        .expect("receive should complete");

    assert_eq!(received.files_written, 1);
    assert_eq!(
        fs::read(destination_root.join("hello.txt")).expect("received file should exist"),
        b"hello over local tcp"
    );
    assert_eq!(received.total_bytes, 20);

    let _ = fs::remove_dir_all(source_root);
    let _ = fs::remove_dir_all(destination_root);
}

#[test]
fn receiver_destination_resolution_is_used_before_write() {
    let destination_root = unique_temp_dir("tcp-conflict");
    fs::write(destination_root.join("hello.txt"), b"existing").expect("existing file");

    let resolved =
        resolve_destination_path(&destination_root, "hello.txt").expect("path should resolve");

    assert_eq!(resolved, destination_root.join("hello (1).txt"));

    let _ = fs::remove_dir_all(destination_root);
}
