use serde::{Deserialize, Serialize};

pub const DEFAULT_CHUNK_SIZE: u64 = 8 * 1024 * 1024;
pub const DEFAULT_PARALLEL_WORKERS: u8 = 4;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ControlMessage {
    Offer {
        transfer_id: String,
        total_bytes: u64,
    },
    Accept {
        transfer_id: String,
    },
    Reject {
        transfer_id: String,
        reason: String,
    },
    ChunkRequest {
        transfer_id: String,
        file_index: usize,
        chunk_index: u64,
    },
    ChunkComplete {
        transfer_id: String,
        file_index: usize,
        chunk_index: u64,
        blake3: String,
    },
    Pause {
        transfer_id: String,
    },
    Resume {
        transfer_id: String,
    },
    Cancel {
        transfer_id: String,
    },
    Complete {
        transfer_id: String,
    },
}
