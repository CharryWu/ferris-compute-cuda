
# 🦀 ferris-compute-cuda

A remote-execution command line tool for CUDA programming. Write CUDA code on your local machine (macOS/Linux/Windows) and execute it instantly on a remote server equipped with NVIDIA GPUs.

## 🏗 Project Structure

This project is a **Rust Workspace** optimized for cross-platform GPU development.

* [`crates/client/`](./crates/client/): The CLI tool used to send code, manage workspaces, and query telemetry.
* [`crates/host/`](./crates/host/): The daemon featuring **auto-discovery** for MSVC, **Token Interceptors**, and **Workspace Reconstruction**.
* [`crates/common/`](./crates/common/): Shared logic and [gRPC Protobuf definitions](./crates/common/proto/compute.proto).

## 📚 Living Lab Manual (Changelog)

We maintain a historical record of all architectural pivots:

* [0017-startup-environment-validation.md](./docs/changelog/0017-startup-environment-validation.md)
* [0018-windows-path-execution-fix.md](./docs/changelog/0018-windows-path-execution-fix.md)
* [0020-configuration-priority-and-dotenv.md](./docs/changelog/0020-configuration-priority-and-dotenv.md)
* [0023-multi-file-workspace-and-telemetry.md](./docs/changelog/0023-multi-file-workspace-and-telemetry.md)
* [0027-client-integration-tests.md](./docs/changelog/0027-client-integration-tests.md)

---

## 🚀 Quick Start

### 1. Setup Authentication

Create a `.env` file in the root directory (or use environment variables) on both the **Host** and **Client**.

```ini
# .env
FERRIS_AUTH_TOKEN=your-secure-token

```

### 2. Start the Host (GPU Server)

The host performs a **Pre-flight Check** to locate Visual Studio (Windows) and NVCC. It requires a token to authorize incoming requests.
If `FERRIS_AUTH_TOKEN` doesn't exist in environment variable (defined in .env file), you will have to provide a `--token` argument:

```bash
# Start the daemon
cargo run -p host -- --token "optional-override-token"

```

### 3. Use the Client (Local Machine)

The client supports subcommands for monitoring and execution.

#### 📊 Check GPU Status

```bash
cargo run -p client -- status --server "http://<SERVER_IP>:50051"

```

#### 🏃 Run CUDA Code

The client automatically detects if you are sending a single file or an entire project (multi-file support with directories).
You have to supply entry point file as first file argument to the `run` subcommand:

```bash
# Single File
cargo run -p client -- run --server "http://<SERVER_IP>:50051" ./samples/helloworld/matrix_addition.cu

# Multiple Files (supports .cu, .cuh, .cpp. .hpp, .h)
cargo run -p client -- run --server "http://<SERVER_IP>:50051" "<ENTRY_POINT_FILE>" "<INCLUDE_FILE_1>" "<INCLUDE_FILE_2>"
```

*Note: The first path provided is treated as the primary entry point.*

### 🧪 Run Tests

Run unit and integration tests:

```bash
cargo test -p client   # Client: CLI parsing, file gathering, ignore rules
cargo test -p host     # Host: auth, workspace prep, nvcc command building
```

Tests live under [`crates/client/tests/`](./crates/client/tests/) and [`crates/host/tests/`](./crates/host/tests/).

**Pre-commit hook:** Run `./scripts/setup-git-hooks.sh` once to run client and host tests automatically before each commit.

Real example - multiple files:

```bash
cargo run -p client -- run --server "http://10.0.0.181:50051" ./samples/deep_project/main.cu ./samples/deep_project/core/wrapper.cuh ./samples/deep_project/core/math/operations.cuh ./samples/deep_project/core/math/constants.cuh
# Equivalent to:
cargo run -p client -- run --server "http://10.0.0.181:50051" ./samples/deep_project/main.cu ./samples/deep_project/
```

### 📂 Directory Structures

When syncing a folder, **ferris-compute-cuda** maintains the internal hierarchy:

```text
Local:                     Remote NVCC Workspace on Host:
project/                   scratch/<UUID>/
 ├── main.cu        --->    ├── main.cu
 └── include/               └── include/
      └── utils.cuh              └── utils.cuh

---

## 🛠 Configuration Priority

| Priority | Method | Example |
| --- | --- | --- |
| **1 (Highest)** | CLI Argument | `--token "xyz"` |
| **2** | Shell Variable | `export FERRIS_AUTH_TOKEN="xyz"` |
| **3** | `.env` File | `FERRIS_AUTH_TOKEN=xyz` |

---

## 🛡 Features & Security

* **Smart Compilation:** Automatically injects `-rdc=true` for multi-file projects to enable device-side linking.
* **Telemetry:** Real-time reporting of remote GPU temperature, memory usage, and load.
* **Token Authentication:** gRPC Interceptors reject unauthorized requests before spawning tasks.
* **Isolation:** Every job runs in a unique UUID-based scratchpad; workspaces are purged immediately after execution.
* **Compiler Streaming:** Full `nvcc` `stdout` and `stderr` are streamed back to the client for real-time debugging.
* **Workspace Size:** Sync is limited to **50MB** to ensure fast transfers and host stability.
