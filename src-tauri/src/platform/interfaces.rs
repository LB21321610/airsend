use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterfaceSummary {
    pub name: String,
    pub ip: String,
    pub allowed_for_bind: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{0} is not a private LAN bind target")]
pub struct BindGuardError(pub IpAddr);

pub fn is_private_lan_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_private_ipv4(v4),
        IpAddr::V6(v6) => is_link_local_ipv6(v6),
    }
}

pub fn is_allowed_bind_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_private_ipv4(v4) && !v4.is_loopback() && !v4.is_unspecified(),
        IpAddr::V6(v6) => is_link_local_ipv6(v6) && !v6.is_loopback() && !v6.is_unspecified(),
    }
}

pub fn guard_bind_ip(ip: IpAddr) -> Result<IpAddr, BindGuardError> {
    if is_allowed_bind_ip(ip) {
        Ok(ip)
    } else {
        Err(BindGuardError(ip))
    }
}

pub async fn bind_guarded_tcp_listener(
    ip: IpAddr,
    port: u16,
) -> Result<tokio::net::TcpListener, BindTcpListenerError> {
    let guarded_ip = guard_bind_ip(ip).map_err(BindTcpListenerError::Guard)?;
    if matches!(guarded_ip, IpAddr::V6(_)) {
        return Err(BindTcpListenerError::UnscopedIpv6LinkLocal);
    }
    tokio::net::TcpListener::bind(SocketAddr::new(guarded_ip, port))
        .await
        .map_err(BindTcpListenerError::Bind)
}

#[derive(Debug, thiserror::Error)]
pub enum BindTcpListenerError {
    #[error(transparent)]
    Guard(#[from] BindGuardError),
    #[error("IPv6 link-local listener binding requires an interface scope")]
    UnscopedIpv6LinkLocal,
    #[error("failed to bind local TCP listener: {0}")]
    Bind(std::io::Error),
}

pub fn allowed_lan_interfaces() -> Vec<NetworkInterfaceSummary> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .map(|iface| {
            let ip = iface.ip();
            NetworkInterfaceSummary {
                name: iface.name,
                ip: ip.to_string(),
                allowed_for_bind: is_allowed_bind_ip(ip),
            }
        })
        .collect()
}

fn is_private_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    match octets {
        [10, _, _, _] => true,
        [172, second, _, _] => (16..=31).contains(&second),
        [192, 168, _, _] => true,
        [169, 254, _, _] => true,
        _ => false,
    }
}

fn is_link_local_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    (segments[0] & 0xffc0) == 0xfe80
}
