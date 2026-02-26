# 📄 Architecture: Host Execution Engine

**File:** [crates/host/src/main.rs](/crates/host/src/main.rs)

The Host is the GPU-side daemon that receives CUDA source code over gRPC, compiles it with `nvcc`, executes the resulting binary, and streams output back to the client in real time.

---

## Imports Breakdown

```rust
use common::compute::cuda_executor_server::{CudaExecutor, CudaExecutorServer};
use common::compute::{ComputeRequest, ComputeResponse};
use std::path::Path;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};
```

| Import | Purpose |
|---|---|
| `CudaExecutor` | The **trait** auto-generated from the protobuf `service CUDAExecutor`. The host must implement this trait to handle RPCs. |
| `CudaExecutorServer` | A wrapper struct that plugs a `CudaExecutor` implementation into a `tonic` gRPC server. |
| `ComputeRequest` / `ComputeResponse` | Auto-generated Rust structs from the protobuf `message` definitions. |
| `std::path::Path` | Borrowed path reference (the immutable counterpart of `PathBuf`). Used for path manipulation. |
| `tokio::fs` | Async file system operations -- non-blocking versions of `std::fs`. Safe to use inside Tokio tasks. |
| `tokio::process::Command` | Async version of `std::process::Command`. Spawns child processes without blocking the runtime. |
| `tokio::sync::mpsc` | Multi-producer, single-consumer async channel. Used to pipe results from the background task to the gRPC stream. |
| `ReceiverStream` | Adapts an `mpsc::Receiver` into a `Stream` that tonic can send over gRPC. |
| `Server` | Tonic's gRPC server builder. Used in `main()` to bind the service to a port. |
| `Request` / `Response` | Tonic wrappers around the raw protobuf messages, carrying metadata like headers. |
| `Status` | gRPC status codes (like HTTP status codes). Used to signal errors such as `INTERNAL` or `NOT_FOUND`. |

### `tokio::fs` vs `std::fs`

This is a key distinction for backend developers. `std::fs::write` is **synchronous** -- it blocks the OS thread until the disk write completes. Inside a Tokio task, this starves other tasks sharing that thread. `tokio::fs::write` is **async** -- it offloads the blocking I/O to a dedicated thread pool and yields the Tokio worker thread back to the scheduler. Always prefer `tokio::fs` inside async contexts.

---

## The Unit Struct: `HostExecutor`

```rust
pub struct HostExecutor;
```

This is a **unit struct** -- a struct with no fields. It has zero size at runtime (ZST: zero-sized type). It exists purely to be the "receiver" for the trait implementation. Think of it as a namespace for methods.

In Rust, you cannot implement a trait on "nothing" -- you need a type. `HostExecutor` is that type. Since it holds no state, it costs nothing to create or pass around.

---

## `#[tonic::async_trait]` -- Async Traits

```rust
#[tonic::async_trait]
impl CudaExecutor for HostExecutor {
```

Rust traits cannot natively contain `async fn` methods (as of Rust 2024, this is stabilizing but tonic still uses the macro). The `#[tonic::async_trait]` procedural macro rewrites `async fn` methods into regular functions that return `Pin<Box<dyn Future>>`. This is the same `async_trait` crate re-exported by tonic.

Without this macro, the compiler would reject `async fn execute_code(...)` inside a trait impl.

---

## Associated Type: `ExecuteCodeStream`

```rust
type ExecuteCodeStream = ReceiverStream<Result<ComputeResponse, Status>>;
```

This is an **associated type** -- the trait `CudaExecutor` declares that implementors must specify what type of stream `ExecuteCode` returns. This project uses `ReceiverStream`, which wraps an `mpsc::Receiver`.

The full type reads as: "a stream that yields `Result<ComputeResponse, Status>` items" -- each item is either a successful response message or a gRPC error.

---

## The `execute_code` Method Signature

```rust
async fn execute_code(
    &self,
    request: Request<ComputeRequest>,
) -> Result<Response<Self::ExecuteCodeStream>, Status> {
```

