# Decision 0029: Windows Local Discovery Fallback

## Context

On Windows, `client discover` could fail to find a host even when both client and host were running on the same machine. Runtime tracing showed mDNS browse startup succeeded, but no `ServiceResolved` events were emitted for the local host within the scan window.

## Decision

Added a local loopback fallback in the client discovery path:

- The client first performs the normal mDNS browse for `_ferris-compute._tcp.local.`.
- If no hosts are resolved, the client probes `127.0.0.1:50051` with a short TCP timeout.
- If the probe succeeds, discovery returns a synthetic local entry:
  - Address: `127.0.0.1`
  - Hostname: `localhost`
  - Port: `50051`

## Rationale

- Preserves existing LAN discovery behavior.
- Fixes the practical same-machine workflow on Windows where mDNS may not resolve self-advertised services reliably.
- Keeps behavior deterministic for local development and smoke testing.

## Scope

- File changed: `crates/client/src/discovery.rs`
- No protocol or host-side behavior changes.

## Risks / Trade-offs

- Fallback is currently tied to the default host port (`50051`).
- A successful loopback probe only confirms something is listening on that port; authentication and RPC compatibility are still validated later by normal client commands.
