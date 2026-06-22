# Airsend Local P2P File Transfer Plan

## Summary

Airsend is a local-network file transfer app built with Tauri v2, React, and Rust. The MVP focuses on a runnable desktop shell, JSON-backed local state, private-LAN safety checks, Tauri commands, and the protocol boundaries needed for discovery, pairing, and resumable chunked transfer.

The first target is macOS, with Rust and TypeScript boundaries kept portable for Windows and Linux.

## Architecture

- Desktop app: Tauri v2 + React + TypeScript.
- Rust core: Tokio-ready modules for discovery, transfer, protocol, security, JSON storage, and network interface safety.
- Discovery: mDNS service name `_airsend._tcp.local.` with UDP heartbeat fallback planned behind the same discovery boundary.
- Discovery payloads include app version, device ID, display name, fingerprint, protocol version, and control port; peer tracking ignores self-announcements and marks stale peers offline.
- Transfer: TCP control messages, recursive file/folder manifests, resumable chunk planning, destination conflict handling, guarded listener entrypoints, and a tested loopback TCP send/receive path with BLAKE3 verification. MVP defaults to 8 MiB chunks and 4 workers.
- Security: trusted-device pairing and TLS/session encryption first. AES-256-GCM chunk encryption is a post-MVP hardening pass.
- Storage: one serde JSON file named `airsend.json` in the Tauri app data directory. SQLite is intentionally deferred.

## MVP Tasks

1. Bootstrap Tauri v2 + React and use v2 plugin packages/capabilities.
2. Persist settings, trusted peers, transfer history, and partial manifests in JSON.
3. Reject public, wildcard, and loopback binding targets for transfer services.
4. Expose Tauri commands for app snapshot, devices, settings, send, accept, and reject.
5. Replace starter UI with a dark AirDrop-style device and transfer dashboard.
6. Add tests for JSON recovery, atomic save, private-LAN guard behavior, discovery payloads, stale peer pruning, pairing codes, manifest expansion, chunk planning, resume planning, destination conflict handling, path traversal rejection, and symlink rejection.

## Post-MVP Tasks

- Real socket loop for mDNS publisher/browser and UDP heartbeat runtime.
- TLS handshakes, pairing approval flow, and peer fingerprint verification.
- Multi-connection chunk transfer engine and resume-aware scheduling.
- AES-256-GCM per-chunk encryption.
- Large-file stress testing, adaptive buffers, and platform-specific firewall UX.
- Mobile extension for iOS and Android.

## Acceptance Scenarios

- Missing or corrupt `airsend.json` recovers safely.
- Public and wildcard addresses are refused as bind targets.
- Discovery announcements round-trip with protocol metadata.
- Self-announcements are ignored and stale peers are marked offline.
- Pairing codes are stable six-digit codes and trusted peers persist.
- Frontend can load local device, interfaces, settings, and history through Tauri.
- User can choose files/folders, queue a transfer, and see it in history.
- Folder manifests preserve relative paths.
- Chunk planning skips verified chunks for resume.
- Resume state keys verified chunks by file index and chunk index.
- Destination paths reject traversal and absolute paths.
- Symlinks are rejected for the MVP manifest.
- A file can be sent over TCP loopback and written to the receiver directory with BLAKE3 verification.
- App builds with Tauri v2 plugin dependencies and capabilities.
