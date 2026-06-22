use app_lib::discovery::{
    prune_expired_peers, DiscoveryAnnouncement, DiscoveryPeer, PeerTable, PROTOCOL_VERSION,
    SERVICE_NAME,
};
use app_lib::models::Device;

fn device(id: &str) -> Device {
    Device {
        device_id: id.into(),
        display_name: format!("Device {id}"),
        fingerprint: format!("fingerprint-{id}"),
        protocol_version: PROTOCOL_VERSION,
        app_version: "0.1.0".into(),
        control_port: Some(49152),
        online: true,
        trusted: false,
        last_seen: None,
        addresses: vec!["192.168.1.20".into()],
    }
}

#[test]
fn discovery_announcement_round_trips_with_protocol_metadata() {
    let announcement = DiscoveryAnnouncement::new(device("peer-1"), 49152);

    let encoded = announcement
        .to_udp_payload()
        .expect("payload should serialize");
    let decoded = DiscoveryAnnouncement::from_udp_payload(&encoded).expect("payload should decode");

    assert_eq!(decoded.service_name, SERVICE_NAME);
    assert_eq!(decoded.protocol_version, PROTOCOL_VERSION);
    assert_eq!(decoded.device.device_id, "peer-1");
    assert_eq!(decoded.control_port, 49152);
}

#[test]
fn peer_table_ignores_own_announcement_and_tracks_remote_peer() {
    let mut table = PeerTable::new("local-device".into());
    table.apply_announcement(
        DiscoveryAnnouncement::new(device("local-device"), 49152),
        10,
    );
    table.apply_announcement(DiscoveryAnnouncement::new(device("peer-1"), 49153), 11);

    let peers = table.peers();

    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].device.device_id, "peer-1");
    assert_eq!(peers[0].last_seen_epoch_seconds, 11);
}

#[test]
fn prune_expired_peers_marks_stale_devices_offline() {
    let mut peers = vec![DiscoveryPeer {
        device: device("peer-1"),
        last_seen_epoch_seconds: 10,
    }];

    prune_expired_peers(&mut peers, 45, 30);

    assert!(!peers[0].device.online);
}
