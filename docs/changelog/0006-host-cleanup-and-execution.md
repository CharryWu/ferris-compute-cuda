# Decision 0006: Host Cleanup and Execution

## Context

Running remote CUDA code generates multiple artifacts (source files, object files, executables). Without a cleanup strategy, the host server's disk would eventually exhaust.

## Decision

- **Scoped Workspaces:** Each job is assigned a unique UUID and its own subdirectory within a `scratch/` folder.
- **Synchronous Cleanup:** The host uses `tokio::fs::remove_dir_all` at the end of the spawned task's lifecycle.
- **Binary Execution:** After successful compilation, the host executes the resulting binary and captures both `stdout` and `stderr` to return to the client.

## Key Considerations

- **Platform Agnostic Binaries:** Added a check for `cfg!(windows)` to handle `.exe` extensions, ensuring the host runs on both Linux and Windows.
- **Atomic Cleanup:** By putting all job files in a single UUID-named folder, we can delete everything in one command rather than tracking individual files.