| Parameter | Meaning |
|---|---|
| `&self` | Borrows the `HostExecutor`. Since it's a ZST, this is essentially free. The `&` means "I'm reading, not consuming." |
| `Request<ComputeRequest>` | Tonic wrapper holding the decoded protobuf message plus gRPC metadata (headers, extensions). |
| `Result<Response<...>, Status>` | Returns either a success response containing the stream, or a gRPC error status. |
| `Self::ExecuteCodeStream` | Refers to the associated type defined above -- `ReceiverStream<Result<ComputeResponse, Status>>`. |

---

## `request.into_inner()`

```rust
let req = request.into_inner();
```

`.into_inner()` **consumes** the `Request` wrapper and returns the inner `ComputeRequest`. The word "into" in Rust conventionally means "take ownership and transform" (as opposed to `.as_ref()` which borrows). After this call, `request` no longer exists -- its ownership moved into `req`.

---

## The `mpsc` Channel Pattern

```rust
let (tx, rx) = mpsc::channel(100);
```

This creates a **bounded async channel** with a buffer of 100 messages:

- `tx` (transmitter / sender) -- used inside the spawned task to push responses.
- `rx` (receiver) -- wrapped in `ReceiverStream` and returned to tonic, which drains it to the client.

The buffer size of 100 means the sender can push up to 100 messages before it has to wait for the receiver to consume them. This decouples the compilation speed from the network speed.

### Why this pattern?

The method must return the stream **immediately** (so the client can start listening), but the compilation takes time. The solution: spawn a background task that does the work and sends results through the channel. The stream starts delivering messages as soon as the first one is sent.

```
execute_code() returns immediately
       │
       ├── returns ReceiverStream(rx) ──> tonic ──> gRPC ──> client
       │
       └── spawns background task
               │
               ├── writes file
               ├── runs nvcc ──> tx.send(compile result)
               ├── runs binary ──> tx.send(stdout) / tx.send(stderr)
               └── cleans up
```

---

## `tokio::spawn(async move { ... })`

```rust
tokio::spawn(async move {
    // ...
});
```

This spawns a new **Tokio task** -- a lightweight unit of concurrent execution (not an OS thread). Key details:

### `async move`

The `move` keyword **transfers ownership** of all captured variables (`tx`, `req`) into the async block. Without `move`, the block would try to borrow them, but since the block outlives `execute_code` (it runs in the background), the borrow checker would reject it. `move` tells the compiler: "this closure owns these values now."

### What `tokio::spawn` returns

It returns a `JoinHandle`, which is deliberately ignored here with no `let` binding. The task runs independently -- "fire and forget." The stream (via `rx`) is the only connection back to the caller.

---

## UUID-Based Job Isolation

```rust
let job_id = uuid::Uuid::new_v4().to_string();
let working_dir = Path::new("scratch").join(&job_id);
```

Each job gets a unique directory under `scratch/` (e.g., `scratch/a1b2c3d4-...`). This prevents concurrent jobs from overwriting each other's files.

### `Path::new("scratch").join(&job_id)`

- `Path::new("scratch")` creates a borrowed `&Path` from a string literal.
- `.join(&job_id)` appends the UUID as a subdirectory, returning an owned `PathBuf`. On Unix this produces `scratch/a1b2c3d4-...`, on Windows `scratch\a1b2c3d4-...`.

---

## `if let` -- Pattern Matching for Single Variants

```rust
if let Err(e) = fs::create_dir_all(&working_dir).await {
    let _ = tx.send(Err(Status::internal(format!("Failed to create workspace: {}", e)))).await;
    return;
}
```

`if let` is syntactic sugar for a `match` with only one arm you care about. This reads: "if the result is an `Err`, bind the error to `e` and run this block; otherwise, keep going."

The equivalent `match` would be:

```rust
match fs::create_dir_all(&working_dir).await {
    Err(e) => {
        let _ = tx.send(Err(Status::internal(...))).await;
        return;
    }
    Ok(_) => {} // do nothing, continue
}
```

### `let _ = tx.send(...).await`

The `let _ =` pattern explicitly **discards** the result. `tx.send()` returns `Result<(), SendError>` -- it fails if the receiver was dropped (client disconnected). In this fire-and-forget context, there's nothing useful to do if the send fails, so the error is silently dropped. The `_` tells the compiler "I know I'm ignoring this, don't warn me."

