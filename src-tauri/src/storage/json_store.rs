use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const STORE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrustedPeer {
    pub device_id: String,
    pub display_name: String,
    pub fingerprint: String,
    pub trusted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoreSettings {
    pub save_directory: Option<String>,
    pub auto_accept: bool,
    pub parallel_workers: u8,
}

impl Default for StoreSettings {
    fn default() -> Self {
        Self {
            save_directory: None,
            auto_accept: false,
            parallel_workers: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferHistoryRecord {
    pub transfer_id: String,
    pub peer_id: String,
    pub peer_name: String,
    pub direction: TransferDirection,
    pub file_count: usize,
    pub total_bytes: u64,
    pub status: TransferStatus,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TransferDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TransferStatus {
    Queued,
    WaitingForApproval,
    Transferring,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PartialTransferManifest {
    pub transfer_id: String,
    pub root_name: String,
    pub total_bytes: u64,
    pub chunk_size: u64,
    pub verified_chunks: Vec<VerifiedChunkRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedChunkRecord {
    pub file_index: usize,
    pub chunk_index: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIdentity {
    pub device_id: String,
    pub display_name: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoreData {
    pub schema_version: u32,
    pub device: DeviceIdentity,
    pub trusted_peers: Vec<TrustedPeer>,
    pub settings: StoreSettings,
    pub transfer_history: Vec<TransferHistoryRecord>,
    pub partial_transfers: Vec<PartialTransferManifest>,
}

impl Default for StoreData {
    fn default() -> Self {
        let device_id = uuid::Uuid::new_v4().to_string();
        let display_name = hostname::get()
            .ok()
            .and_then(|name| name.into_string().ok())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "Airsend Device".to_string());
        let fingerprint = blake3::hash(device_id.as_bytes()).to_hex().to_string();

        Self {
            schema_version: STORE_SCHEMA_VERSION,
            device: DeviceIdentity {
                device_id,
                display_name,
                fingerprint,
            },
            trusted_peers: Vec::new(),
            settings: StoreSettings::default(),
            transfer_history: Vec::new(),
            partial_transfers: Vec::new(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("failed to create store directory: {0}")]
    CreateDir(io::Error),
    #[error("failed to read store: {0}")]
    Read(io::Error),
    #[error("failed to parse store: {0}")]
    Parse(serde_json::Error),
    #[error("failed to write store: {0}")]
    Write(io::Error),
    #[error("failed to replace store atomically: {0}")]
    Rename(io::Error),
}

pub struct AirsendStore {
    path: PathBuf,
    data: StoreData,
}

impl AirsendStore {
    pub fn load_or_create(path: PathBuf) -> Result<Self, StoreError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(StoreError::CreateDir)?;
        }

        if !path.exists() {
            let store = Self {
                path,
                data: StoreData::default(),
            };
            store.save()?;
            return Ok(store);
        }

        let bytes = fs::read(&path).map_err(StoreError::Read)?;
        match serde_json::from_slice::<StoreData>(&bytes) {
            Ok(mut data) => {
                if data.schema_version == 0 {
                    data.schema_version = STORE_SCHEMA_VERSION;
                }
                Ok(Self { path, data })
            }
            Err(_) => {
                let corrupt_path = path.with_extension("json.corrupt");
                let _ = fs::rename(&path, corrupt_path);
                let store = Self {
                    path,
                    data: StoreData::default(),
                };
                store.save()?;
                Ok(store)
            }
        }
    }

    pub fn data(&self) -> &StoreData {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut StoreData {
        &mut self.data
    }

    pub fn save(&self) -> Result<(), StoreError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(StoreError::CreateDir)?;
        }

        let tmp_path = self.path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(&self.data)
            .map_err(|err| StoreError::Write(io::Error::new(io::ErrorKind::InvalidData, err)))?;

        {
            let mut file = File::create(&tmp_path).map_err(StoreError::Write)?;
            file.write_all(&bytes).map_err(StoreError::Write)?;
            file.sync_all().map_err(StoreError::Write)?;
        }

        replace_file(&tmp_path, &self.path).map_err(StoreError::Rename)?;

        if let Some(parent) = self.path.parent() {
            if let Ok(dir) = File::open(parent) {
                let _ = dir.sync_all();
            }
        }

        Ok(())
    }
}

fn replace_file(tmp_path: &std::path::Path, target_path: &std::path::Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        if target_path.exists() {
            fs::remove_file(target_path)?;
        }
    }

    fs::rename(tmp_path, target_path)
}
