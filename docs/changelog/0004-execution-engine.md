# Decision 04: Host Execution Logic (MVP)

## Context

The host needs to transform raw strings of CUDA code into running GPU kernels and report progress in real-time.

## Decision

- **Concurrency:** Used `tokio::process::Command` to run `nvcc` asynchronously. This prevents the entire server from hanging while one person compiles.
- **Real-time Feedback:** Used `tokio::sync::mpsc` channels to pipe compiler `stderr` into the gRPC stream.
- **Job Isolation (Initial):** Using UUID-named files in the local directory. (Note: This is temporary; Phase 2 will require actual sandboxing).

## Key Considerations

- **Security:** Currently, any code sent can run `system()` calls on the host. This is acceptable for a local-network MVP but must be fixed before public use.
- **Platform:** Using `nvcc` via `std::process` ensures compatibility with both Linux and Windows, provided `nvcc` is in the system PATH.
