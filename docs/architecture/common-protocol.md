# 📄 Architecture: Protocol Definitions

**File:** [crates/common/proto/compute.proto](/crates/common/proto/compute.proto)

This file is the source of truth for the entire project. Written in language-agnostic Protobuf, it ensures that a macOS client and a Windows/Linux host can communicate even when built with different toolchains.

---

## The Service: `CUDAExecutor`

The service defines two Remote Procedure Call (RPC) methods:

### `ExecuteCode` — Server-Streaming Response

```protobuf
rpc ExecuteCode(ComputeRequest) returns (stream ComputeResponse);
```

The client sends one batch of data (the full file map and entry point), and the server responds with a continuous **stream** of messages. This is vital so the user sees compiler warnings and `printf` outputs as they happen, rather than waiting for the entire job to finish.

### `GetGpuStatus` — Unary

```protobuf
rpc GetGpuStatus(Empty) returns (GpuStatus);
```

A simple unary (request-response) call that returns a snapshot of the remote GPU's hardware telemetry. The client sends an `Empty` message (no payload needed) and receives a single `GpuStatus` response.

---

## The Message: `ComputeRequest`

This is the payload sent from the local machine to the remote GPU server.

```protobuf
message ComputeRequest {
  map<string, string> files = 1;
  string entry_point_file = 2;
  repeated string compiler_flags = 3;
}
```

| Field | Type | Purpose |
|---|---|---|
| `files` | `map<string, string>` | A key-value map of **relative path → file content** for every file in the project (e.g., `"include/utils.cuh" → "..."`, `"main.cu" → "..."`). |
| `entry_point_file` | `string` | The relative path of the file that `nvcc` should compile as the main translation unit (e.g., `"main.cu"`). Must be a key present in `files`. |
| `compiler_flags` | `repeated string` | User-supplied flags forwarded verbatim to `nvcc` (e.g., `["-arch=sm_80", "-O3"]`). |

Using `map<string, string>` for `files` replaces the old single `source_code` + `file_name` approach. This enables **project-level workspace sync** — the entire directory tree, including headers and sub-directories, is transferred in one RPC call. The Host reconstructs the tree verbatim before invoking `nvcc`, preserving all relative include paths.

---

## The Message: `ComputeResponse`

This is the data packet streamed from the Host back to the Client.

```protobuf
message ComputeResponse {
    string output = 1;
    bool is_error = 2;
}
```

| Field | Purpose |
|---|---|
| `output` | A single line or chunk of text — a compiler warning, a status update, or the executed program's output. |
| `is_error` | If `true`, the client renders the text in **red** to signify `stderr`, a compilation failure, or an internal error. |

---

## The Message: `GpuStatus`

Returned by `GetGpuStatus`. Contains a real-time snapshot of the GPU hardware, sourced from `nvidia-smi`.

```protobuf
message GpuStatus {
    string gpu_name = 1;
    uint32 temperature_celsius = 2;
    uint32 memory_used_mb = 3;
    uint32 memory_total_mb = 4;
    uint32 load_percentage = 5;
}
```

All numeric values are `uint32` since hardware metrics are never negative and fit comfortably within 32-bit unsigned integers.

---

## The Message: `Empty`

```protobuf
message Empty {}
```

A zero-field message used as the request body for `GetGpuStatus`. Protobuf 3 has no native `void` type, so `Empty` serves as a no-payload sentinel. (The well-known `google.protobuf.Empty` could be used, but defining it locally keeps the dependency surface minimal.)

---

## Why Use a Stream for `ExecuteCode`?

Without a stream, the client would send the code and sit in silence while the server compiles and runs it — potentially 10+ seconds. With a stream, as soon as `nvcc` prints its first line, the Host pushes that line to the Client immediately. The CLI feels interactive rather than frozen.

The unary `GetGpuStatus` does not need streaming because the response is a single atomic snapshot with no time-varying parts.

---

### Protobuf-to-Rust Name Mapping

`tonic-build` automatically converts Protobuf naming conventions to Rust conventions:

| Protobuf | Generated Rust | Convention |
|---|---|---|
| `service CUDAExecutor` | `mod cuda_executor_server` / `mod cuda_executor_client` | snake_case modules |
| `rpc ExecuteCode(...)` | `fn execute_code(...)` | snake_case function |
| `rpc GetGpuStatus(...)` | `fn get_gpu_status(...)` | snake_case function |
| `message ComputeRequest` | `struct ComputeRequest` | PascalCase (unchanged — already Rust convention) |
| `message GpuStatus` | `struct GpuStatus` | PascalCase |
| `map<string, string> files` | `files: HashMap<String, String>` | Standard Rust map |

So `client.execute_code(request)` in the client maps to protobuf's `ExecuteCode` RPC, which dispatches to the `execute_code` trait method on `HostExecutor`.
