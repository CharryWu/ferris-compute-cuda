# Decision 0016: Modular Utility Refactoring

## Context

As the Host logic grew to include complex environment discovery (vswhere, path traversal), `main.rs` was becoming cluttered. This made the core gRPC server logic harder to maintain.

## Decision

- **Modularization:** Moved all toolchain discovery and helper functions to `crates/host/src/utils.rs`.
- **Documentation:** Implemented Rust-standard docstrings (`///`) for utility functions to support `cargo doc` generation.
- **Visibility:** Marked helper functions as `pub` within the host crate to allow access from the gRPC task handlers.

## Key Considerations

- **Separation of Concerns:** `main.rs` now handles the "How" (streaming, networking), while `utils.rs` handles the "Where" (system paths, environment setup).
- **Scalability:** This structure allows us to add further utilities (like GPU health checks or telemetry) without bloating the networking code.
