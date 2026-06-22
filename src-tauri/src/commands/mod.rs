use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State};

use crate::app_state::AppState;
use crate::models::{AppSnapshot, Device, SendFilesRequest, TransferProgressEvent};
use crate::security::{pairing_code_for_fingerprints, PairingCode};
use crate::storage::json_store::{
    TransferDirection, TransferHistoryRecord, TransferStatus, TrustedPeer,
};
use crate::transfer::build_transfer_manifest;

#[tauri::command]
pub fn get_app_snapshot(state: State<'_, AppState>) -> Result<AppSnapshot, String> {
    state.snapshot()
}

#[tauri::command]
pub fn get_local_device(state: State<'_, AppState>) -> Result<Device, String> {
    Ok(state.snapshot()?.local_device)
}

#[tauri::command]
pub fn list_devices(state: State<'_, AppState>) -> Result<Vec<Device>, String> {
    state.list_devices()
}

#[tauri::command]
pub fn pairing_code(
    state: State<'_, AppState>,
    remote_fingerprint: String,
) -> Result<PairingCode, String> {
    let local = state.snapshot()?.local_device;
    Ok(pairing_code_for_fingerprints(
        &local.fingerprint,
        &remote_fingerprint,
    ))
}

#[tauri::command]
pub fn trust_peer(
    state: State<'_, AppState>,
    device_id: String,
    display_name: String,
    fingerprint: String,
) -> Result<AppSnapshot, String> {
    let mut store = state.lock_store()?;
    if !store
        .data()
        .trusted_peers
        .iter()
        .any(|peer| peer.fingerprint == fingerprint)
    {
        store.data_mut().trusted_peers.push(TrustedPeer {
            device_id,
            display_name,
            fingerprint,
            trusted_at: now_timestamp(),
        });
        store.save().map_err(|err| err.to_string())?;
    }
    drop(store);
    state.snapshot()
}

#[tauri::command]
pub fn set_save_directory(
    state: State<'_, AppState>,
    save_directory: Option<String>,
) -> Result<AppSnapshot, String> {
    let mut store = state.lock_store()?;
    store.data_mut().settings.save_directory = save_directory;
    store.save().map_err(|err| err.to_string())?;
    drop(store);
    state.snapshot()
}

#[tauri::command]
pub fn set_auto_accept(
    state: State<'_, AppState>,
    auto_accept: bool,
) -> Result<AppSnapshot, String> {
    let mut store = state.lock_store()?;
    store.data_mut().settings.auto_accept = auto_accept;
    store.save().map_err(|err| err.to_string())?;
    drop(store);
    state.snapshot()
}

#[tauri::command]
pub fn accept_transfer(_state: State<'_, AppState>, transfer_id: String) -> Result<String, String> {
    Ok(format!("accepted:{transfer_id}"))
}

#[tauri::command]
pub fn reject_transfer(_state: State<'_, AppState>, transfer_id: String) -> Result<String, String> {
    Ok(format!("rejected:{transfer_id}"))
}

#[tauri::command]
pub fn send_files(
    app: AppHandle,
    state: State<'_, AppState>,
    request: SendFilesRequest,
) -> Result<AppSnapshot, String> {
    if request.paths.is_empty() {
        return Err("Select at least one file or folder to send.".to_string());
    }

    let transfer_id = uuid::Uuid::new_v4().to_string();
    let manifest = build_transfer_manifest(request.paths.iter().map(PathBuf::from).collect())
        .map_err(|err| err.to_string())?;
    let file_count = manifest.entries.len();
    let total_bytes = manifest.total_bytes;
    let local = state.snapshot()?.local_device;
    if request.target_device_id == local.device_id {
        return Err("Select a nearby peer before queueing a transfer.".to_string());
    }

    {
        let mut store = state.lock_store()?;
        store.data_mut().transfer_history.insert(
            0,
            TransferHistoryRecord {
                transfer_id: transfer_id.clone(),
                peer_id: request.target_device_id,
                peer_name: "Selected peer".to_string(),
                direction: TransferDirection::Outgoing,
                file_count,
                total_bytes,
                status: TransferStatus::Queued,
                created_at: now_timestamp(),
                completed_at: None,
            },
        );
        store.save().map_err(|err| err.to_string())?;
    }

    let _ = app.emit(
        "transfer-progress",
        TransferProgressEvent {
            transfer_id,
            status: format!("Queued by {}", local.display_name),
            transferred_bytes: 0,
            total_bytes,
            bytes_per_second: 0,
        },
    );

    state.snapshot()
}

fn now_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
