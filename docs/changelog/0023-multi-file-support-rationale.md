# 0023-multi-file-support-rationale.md

## Context

The current implementation is restricted to a single source file per gRPC request. This prevents the use of modular code architectures, custom header files, and shared libraries.

## Decision

- **Protocol Evolution:** Move from a single `source_code` string to a `map<string, string>` or a `repeated` collection of files in the gRPC `ComputeRequest`.
- **Workspace Reconstruction:** The Host must be updated to iterate through all incoming files and recreate the directory tree before invoking `nvcc`.

## Key Considerations

- **Include Paths:** Ensuring the Host passes correct `-I` (include) flags to `nvcc` so it looks in the local scratch directory for headers.
- **Bandwidth:** Sending multiple files increases the payload size, necessitating efficient serialization.
