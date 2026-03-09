# [Build script](/crates/common/build.rs)

This is the "handshake" of the project. Since the macOS client and the Windows/Linux host are built separately and may use different toolchains, Protobuf acts as the universal translator that ensures both sides agree on the exact structure of every message sent over the wire.

---

## 1. Protobuf Breakdown (`compute.proto`)

```protobuf
syntax = "proto3";
package compute;

service CUDAExecutor {
    // Client sends a full file workspace; Host streams back compilation/execution logs.
    rpc ExecuteCode(ComputeRequest) returns (stream ComputeResponse);

    // Unary call to query remote GPU hardware telemetry.
    rpc GetGpuStatus(Empty) returns (GpuStatus);
}

// Payload sent from the Client to the Host.
message ComputeRequest {
    // Key: relative path (e.g., "include/utils.cuh")
    // Value: raw UTF-8 file content
    map<string, string> files = 1;

    // The file nvcc should treat as the main translation unit.
    string entry_point_file = 2;

    // 'repeated' is Protobuf's way of saying "a Vec or List".
    repeated string compiler_flags = 3;
}

// Data streamed back from the Host to the Client.
message ComputeResponse {
    string output = 1;   // Compiler output, status update, or program stdout/stderr.
    bool is_error = 2;   // If true, the client renders this line in red.
}

message Empty {}

message GpuStatus {
    string gpu_name = 1;
    uint32 temperature_celsius = 2;
    uint32 memory_used_mb = 3;
    uint32 memory_total_mb = 4;
    uint32 load_percentage = 5;
}
```

### Why `map<string, string>` for `files`?

Earlier versions sent a single `source_code` string and a `file_name`. This broke for any project with more than one file — headers, utilities, and sub-directory structures couldn't be transmitted. The `map<string, string>` field bundles the entire workspace in one shot:

- **Key** — the relative path of the file, normalized to forward slashes (`/`) so it works on both Windows and Linux.
- **Value** — the raw UTF-8 content of that file.

The Host reconstructs this exact directory tree inside a UUID-named scratch directory before invoking `nvcc`, so all `#include` paths resolve correctly.

### Why `stream` for `ExecuteCode`?

Without a stream, the client would send the code and then sit in silence for 10+ seconds while the server compiles and runs it. With a **stream**, as soon as `nvcc` prints its first line of output, the Host can push that line to the Client immediately. This makes the CLI feel interactive.

`GetGpuStatus` does not need streaming — the response is a single atomic snapshot.

---

## 2. The Build Script (`crates/common/build.rs`)

In Rust, a `build.rs` file is executed **before** the crate is compiled. It tells `tonic-build` to take the `.proto` file and turn it into valid Rust structs, traits, and client/server stubs.

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compiles the .proto file into Rust code, placed in OUT_DIR (/target/...).
    // Keeping generated files out of src/ keeps the source tree clean.
    tonic_build::compile_protos("proto/compute.proto")?;
    Ok(())
}
```

### How it works

1. When you run `cargo build`, Cargo detects `build.rs` and runs it first.
2. `tonic-build` reads `compute.proto`.
3. It generates a file (usually `compute.rs`) containing:
   - `ComputeRequest`, `ComputeResponse`, `GpuStatus`, `Empty` structs.
   - A `CudaExecutorClient` for the client crate.
   - A `CudaExecutorServer` + `CudaExecutor` trait for the host crate.
4. These live in the `/target` folder and are pulled in via `include_proto!`.

---

## 3. Exposing Generated Code

To make the generated code accessible to both the `client` and `host` crates, it is re-exported from the `common` library.

### `crates/common/src/lib.rs`

```rust
pub mod compute {
    tonic::include_proto!("compute");
}
```

---

## 4. Sample Usage

### Client — submitting a multi-file project

```rust
use common::compute::cuda_executor_client::CudaExecutorClient;
use common::compute::ComputeRequest;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = CudaExecutorClient::connect("http://[YOUR_SERVER_IP]:50051").await?;

    let mut files = HashMap::new();
    files.insert("main.cu".into(), include_str!("main.cu").into());
    files.insert("include/utils.cuh".into(), include_str!("include/utils.cuh").into());

    let request = tonic::Request::new(ComputeRequest {
        files,
        entry_point_file: "main.cu".into(),
        compiler_flags: vec!["-arch=sm_75".into()],
    });

    let mut stream = client.execute_code(request).await?.into_inner();

    while let Some(response) = stream.message().await? {
        if response.is_error {
            eprintln!("{}", response.output);
        } else {
            println!("{}", response.output);
        }
    }

    Ok(())
}
```

### Client — querying GPU status

```rust
use common::compute::cuda_executor_client::CudaExecutorClient;
use common::compute::Empty;

let mut client = CudaExecutorClient::connect("http://[YOUR_SERVER_IP]:50051").await?;
let request = tonic::Request::new(Empty {});
let status = client.get_gpu_status(request).await?.into_inner();

println!("GPU: {} | Temp: {}°C | Mem: {}/{} MB | Load: {}%",
    status.gpu_name,
    status.temperature_celsius,
    status.memory_used_mb,
    status.memory_total_mb,
    status.load_percentage,
);
```

### Dependencies for `common`

```toml
[package]
name = "common"
version.workspace = true
edition.workspace = true

[dependencies]
tonic = "0.12"
prost = "0.13"
tokio = { version = "1", features = ["full"] }

[build-dependencies]
tonic-build = "0.12"
```
