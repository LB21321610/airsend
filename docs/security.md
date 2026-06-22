# Airsend Security Notes

## MVP Security

- No cloud service, relay server, or account system.
- First connection requires device pairing confirmation.
- Trusted peers are stored locally by fingerprint.
- Pairing confirmation codes are derived from local and remote fingerprints and displayed as six digits.
- Transfer sessions use temporary TLS/session encryption.
- Public network interfaces are rejected before binding.

## Deferred Hardening

- AES-256-GCM per-chunk file encryption.
- Key rotation for long transfers.
- Certificate pinning over persisted peer fingerprints.
- Signed discovery announcements.
- Stronger replay protection for UDP heartbeat.

## Firewall UX

Airsend should explain OS firewall prompts in plain language:

- macOS: allow incoming network connections for local Wi-Fi transfers.
- Windows: allow private networks only.
- Linux: check local firewall rules if peers cannot connect.

The app should fail visibly if binding is denied instead of silently appearing offline.
