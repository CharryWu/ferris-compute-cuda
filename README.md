# 🦀 ferris-compute-cuda

A remote-execution command line tool for CUDA programming. Write CUDA code on your local machine (macOS/Linux/Windows) and execute it instantly on a remote server equipped with NVIDIA GPUs.

## 🏗 Project Structure

This project is a **Rust Workspace** optimized for cross-platform GPU development.

* [`crates/client/`](./crates/client/): The CLI tool (runs on any OS) used to send code and receive results.
* [`crates/host/`](./crates/host/): The daemon (runs on the GPU server) featuring **auto-discovery** for MSVC and NVCC toolchains.
* [`crates/common/`](./crates/common/): Shared logic, including the [gRPC Protobuf definitions](./crates/common/proto/compute.proto).

## 📚 Living Lab Manual

### Architecture (The "What")

* **[Communication Protocol](./docs/architecture/common-protocol.md)**: gRPC and Protobuf specifications.
* **[Host Execution Engine](./docs/architecture/host-engine.md)**: Job isolation and dynamic toolchain resolution.

### Changelog (The "Why")

We maintain a historical record of all architectural pivots:

* [0008-execution-timeouts.md](./docs/changelog/0008-execution-timeouts.md)
* [0009-network-binding-and-firewall.md](./docs/changelog/0009-network-binding-and-firewall.md)
* [0016-modular-utility-refactoring.md](./docs/changelog/0016-modular-utility-refactoring.md)
* [0017-startup-environment-validation.md](./docs/changelog/0017-startup-environment-validation.md)
* [0018-windows-path-execution-fix.md](./docs/changelog/0018-windows-path-execution-fix.md)

---

## 🚀 Quick Start

### 1. Start the Host (GPU Server)

The host now performs a **Pre-flight Check** at startup. On Windows, it will automatically locate your Visual Studio toolchain using `vswhere`.

```bash
# On the GPU Server (Windows/Linux)
cargo run -p host

```

### 2. Run your Code (Local Client)

Use the following format to point your local machine to your server's IP address.

```bash
# From your local machine (e.g., macOS)
cargo run -p client -- --server "http://<SERVER_IP>:50051" <PATH_TO_CU_FILE>

```

**Example:**

```bash
cargo run -p client -- --server "http://10.0.0.181:50051" ./examples/matrix_addition.cu

```

### 🔑 Authentication

1. **CLI Flag:** `--token "my-secret"`
2. **.env File:** Create a `.env` file with `FERRIS_AUTH_TOKEN=my-secret`

**Usage:**
Host: `cargo run -p host`
Client: `cargo run -p client -- --server "http://localhost:50051" ./kernel.cu`

| Method | Where to find it | Priority |
| --- | --- | --- |
| CLI Argument | cargo run -- --token "xyz" | 1 (Highest) |
| Shell Variable | export FERRIS_AUTH_TOKEN="xyz" | 2 |
| .env File | FERRIS_AUTH_TOKEN=xyz | 3 |
| Hardcoded Default | None (Application Error) | 4 (Lowest) |

### 📊 Check GPU Status

You can now check the health of the remote GPU before running a job:

```bash
cargo run -p client -- status --server "http://10.0.0.181:50051"
```

---

## 🛡 Security & Resource Management

* **Timeouts:** Execution is capped at **30 seconds** by default to prevent zombie kernels.
* **Isolation:** Every job runs in a unique UUID-based scratchpad under the `scratch/` directory.
* **Fail-Fast:** The host will refuse to start if it cannot find the required C++ and CUDA compilers.
