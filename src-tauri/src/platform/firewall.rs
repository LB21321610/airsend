#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirewallGuidance {
    pub title: &'static str,
    pub message: &'static str,
    pub platform_hint: &'static str,
}

pub fn default_firewall_guidance() -> FirewallGuidance {
    #[cfg(target_os = "macos")]
    let platform_hint = "macOS may ask whether Airsend can accept incoming network connections. Allow it for local Wi-Fi transfers.";

    #[cfg(target_os = "windows")]
    let platform_hint = "Windows Defender Firewall may ask for private network access. Allow private networks only.";

    #[cfg(target_os = "linux")]
    let platform_hint = "If peers cannot connect, check that your local firewall allows Airsend on private LAN interfaces.";

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    let platform_hint = "If peers cannot connect, check that your local firewall allows Airsend on private LAN interfaces.";

    FirewallGuidance {
        title: "Local network access needed",
        message: "Airsend listens only on private LAN interfaces and never exposes the control port on public addresses.",
        platform_hint,
    }
}
