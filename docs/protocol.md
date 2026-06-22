# Airsend Protocol Notes

## Service Discovery

- Service name: `_airsend._tcp.local.`
- Protocol version: `1`
- Announced fields: app version, device ID, display name, fingerprint, protocol version, and control port.
- mDNS is primary. UDP heartbeat is fallback for networks where multicast DNS is blocked or unreliable.
- Peer tables ignore announcements from the local device ID.
- Peers older than the TTL are marked offline instead of immediately deleted.

## Control Messages

Control messages are JSON during MVP so they are easy to inspect and test.

- `offer`: sender proposes a transfer with transfer ID and total bytes.
- `accept`: receiver accepts a transfer.
- `reject`: receiver rejects a transfer with a reason.
- `chunkRequest`: receiver asks for a chunk by file index and chunk index.
- `chunkComplete`: sender/receiver reports verified chunk hash by file index and chunk index.
- `pause`: transfer should stop scheduling new chunks.
- `resume`: transfer should continue from verified chunks.
- `cancel`: transfer should stop and clean up runtime state.
- `complete`: all chunks are verified.

## Chunk Defaults

- Default chunk size: 8 MiB.
- Default parallel workers: 4.
- Chunk integrity: BLAKE3 digest.
- Resume state: `.airsend-partial` manifest plus JSON store summary.
- Folder transfer manifests preserve selected folder names and nested relative paths.
- Existing destination files are not overwritten; Airsend appends a numeric suffix.
- Verified resume chunks are keyed by both file index and chunk index.
- Receiver destination paths reject absolute paths and traversal components.
- Receiver destination paths reject symlinked parent directories and file-as-directory parent conflicts.
- Symlinks are rejected for the MVP instead of being followed.

## Network Safety

The control and transfer listeners must bind only to private LAN IPv4 ranges or IPv6 link-local addresses. They must not bind to public, loopback-only, or wildcard external addresses.

## TCP MVP Path

- The current tested TCP path sends a length-prefixed JSON transfer header followed by file bytes.
- The receiver writes files through the safe destination resolver.
- Each received file is checked against its BLAKE3 digest before the transfer is accepted.
