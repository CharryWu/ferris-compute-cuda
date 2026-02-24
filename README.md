# 🦀 ferris-compute-cuda

A remote-execution command line tool for CUDA programming. Write CUDA code on your local machine (macOS/Linux/Windows) and execute it instantly on a remote server equipped with NVIDIA GPUs.

## 🏗 Project Structure

This project is organized as a **Rust Workspace** to separate concerns between the client, the server, and shared protocols.

* [`crates/client/`](/crates/client/): The CLI tool used to send code and receive results.
* [`crates/host/`](/crates/host/): The daemon that runs on the GPU server, handles compilation (`nvcc`), and execution.
* [`crates/common/`](/crates/common/): Shared logic, including the [gRPC Protobuf definitions](/crates/common/proto/compute.proto).

## 📚 Documentation

We maintain a "Living Lab Manual" to track both the technical architecture and the evolution of the project.

### Architecture (The "What")

Detailed explanations of how individual components work:

* **[Communication Protocol](/docs/architecture/common-protocol.md)**: gRPC and Protobuf specifications.
* **[Host Execution Engine](/docs/architecture/host-engine.md)**: How code is compiled and run on the GPU.

### Decision Log (The "Why")

Historical records of key architectural decisions:

1. [01: Initial Project Setup](/docs/decisions/01-initial-setup.md)
2. [02: Gitignore Strategy](/docs/decisions/02-gitignore-strategy.md)
3. [03: Communication Contract](/docs/decisions/03-communication-contract.md)
4. [04: Host Execution Logic](/docs/decisions/04-host-execution-logic.md)

---

## 🚀 Quick Start (MVP)

### Prerequisites

* **Local:** Rust (2024 edition).
* **Remote:** NVIDIA GPU + [CUDA Toolkit](https://developer.nvidia.com/cuda-downloads) (`nvcc` must be in PATH).

### Running the Host

```bash
# On the GPU Server
cargo run -p host

```

### Running the Client

```bash
# On your local machine
cargo run -p client -- path/to/kernel.cu

```

## 🛡 Security Note

**Warning:** This MVP currently allows Remote Code Execution (RCE) by design. Only run the host in a trusted, private network until the [Sandboxing Architecture](/docs/architecture/host-security.md) (Phase 2) is implemented.
