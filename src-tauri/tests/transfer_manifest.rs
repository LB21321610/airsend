use std::fs;
use std::path::PathBuf;

use app_lib::transfer::{
    build_transfer_manifest, chunk_plan_for_file, resolve_destination_path, resume_chunk_plan,
    safe_relative_destination_path, VerifiedChunk, DEFAULT_CHUNK_SIZE,
};

fn unique_temp_dir(name: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("airsend-{name}-{nonce}"));
    fs::create_dir_all(&path).expect("temp dir should be created");
    path
}

#[test]
fn manifest_expands_folders_and_preserves_relative_paths() {
    let root = unique_temp_dir("manifest");
    let folder = root.join("album");
    fs::create_dir_all(folder.join("nested")).expect("nested dir should be created");
    fs::write(folder.join("cover.txt"), b"cover").expect("cover should be written");
    fs::write(folder.join("nested/song.txt"), b"song").expect("song should be written");

    let manifest = build_transfer_manifest(vec![folder.clone()]).expect("manifest should build");

    assert_eq!(manifest.entries.len(), 2);
    assert_eq!(manifest.total_bytes, 9);
    assert!(manifest
        .entries
        .iter()
        .any(|entry| entry.relative_path == PathBuf::from("album/cover.txt")));
    assert!(manifest
        .entries
        .iter()
        .any(|entry| entry.relative_path == PathBuf::from("album/nested/song.txt")));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn chunk_plan_splits_large_files_at_default_chunk_size() {
    let chunks = chunk_plan_for_file(0, DEFAULT_CHUNK_SIZE * 2 + 17, DEFAULT_CHUNK_SIZE);

    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].offset, 0);
    assert_eq!(chunks[0].length, DEFAULT_CHUNK_SIZE);
    assert_eq!(chunks[2].offset, DEFAULT_CHUNK_SIZE * 2);
    assert_eq!(chunks[2].length, 17);
}

#[test]
fn resume_chunk_plan_skips_verified_chunks() {
    let chunks = chunk_plan_for_file(3, DEFAULT_CHUNK_SIZE * 3, DEFAULT_CHUNK_SIZE);
    let pending = resume_chunk_plan(
        chunks,
        &[
            VerifiedChunk {
                file_index: 3,
                chunk_index: 0,
            },
            VerifiedChunk {
                file_index: 3,
                chunk_index: 2,
            },
        ],
    );

    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].chunk_index, 1);
}

#[test]
fn resume_chunk_plan_does_not_skip_same_chunk_index_for_other_files() {
    let chunks = chunk_plan_for_file(4, DEFAULT_CHUNK_SIZE * 2, DEFAULT_CHUNK_SIZE);
    let pending = resume_chunk_plan(
        chunks,
        &[VerifiedChunk {
            file_index: 3,
            chunk_index: 0,
        }],
    );

    assert_eq!(pending.len(), 2);
}

#[test]
fn destination_conflicts_append_numeric_suffix() {
    let root = unique_temp_dir("conflict");
    let target = root.join("report.txt");
    fs::write(&target, b"existing").expect("existing file should be written");

    let resolved = resolve_destination_path(&root, "report.txt").expect("path should resolve");

    assert_eq!(resolved, root.join("report (1).txt"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn nested_destination_conflicts_preserve_parent_directory() {
    let root = unique_temp_dir("nested-conflict");
    fs::create_dir_all(root.join("dir")).expect("nested dir should be created");
    fs::write(root.join("dir/report.txt"), b"existing").expect("existing file should be written");

    let resolved = resolve_destination_path(&root, "dir/report.txt").expect("path should resolve");

    assert_eq!(resolved, root.join("dir/report (1).txt"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn destination_path_rejects_existing_file_as_parent() {
    let root = unique_temp_dir("parent-file");
    fs::write(root.join("dir"), b"not a directory").expect("parent file should be written");

    let err = resolve_destination_path(&root, "dir/report.txt")
        .expect_err("file parent should not be accepted");

    assert!(err
        .to_string()
        .contains("destination parent is not a directory"));

    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn destination_path_rejects_symlinked_parent_directories() {
    use std::os::unix::fs::symlink;

    let root = unique_temp_dir("dest-symlink");
    let outside = unique_temp_dir("dest-outside");
    symlink(&outside, root.join("dir")).expect("symlink should be created");

    let err = resolve_destination_path(&root, "dir/report.txt")
        .expect_err("symlink parent should not be accepted");

    assert!(err
        .to_string()
        .contains("destination parent uses a symlink"));

    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(outside);
}

#[test]
fn destination_path_rejects_traversal_and_absolute_paths() {
    assert!(safe_relative_destination_path("../evil.txt").is_err());
    assert!(safe_relative_destination_path("/tmp/evil.txt").is_err());
}

#[cfg(unix)]
#[test]
fn manifest_rejects_symlinks_for_mvp() {
    use std::os::unix::fs::symlink;

    let root = unique_temp_dir("symlink");
    let outside = root.join("outside.txt");
    fs::write(&outside, b"outside").expect("outside file should be written");
    let folder = root.join("folder");
    fs::create_dir_all(&folder).expect("folder should be created");
    symlink(&outside, folder.join("link.txt")).expect("symlink should be created");

    let err = build_transfer_manifest(vec![folder]).expect_err("symlink should be rejected");

    assert!(err.to_string().contains("symlinks are not supported"));

    let _ = fs::remove_dir_all(root);
}
