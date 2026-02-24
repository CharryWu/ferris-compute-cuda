# Decision 01: Initial Project Setup & Workspace

## Context

I am building a remote CUDA executor with a macOS client and Windows/Linux hosts. I need a structure that supports multiple binaries and shared logic.

## Decision

- **Structure:** Cargo Workspace (Monorepo).
- **Edition:** Rust 2024 (latest).
- **Communication:** gRPC (Tonic/Prost) for streaming support.
- **Crate Breakdown:**
  - `client`: macOS CLI.
  - `host`: Remote GPU worker.
  - `common`: Shared Protobuf and networking logic.

## Key Considerations

- Avoided `src/` at the root to prevent dependency bleed.
- Used `resolver = "2"` to handle different OS-specific dependencies.
- Opted for gRPC over JSON to allow real-time log streaming from `nvcc`.
