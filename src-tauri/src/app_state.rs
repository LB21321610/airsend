use std::path::PathBuf;
use std::sync::Mutex;

use crate::discovery::PeerTable;
use crate::models::{AppSnapshot, Device, Settings};
use crate::platform::{firewall, interfaces};
use crate::storage::json_store::AirsendStore;

pub struct AppState {
    store: Mutex<AirsendStore>,
    peers: Mutex<PeerTable>,
}

impl AppState {
    pub fn load(store_path: PathBuf) -> Result<Self, String> {
        let store = AirsendStore::load_or_create(store_path).map_err(|err| err.to_string())?;
        let local_device_id = store.data().device.device_id.clone();

        Ok(Self {
            store: Mutex::new(store),
            peers: Mutex::new(PeerTable::new(local_device_id)),
        })
    }

    pub fn snapshot(&self) -> Result<AppSnapshot, String> {
        let store = self.lock_store()?;
        let data = store.data().clone();
        let mut local_device = Device::from(data.device);
        local_device.addresses = interfaces::allowed_lan_interfaces()
            .into_iter()
            .filter(|iface| iface.allowed_for_bind)
            .map(|iface| iface.ip)
            .collect();

        let trusted_peers = data.trusted_peers.clone();
        let devices = self
            .peers
            .lock()
            .map_err(|_| "peer table lock poisoned".to_string())?
            .peers()
            .into_iter()
            .map(|mut peer| {
                peer.device.trusted = trusted_peers
                    .iter()
                    .any(|trusted| trusted.fingerprint == peer.device.fingerprint);
                peer.device
            })
            .collect();

        Ok(AppSnapshot {
            local_device,
            devices,
            settings: Settings {
                save_directory: data.settings.save_directory,
                auto_accept: data.settings.auto_accept,
                parallel_workers: data.settings.parallel_workers,
            },
            transfer_history: data.transfer_history,
            interfaces: interfaces::allowed_lan_interfaces(),
            firewall_guidance: firewall::default_firewall_guidance(),
        })
    }

    pub fn lock_store(&self) -> Result<std::sync::MutexGuard<'_, AirsendStore>, String> {
        self.store
            .lock()
            .map_err(|_| "store lock poisoned".to_string())
    }

    pub fn list_devices(&self) -> Result<Vec<Device>, String> {
        let peers = self
            .peers
            .lock()
            .map_err(|_| "peer table lock poisoned".to_string())?;
        Ok(peers.peers().into_iter().map(|peer| peer.device).collect())
    }
}
