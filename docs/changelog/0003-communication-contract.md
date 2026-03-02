# Decision 0003: Communication Contract (gRPC vs JSON)

## Context

The Client needs to send source code and flags to the Host, and the Host needs to return logs and execution results. We need a communication layer that is efficient and supports real-time feedback.

## Decision

- **Protocol:** gRPC (via the `tonic` crate).
- **Serialization:** Protocol Buffers (proto3).
- **Pattern:** Server-Side Streaming.

## Key Considerations

- **Why gRPC over JSON?** - **Streaming:** JSON typically requires a "request-response" cycle where the server only sends data once the job is finished. gRPC allows the Host to stream `stdout/stderr` back to the Client line-by-line while `nvcc` is still running.
  - **Type Safety:** Protobuf generates Rust structs automatically, ensuring the Client and Host never disagree on the data format.
- **Crate Separation:** All generated code lives in `crates/common` so that both the Client and Host share the exact same API definition.
