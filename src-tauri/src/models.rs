use serde::{Deserialize, Serialize};

use crate::storage::json_store::{DeviceIdentity, TransferHistoryRecord};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub device_id: String,
    pub display_name: String,
    pub fingerprint: String,
    pub protocol_version: u16,
    pub app_version: String,
    pub control_port: Option<u16>,
    pub online: bool,
    pub trusted: bool,
    pub last_seen: Option<String>,
    pub addresses: Vec<String>,
}

impl From<DeviceIdentity> for Device {
    fn from(identity: DeviceIdentity) -> Self {
        Self {
            device_id: identity.device_id,
            display_name: identity.display_name,
            fingerprint: identity.fingerprint,
            protocol_version: 1,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            control_port: None,
            online: true,
            trusted: true,
            last_seen: None,
            addresses: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub save_directory: Option<String>,
    pub auto_accept: bool,
    pub parallel_workers: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub local_device: Device,
    pub devices: Vec<Device>,
    pub settings: Settings,
    pub transfer_history: Vec<TransferHistoryRecord>,
    pub interfaces: Vec<crate::platform::interfaces::NetworkInterfaceSummary>,
    pub firewall_guidance: crate::platform::firewall::FirewallGuidance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendFilesRequest {
    pub target_device_id: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferOffer {
    pub transfer_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub root_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgressEvent {
    pub transfer_id: String,
    pub status: String,
    pub transferred_bytes: u64,
    pub total_bytes: u64,
    pub bytes_per_second: u64,
}
