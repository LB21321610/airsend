use crate::models::Device;

pub const SERVICE_NAME: &str = "_airsend._tcp.local.";
pub const PROTOCOL_VERSION: u16 = 1;
pub const DEFAULT_PEER_TTL_SECONDS: u64 = 30;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryAnnouncement {
    pub service_name: String,
    pub protocol_version: u16,
    pub device: Device,
    pub control_port: u16,
}

impl DiscoveryAnnouncement {
    pub fn new(mut device: Device, control_port: u16) -> Self {
        device.control_port = Some(control_port);
        Self {
            service_name: SERVICE_NAME.to_string(),
            protocol_version: PROTOCOL_VERSION,
            device,
            control_port,
        }
    }

    pub fn to_udp_payload(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn from_udp_payload(payload: &[u8]) -> Result<Self, DiscoveryError> {
        let announcement: DiscoveryAnnouncement = serde_json::from_slice(payload)?;
        if announcement.service_name != SERVICE_NAME {
            return Err(DiscoveryError::WrongService(announcement.service_name));
        }
        if announcement.protocol_version != PROTOCOL_VERSION {
            return Err(DiscoveryError::UnsupportedProtocol(
                announcement.protocol_version,
            ));
        }
        Ok(announcement)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryPeer {
    pub device: Device,
    pub last_seen_epoch_seconds: u64,
}

#[derive(Debug, Default)]
pub struct PeerTable {
    local_device_id: String,
    peers: Vec<DiscoveryPeer>,
}

impl PeerTable {
    pub fn new(local_device_id: String) -> Self {
        Self {
            local_device_id,
            peers: Vec::new(),
        }
    }

    pub fn apply_announcement(
        &mut self,
        announcement: DiscoveryAnnouncement,
        now_epoch_seconds: u64,
    ) {
        if announcement.device.device_id == self.local_device_id {
            return;
        }

        match self
            .peers
            .iter_mut()
            .find(|peer| peer.device.device_id == announcement.device.device_id)
        {
            Some(peer) => {
                peer.device = announcement.device;
                peer.device.online = true;
                peer.last_seen_epoch_seconds = now_epoch_seconds;
            }
            None => self.peers.push(DiscoveryPeer {
                device: announcement.device,
                last_seen_epoch_seconds: now_epoch_seconds,
            }),
        }
    }

    pub fn prune_expired(&mut self, now_epoch_seconds: u64, ttl_seconds: u64) {
        prune_expired_peers(&mut self.peers, now_epoch_seconds, ttl_seconds);
    }

    pub fn peers(&self) -> Vec<DiscoveryPeer> {
        self.peers.clone()
    }
}

pub fn prune_expired_peers(peers: &mut [DiscoveryPeer], now_epoch_seconds: u64, ttl_seconds: u64) {
    for peer in peers {
        let elapsed = now_epoch_seconds.saturating_sub(peer.last_seen_epoch_seconds);
        if elapsed > ttl_seconds {
            peer.device.online = false;
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("discovery payload is for a different service: {0}")]
    WrongService(String),
    #[error("unsupported discovery protocol version: {0}")]
    UnsupportedProtocol(u16),
}
