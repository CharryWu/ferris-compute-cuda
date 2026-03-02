# Decision 0009: Network Binding and Firewall Configuration

## Context

Initial development used `[::1]` (IPv6 loopback), which restricted the Host to only accepting connections from the same machine. To allow the macOS client to communicate with the Windows/Linux GPU server over a local network (e.g., `10.0.0.x`), the networking configuration had to be expanded.

## Decision

- **Host Binding:** Changed the listener address from `[::1]:50051` to `0.0.0.0:50051`.
- **URL Schema:** Standardized the client connection string to use the `http://` prefix, which is required by the `tonic` crate to initialize the HTTP/2 handshake.
- **Firewall:** Opened an Inbound TCP rule for Port 50051 on the Host machine.

## Key Considerations

- **IP Selection:** Using `0.0.0.0` tells the operating system to listen on all available network interfaces (WiFi, Ethernet, and Loopback).
- **Security Implications:** This transition marks the project's move from "local-only" to "network-accessible." It introduces the risk of unauthorized access to the GPU, highlighting the urgency for Authentication and Sandboxing in the next development phase.
