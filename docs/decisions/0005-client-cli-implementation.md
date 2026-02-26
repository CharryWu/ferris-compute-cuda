# Decision 0005: Client CLI Implementation

## Context

The user needs a way to interact with the remote system from their local terminal (macOS). The experience should feel like running a local command (e.g., `nvcc`), even though the work is happening elsewhere.

## Decision

- **CLI Framework:** Used `clap` with the `derive` feature for a declarative argument parser.
- **UX Feedback:** Integrated the `colored` crate to differentiate between Host errors (stderr) and successful output (stdout).
- **Network Logic:** Used `tonic`'s auto-generated client to establish a gRPC connection and handle the server-side stream.

## Key Considerations

- **File Handling:** The client reads the entire file into a string for the MVP. For Phase 2 (Distributed), we may need to implement multi-file bundling (tar/zip) or chunked streaming for very large projects.
- **Connection Defaults:** Defaulted to `localhost` ([::1]) to allow the user to test the system locally before deploying to a physical GPU server.
