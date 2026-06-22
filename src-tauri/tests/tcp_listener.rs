use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use app_lib::platform::interfaces::bind_guarded_tcp_listener;

#[tokio::test]
async fn guarded_tcp_listener_refuses_public_bind_ip_before_binding() {
    let err = bind_guarded_tcp_listener(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 0)
        .await
        .expect_err("public listener bind should be rejected");

    assert!(err.to_string().contains("not a private LAN bind target"));
}

#[tokio::test]
async fn guarded_tcp_listener_refuses_ipv6_link_local_without_scope() {
    let err = bind_guarded_tcp_listener(
        IpAddr::V6("fe80::1".parse::<Ipv6Addr>().expect("valid link local")),
        0,
    )
    .await
    .expect_err("unscoped link-local listener bind should be rejected");

    assert!(err.to_string().contains("requires an interface scope"));
}