### `return` inside a spawned task

The `return` exits the **async block**, not `execute_code`. Since this code is inside `tokio::spawn(async move { ... })`, `return` ends the background task early -- like a short-circuit on error.

---

## `cfg!(windows)` -- Compile-Time Platform Detection

```rust
let bin_name = if cfg!(windows) { "app.exe" } else { "app.out" };
```

`cfg!(windows)` is a **compile-time macro** that evaluates to `true` or `false` based on the target platform. The compiler replaces this with a constant boolean -- there's zero runtime cost. On a Linux/macOS build, this compiles to just `let bin_name = "app.out";`.

This is different from `#[cfg(windows)]` (attribute form), which conditionally *includes or excludes* code. The `cfg!()` macro form always compiles both branches but evaluates the condition.

---

## `tokio::process::Command` -- Async Process Execution

```rust
let compile_status = Command::new("nvcc")
    .arg(&file_path)
    .args(&req.compiler_flags)
    .arg("-o")
    .arg(&bin_path)
    .current_dir(&working_dir)
    .status()
    .await;
```

This is the **builder pattern** -- each method returns `&mut Command`, allowing chained calls.

| Method | Purpose |
|---|---|
| `Command::new("nvcc")` | Creates a new command for the `nvcc` CUDA compiler. |
| `.arg(&file_path)` | Adds a single argument (the source file). |
| `.args(&req.compiler_flags)` | Adds multiple arguments from a `Vec<String>` (e.g., `["-arch=sm_80", "-O3"]`). |
| `.arg("-o").arg(&bin_path)` | Sets the output binary path. |
| `.current_dir(&working_dir)` | Sets the working directory for the child process. |
| `.status()` | Spawns the process and waits for it to finish, returning `io::Result<ExitStatus>`. |
| `.await` | Yields the Tokio task while waiting -- other tasks can run on this thread. |

### `.status()` vs `.output()`

- `.status()` (line 47) -- waits for the process to finish, returns only the exit code. Used for compilation where we just need pass/fail.
- `.output()` (line 57) -- waits for the process and **captures all stdout and stderr** into memory. Used for execution where we need the program's output.

---

## Match with Guard: `Ok(s) if s.success()`

```rust
match compile_status {
    Ok(s) if s.success() => {
        // compilation succeeded
    }
    _ => {
        // anything else: compile error, or nvcc not found
    }
}
```

This is a **match arm with a guard clause**. The arm `Ok(s) if s.success()` matches only when:

1. The `Result` is `Ok` (nvcc was found and ran), **AND**
2. The exit status indicates success (exit code 0).

If nvcc returned a non-zero exit code (compilation error), `s.success()` is `false`, so it falls through to the `_` wildcard arm. If nvcc wasn't found at all, the `Result` is `Err`, which also doesn't match `Ok(s)`.

The `_` wildcard matches everything else -- it's the "catch-all" default case.

---

## `String::from_utf8_lossy` -- Safe Byte-to-String Conversion

```rust
let stdout = String::from_utf8_lossy(&out.stdout);
let stderr = String::from_utf8_lossy(&out.stderr);
```

Process output (`out.stdout`, `out.stderr`) is raw bytes (`Vec<u8>`), not guaranteed UTF-8. `from_utf8_lossy` converts bytes to a string, replacing any invalid UTF-8 sequences with the Unicode replacement character `U+FFFD` (�). This is "lossy" because information can be lost, but it never panics.

It returns `Cow<str>` (Copy on Write) -- if the bytes are already valid UTF-8, it borrows them (zero-copy). If replacement was needed, it allocates a new `String`. The `.to_string()` on line 65 forces it into an owned `String` either way.

---

## `.into()` -- The `From`/`Into` Conversion

```rust
ComputeResponse { output: "🚀 Compilation successful. Running...".into(), is_error: false }
```

`.into()` calls the `Into` trait to convert a `&str` literal into a `String`. This works because the standard library implements `From<&str> for String`. It's shorthand for `String::from("...")` or `.to_string()`.

The protobuf-generated `ComputeResponse` struct expects `output: String`, so `.into()` performs the conversion inline.

---

