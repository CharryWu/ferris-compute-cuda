# 0023-multi-file-workspace-and-telemetry.md

## Context

Standardized the transition from "Snippet Execution" to "Project Execution." Required a mechanism to sync headers and multiple source files while providing hardware visibility via telemetry.

## Decision

- **Protocol Evolution:** Updated `ComputeRequest` to utilize a `map<string, string>` for bulk file transfer and added an `entry_point_file` field for targeted compilation.
- **Recursive Workspace Sync:** Implemented `gather_files_recursive` in the Client to allow directory-based inputs (e.g., `.`) and `prepare_workspace` in the Host to reconstruct directory trees.
- **Smart Linking:** Configured the Host to automatically inject the `-rdc=true` (Relocatable Device Code) flag when multiple files are detected, enabling cross-file kernel calls.
- **Hardware Telemetry:** Introduced the `GetGpuStatus` gRPC method and a dedicated `handle_status` client function to report remote GPU temperature, memory, and load.
- **Thread-Safe Error Handling:** Adopted `anyhow` for Host utilities to ensure `Send + Sync` compatibility for errors generated within `tokio::spawn` blocks.

## Key Considerations

- **UX Simplification:** Removed the need for manual file-type flags; providing a directory automatically triggers project sync mode.
- **Reliability:** UUID-based scratch directories are now purged immediately following the termination of the execution stream, regardless of success or failure.
