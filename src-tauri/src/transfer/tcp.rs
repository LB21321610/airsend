use std::net::SocketAddr;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::transfer::{resolve_destination_path, TransferEntry, TransferManifest};

const MAX_HEADER_BYTES: u32 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferHeader {
    transfer_id: String,
    entries: Vec<TransferHeaderEntry>,
    total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferHeaderEntry {
    relative_path: String,
    size: u64,
    blake3: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiveSummary {
    pub files_written: usize,
    pub total_bytes: u64,
}

pub async fn send_transfer_once(
    addr: SocketAddr,
    manifest: TransferManifest,
) -> Result<(), TcpTransferError> {
    let mut stream = tokio::net::TcpStream::connect(addr)
        .await
        .map_err(TcpTransferError::Connect)?;
    let header = header_from_manifest(&manifest).await?;
    let header_bytes = serde_json::to_vec(&header)?;
    if header_bytes.len() > MAX_HEADER_BYTES as usize {
        return Err(TcpTransferError::HeaderTooLarge(header_bytes.len()));
    }

    stream
        .write_u32(header_bytes.len() as u32)
        .await
        .map_err(TcpTransferError::Write)?;
    stream
        .write_all(&header_bytes)
        .await
        .map_err(TcpTransferError::Write)?;

    for entry in manifest.entries {
        let mut file = tokio::fs::File::open(&entry.source_path)
            .await
            .map_err(|source| TcpTransferError::ReadFile {
                path: entry.source_path.clone(),
                source,
            })?;
        tokio::io::copy(&mut file, &mut stream)
            .await
            .map_err(TcpTransferError::Write)?;
    }

    stream.shutdown().await.map_err(TcpTransferError::Write)?;
    Ok(())
}

pub async fn receive_transfer_once(
    listener: tokio::net::TcpListener,
    destination_root: PathBuf,
) -> Result<ReceiveSummary, TcpTransferError> {
    let (mut stream, _) = listener.accept().await.map_err(TcpTransferError::Accept)?;
    let header_len = stream.read_u32().await.map_err(TcpTransferError::Read)?;
    if header_len > MAX_HEADER_BYTES {
        return Err(TcpTransferError::HeaderTooLarge(header_len as usize));
    }

    let mut header_bytes = vec![0; header_len as usize];
    stream
        .read_exact(&mut header_bytes)
        .await
        .map_err(TcpTransferError::Read)?;
    let header: TransferHeader = serde_json::from_slice(&header_bytes)?;

    let mut files_written = 0;
    let mut total_bytes = 0;
    for entry in header.entries {
        let destination = resolve_destination_path(&destination_root, &entry.relative_path)?;
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(TcpTransferError::CreateDestinationDir)?;
        }

        let mut remaining = entry.size;
        let mut hasher = blake3::Hasher::new();
        let mut file = tokio::fs::File::create(&destination)
            .await
            .map_err(|source| TcpTransferError::CreateFile {
                path: destination.clone(),
                source,
            })?;
        let mut buffer = vec![0_u8; 64 * 1024];

        while remaining > 0 {
            let read_len = buffer.len().min(remaining as usize);
            stream
                .read_exact(&mut buffer[..read_len])
                .await
                .map_err(TcpTransferError::Read)?;
            hasher.update(&buffer[..read_len]);
            file.write_all(&buffer[..read_len])
                .await
                .map_err(TcpTransferError::Write)?;
            remaining -= read_len as u64;
            total_bytes += read_len as u64;
        }

        let actual = hasher.finalize().to_hex().to_string();
        if actual != entry.blake3 {
            return Err(TcpTransferError::DigestMismatch {
                path: entry.relative_path,
                expected: entry.blake3,
                actual,
            });
        }

        files_written += 1;
    }

    Ok(ReceiveSummary {
        files_written,
        total_bytes,
    })
}

async fn header_from_manifest(
    manifest: &TransferManifest,
) -> Result<TransferHeader, TcpTransferError> {
    let mut entries = Vec::with_capacity(manifest.entries.len());
    for entry in &manifest.entries {
        entries.push(header_entry(entry).await?);
    }

    Ok(TransferHeader {
        transfer_id: manifest.transfer_id.clone(),
        entries,
        total_bytes: manifest.total_bytes,
    })
}

async fn header_entry(entry: &TransferEntry) -> Result<TransferHeaderEntry, TcpTransferError> {
    let bytes = tokio::fs::read(&entry.source_path)
        .await
        .map_err(|source| TcpTransferError::ReadFile {
            path: entry.source_path.clone(),
            source,
        })?;
    Ok(TransferHeaderEntry {
        relative_path: entry.relative_path.to_string_lossy().to_string(),
        size: entry.size,
        blake3: blake3::hash(&bytes).to_hex().to_string(),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum TcpTransferError {
    #[error("failed to connect to peer: {0}")]
    Connect(std::io::Error),
    #[error("failed to accept peer connection: {0}")]
    Accept(std::io::Error),
    #[error("failed to read from peer: {0}")]
    Read(std::io::Error),
    #[error("failed to write to peer: {0}")]
    Write(std::io::Error),
    #[error("failed to read source file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to create destination directory: {0}")]
    CreateDestinationDir(std::io::Error),
    #[error("failed to create destination file {path}: {source}")]
    CreateFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("transfer header too large: {0} bytes")]
    HeaderTooLarge(usize),
    #[error("failed to encode/decode transfer header: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Transfer(#[from] crate::transfer::TransferError),
    #[error("digest mismatch for {path}: expected {expected}, got {actual}")]
    DigestMismatch {
        path: String,
        expected: String,
        actual: String,
    },
}