## Cleanup: `fs::remove_dir_all`

```rust
let _ = fs::remove_dir_all(&working_dir).await;
println!("🧹 Cleaned up job {}", job_id);
```

Deletes the entire job directory (source file + compiled binary) asynchronously. The `let _ =` discards any error -- if cleanup fails (e.g., permission issue), the server continues running. This is acceptable for an MVP; production code might log the error.

---

## The Return: Wiring `rx` into the gRPC Stream

```rust
Ok(Response::new(ReceiverStream::new(rx)))
```

This line runs **immediately** after `tokio::spawn` -- it doesn't wait for the background task to finish. It wraps the channel receiver (`rx`) in a `ReceiverStream`, which implements tonic's `Stream` trait, and returns it as the gRPC response.

Tonic then polls this stream, sending each `ComputeResponse` to the client as it arrives through the channel. When the background task finishes and `tx` is dropped, the stream ends naturally -- the client sees the end of the stream.

This is the core of the **server-side streaming** pattern: return the stream handle immediately, populate it asynchronously from a background task.

---

## The `main` Entry Point

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let executor = HostExecutor;

    fs::create_dir_all("scratch").await?;

    println!("🦀 Ferris-Compute-Cuda Host listening on {}", addr);

    Server::builder()
        .add_service(CudaExecutorServer::new(executor))
        .serve(addr)
        .await?;

    Ok(())
}
```

### `"[::1]:50051".parse()?`

The `.parse()` method calls the `FromStr` trait to convert a string into a `SocketAddr`. `[::1]` is the IPv6 loopback address (equivalent to `127.0.0.1` in IPv4). Port `50051` is the conventional default for gRPC servers. The `?` propagates the error if the string is not a valid address.

### `fs::create_dir_all("scratch").await?`

Ensures the base `scratch/` directory exists before the server starts accepting jobs. This uses `tokio::fs` (async) and propagates errors with `?` -- if the directory can't be created (e.g., permission denied), the server exits immediately with a clear error rather than failing silently on the first job.

### `Server::builder()` -- The gRPC Server

This uses the **builder pattern** to configure and start the tonic gRPC server:

| Method | Purpose |
|---|---|
| `Server::builder()` | Creates a new server configuration. |
| `.add_service(CudaExecutorServer::new(executor))` | Registers the `HostExecutor` implementation as the handler for the `CUDAExecutor` gRPC service. `CudaExecutorServer` is the auto-generated wrapper from protobuf that routes incoming RPCs to the trait methods. |
| `.serve(addr).await?` | Binds to the address and starts listening. This **blocks forever** (awaits indefinitely) until the server is shut down or an error occurs. |

### Why `Server::builder().serve()` never returns

`.serve(addr).await` is an infinite loop that accepts connections and dispatches requests. The `Ok(())` at the end of `main` is only reached if the server is explicitly shut down (e.g., via a signal handler). In practice, you stop the host with `Ctrl+C`.

---

## Execution Flow

### Server Startup

1. **Parse address** -- `"[::1]:50051"` is parsed into a `SocketAddr`.
2. **Create executor** -- instantiate the zero-sized `HostExecutor`.
3. **Ensure scratch directory** -- create `scratch/` if it doesn't exist.
4. **Start gRPC server** -- bind to the port and listen for incoming connections.

### Per-Job Pipeline (inside `execute_code`)

1. **Receive request** -- tonic decodes the gRPC call into a `ComputeRequest`.
2. **Create channel** -- `mpsc::channel(100)` for communication between the background task and the stream.
3. **Spawn background task** -- `tokio::spawn` starts the compilation pipeline without blocking.
4. **Return stream immediately** -- the client starts listening before any work is done.
5. **Create workspace** -- UUID-named directory under `scratch/` for job isolation.
6. **Write source file** -- async write of the CUDA source code to disk.
7. **Compile** -- run `nvcc` asynchronously, check exit status.
8. **Execute** -- if compilation succeeded, run the binary and capture output.
9. **Stream results** -- send stdout/stderr through the channel to the client.
10. **Cleanup** -- delete the job directory.
11. **Stream ends** -- when the task finishes, `tx` is dropped, closing the channel and ending the gRPC stream.
