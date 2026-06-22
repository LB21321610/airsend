use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use app_lib::platform::interfaces::{guard_bind_ip, is_allowed_bind_ip, is_private_lan_ip};

#[test]
fn private_ipv4_ranges_are_lan_addresses() {
    assert!(is_private_lan_ip(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3))));
    assert!(is_private_lan_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 9))));
    assert!(is_private_lan_ip(IpAddr::V4(Ipv4Addr::new(
        192, 168, 1, 44
    ))));
}

#[test]
fn public_and_loopback_ipv4_are_not_lan_bind_targets() {
    assert!(!is_allowed_bind_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    assert!(!is_allowed_bind_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
    assert!(!is_allowed_bind_ip(IpAddr::V4(Ipv4Addr::UNSPECIFIED)));
}

#[test]
fn guard_bind_ip_returns_an_error_for_public_targets() {
    let err = guard_bind_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)))
        .expect_err("public IP should not be accepted");

    assert!(err.to_string().contains("not a private LAN bind target"));
}

#[test]
fn ipv6_link_local_is_allowed_but_global_and_loopback_are_rejected() {
    assert!(is_allowed_bind_ip(IpAddr::V6(
        "fe80::1".parse::<Ipv6Addr>().expect("valid link-local")
    )));
    assert!(!is_allowed_bind_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    assert!(!is_allowed_bind_ip(IpAddr::V6(
        "2606:4700:4700::1111"
            .parse::<Ipv6Addr>()
            .expect("valid global")
    )));
}
