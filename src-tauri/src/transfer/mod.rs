use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub mod tcp;

pub use crate::protocol::{DEFAULT_CHUNK_SIZE, DEFAULT_PARALLEL_WORKERS};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferManifest {
    pub transfer_id: String,
    pub root_paths: Vec<PathBuf>,
    pub entries: Vec<TransferEntry>,
    pub total_bytes: u64,
    pub chunk_size: u64,
    pub parallel_workers: u8,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferEntry {
    pub source_path: PathBuf,
    pub relative_path: PathBuf,
    pub size: u64,
    pub blake3: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChunkPlan {
    pub file_index: usize,
    pub chunk_index: u64,
    pub offset: u64,
    pub length: u64,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedChunk {
    pub file_index: usize,
    pub chunk_index: u64,
}

impl TransferManifest {
    pub fn new(root_paths: Vec<PathBuf>, entries: Vec<TransferEntry>) -> Self {
        let total_bytes = entries.iter().map(|entry| entry.size).sum();
        Self {
            transfer_id: uuid::Uuid::new_v4().to_string(),
            root_paths,
            entries,
            total_bytes,
            chunk_size: DEFAULT_CHUNK_SIZE,
            parallel_workers: DEFAULT_PARALLEL_WORKERS,
        }
    }
}

pub fn build_transfer_manifest(paths: Vec<PathBuf>) -> Result<TransferManifest, TransferError> {
    if paths.is_empty() {
        return Err(TransferError::EmptySelection);
    }

    let mut entries = Vec::new();
    for path in &paths {
        reject_symlink(path)?;
        let metadata = fs::metadata(path).map_err(|source| TransferError::ReadMetadata {
            path: path.clone(),
            source,
        })?;

        if metadata.is_file() {
            let relative_path = path
                .file_name()
                .map(PathBuf::from)
                .ok_or_else(|| TransferError::InvalidFileName(path.clone()))?;
            entries.push(TransferEntry {
                source_path: path.clone(),
                relative_path,
                size: metadata.len(),
                blake3: None,
            });
        } else if metadata.is_dir() {
            let root_name = path
                .file_name()
                .map(PathBuf::from)
                .ok_or_else(|| TransferError::InvalidFileName(path.clone()))?;
            collect_dir_entries(path, &root_name, &mut entries)?;
        }
    }

    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(TransferManifest::new(paths, entries))
}

pub fn chunk_plan_for_file(file_index: usize, file_size: u64, chunk_size: u64) -> Vec<ChunkPlan> {
    if file_size == 0 || chunk_size == 0 {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut offset = 0;
    let mut chunk_index = 0;
    while offset < file_size {
        let remaining = file_size - offset;
        let length = remaining.min(chunk_size);
        chunks.push(ChunkPlan {
            file_index,
            chunk_index,
            offset,
            length,
        });
        offset += length;
        chunk_index += 1;
    }
    chunks
}

pub fn resume_chunk_plan(
    chunks: Vec<ChunkPlan>,
    verified_chunks: &[VerifiedChunk],
) -> Vec<ChunkPlan> {
    let verified: HashSet<VerifiedChunk> = verified_chunks.iter().copied().collect();
    chunks
        .into_iter()
        .filter(|chunk| {
            !verified.contains(&VerifiedChunk {
                file_index: chunk.file_index,
                chunk_index: chunk.chunk_index,
            })
        })
        .collect()
}

pub fn resolve_destination_path(
    root: &Path,
    relative_name: &str,
) -> Result<PathBuf, TransferError> {
    let safe_relative = safe_relative_destination_path(relative_name)?;
    validate_destination_parent(root, &safe_relative)?;
    let candidate = root.join(&safe_relative);
    if !candidate.exists() {
        return Ok(candidate);
    }

    let parent = safe_relative.parent().unwrap_or_else(|| Path::new(""));
    let file_name = safe_relative
        .file_name()
        .ok_or_else(|| TransferError::UnsafeDestination(relative_name.to_string()))?;
    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(relative_name);
    let extension = path.extension().and_then(|value| value.to_str());

    for suffix in 1.. {
        let file_name = match extension {
            Some(extension) => format!("{stem} ({suffix}).{extension}"),
            None => format!("{stem} ({suffix})"),
        };
        let candidate = root.join(parent).join(file_name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    unreachable!("suffix loop should always return");
}

pub fn safe_relative_destination_path(relative_name: &str) -> Result<PathBuf, TransferError> {
    let path = Path::new(relative_name);
    if path.is_absolute() {
        return Err(TransferError::UnsafeDestination(relative_name.to_string()));
    }

    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) if part != OsStr::new("") => safe.push(part),
            _ => return Err(TransferError::UnsafeDestination(relative_name.to_string())),
        }
    }

    if safe.as_os_str().is_empty() {
        return Err(TransferError::UnsafeDestination(relative_name.to_string()));
    }

    Ok(safe)
}

fn collect_dir_entries(
    dir: &Path,
    relative_root: &Path,
    entries: &mut Vec<TransferEntry>,
) -> Result<(), TransferError> {
    for entry in fs::read_dir(dir).map_err(|source| TransferError::ReadDirectory {
        path: dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| TransferError::ReadDirectory {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        reject_symlink(&path)?;
        let metadata = entry
            .metadata()
            .map_err(|source| TransferError::ReadMetadata {
                path: path.clone(),
                source,
            })?;
        let relative_path = relative_root.join(entry.file_name());

        if metadata.is_file() {
            entries.push(TransferEntry {
                source_path: path,
                relative_path,
                size: metadata.len(),
                blake3: None,
            });
        } else if metadata.is_dir() {
            collect_dir_entries(&path, &relative_path, entries)?;
        }
    }
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<(), TransferError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| TransferError::ReadMetadata {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() {
        return Err(TransferError::UnsupportedSymlink(path.to_path_buf()));
    }
    Ok(())
}

fn validate_destination_parent(root: &Path, safe_relative: &Path) -> Result<(), TransferError> {
    let mut current = root.to_path_buf();
    if let Some(parent) = safe_relative.parent() {
        for component in parent.components() {
            let std::path::Component::Normal(part) = component else {
                return Err(TransferError::UnsafeDestination(
                    safe_relative.display().to_string(),
                ));
            };
            current.push(part);
            if current.exists() {
                let metadata = fs::symlink_metadata(&current).map_err(|source| {
                    TransferError::ReadMetadata {
                        path: current.clone(),
                        source,
                    }
                })?;
                if metadata.file_type().is_symlink() {
                    return Err(TransferError::DestinationParentSymlink(current));
                }
                if !metadata.is_dir() {
                    return Err(TransferError::DestinationParentNotDirectory(current));
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum TransferError {
    #[error("select at least one file or folder")]
    EmptySelection,
    #[error("invalid file name for {0}")]
    InvalidFileName(PathBuf),
    #[error("failed to read metadata for {path}: {source}")]
    ReadMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to read directory {path}: {source}")]
    ReadDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("unsafe destination path: {0}")]
    UnsafeDestination(String),
    #[error("symlinks are not supported in the MVP transfer manifest: {0}")]
    UnsupportedSymlink(PathBuf),
    #[error("destination parent uses a symlink: {0}")]
    DestinationParentSymlink(PathBuf),
    #[error("destination parent is not a directory: {0}")]
    DestinationParentNotDirectory(PathBuf),
}
