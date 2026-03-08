# 0022-phase-3-telemetry-init.md

## Context

As the project transitioned from basic execution to a managed remote service, users needed insight into the remote GPU's health and a more sophisticated CLI to manage different types of requests.

## Decision

- **Protocol Expansion:** Added `GetGpuStatus` to the gRPC service to allow remote monitoring of hardware vitals.
- **SMI Integration:** Implemented a Host-side utility to parse `nvidia-smi` output for real-time temperature, memory, and load metrics.
- **CLI Refactoring:** Converted the Client `Args` into a `Subcommand` enum, separating the logic for `run` (job submission) and `status` (telemetry).
- **Modularization:** Encapsulated telemetry logic into a dedicated `handle_status` function with specific gRPC metadata injection and human-readable unit formatting.

## Key Considerations

- **Architecture:** The use of subcommands prevents "illegal states" where a user might be asked for a CUDA file path when they only want to check the GPU temperature.
- **Extensibility:** The new structure allows Phase 3 features (like multi-file support) to be added as new variants or flags without breaking existing code.
