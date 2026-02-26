# 📄 Architecture: Client CLI

**File:** [crates/client/src/main.rs](/crates/client/src/main.rs)

The Client is the command-line interface that reads a local `.cu` file, connects to the remote GPU host via gRPC, sends the source code, and streams back compiler/execution output in real time.

---

## CLI Argument Parsing (`clap` + Derive)

```rust
#[derive(Parser, Debug)]
#[command(author, version, about = "Remote CUDA Executor Client")]
struct Args {
    file: PathBuf,

    #[arg(short, long, default_value = "http://[::1]:50051")]
    server: String,

    #[arg(short, long)]
    flags: Vec<String>,
}
```

### Why `PathBuf` instead of `String`?

`PathBuf` (from `std::path`) is used because:

1. **Not all file paths are valid UTF-8.** A `String` in Rust is guaranteed UTF-8, but file paths on Linux/macOS are arbitrary byte sequences, and Windows uses UTF-16 internally. `PathBuf` wraps `OsString` to represent platform-native paths without data loss.
2. **Semantic meaning.** `PathBuf` provides purpose-built methods like `.file_name()`, `.extension()`, `.parent()`, `.display()`, and `.join()` that handle cross-platform path separators automatically.
3. **`clap` integrates natively** with `PathBuf`, parsing CLI arguments directly into the correct type.

The `server` field remains a `String` because a URL is guaranteed UTF-8 text, not a file path.

---

## The `main` Function Signature

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
```

### Standard return type of `main` in Rust

The main function is able to return any type that implements the `std::process::Termination` trait, including `Result<T, E>` where `T` is the unit type `()` and `E` implements the `Debug` trait.

Box propagates the Debug trait from the value it boxes:  
`Box<T>` implements `Debug` whenever `T: Debug`:

```rust
impl<T: Debug + ?Sized> Debug for Box<T> { ... }
```

Chaining it all together:

1. `dyn Error` implies Debug (because `Error: Debug + Display`)
2. `Box<T>` implements Debug when `T: Debug`
3. Therefore `Box<dyn Error> implements Debug`
4. Therefore `Result<(), Box<dyn Error>>` satisfies main()'s requirement

### `dyn` -- Dynamic Dispatch

`dyn std::error::Error` is a **trait object**. The function can return many different error types (I/O errors from file reading, connection errors from gRPC, stream errors, etc.). They are all different structs, but they all implement the `Error` trait. `dyn` tells the compiler to resolve method calls at **runtime** via a vtable, rather than at compile time.

### `Box<dyn Error>` -- Why Heap Allocation?

Rust return types must have a known size at compile time. `dyn Error` is **unsized** (the concrete error could be 24 bytes or 48 bytes). `Box` solves this by placing the error on the heap and keeping only a fixed-size pointer (8 bytes on 64-bit) on the stack.

`Box<dyn std::error::Error>` means: "a heap-allocated value of some type that implements `Error`, accessed through a pointer." This is idiomatic Rust for "any error" in CLIs and prototypes.

### `Result<(), Box<dyn Error>>`

- `Result<T, E>` is Rust's enum for recoverable errors: `Ok(T)` or `Err(E)`.
- `()` is the unit type (like `void`): on success, return nothing.
- This return type enables the `?` operator throughout the function. Each `?` says "if this is `Err`, convert to `Box<dyn Error>` and return early."

### `#[tokio::main]` -- Async Runtime Bootstrap

Rust has no built-in async runtime. `#[tokio::main]` is a **procedural macro** that transforms the function at compile time, wrapping it in a Tokio runtime:

```rust
// What you write:
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> { ... }

// What the compiler sees after macro expansion:
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { ... })
}
```

---

## Procedural Macros in Rust

Procedural macros are Rust programs that receive source code as input, transform it, and output new source code at compile time. There are three flavors:

### Derive Macros

```rust
#[derive(Parser, Debug)]
struct Args { ... }
```

The compiler hands the token stream of `struct Args` to a function in the `clap` crate, which generates an `impl Parser for Args` block with all argument-parsing logic.

### Attribute Macros

```rust
#[tokio::main]
async fn main() { }
```

The macro receives both the attribute arguments and the full function definition as tokens, outputting a completely rewritten function.

### Function-like Macros

```rust
tonic::include_proto!("compute");
```

Called like functions with `!`. This one reads generated protobuf Rust code from the build output directory and pastes it inline.

**Mental model:** Procedural macros are compile-time code generators. They're separate Rust programs that see your source as tokens, transform them, and return new tokens for the compiler.

---

## The Tokio Runtime

### What Tokio Is

Tokio bundles three things:

1. **An event loop** (reactor) -- listens for OS events using `epoll` (Linux) / `kqueue` (macOS).
2. **A task scheduler** -- a multi-threaded work-stealing scheduler across a thread pool (one thread per CPU core by default).
3. **An I/O library** -- async networking, file I/O, timers, channels, etc.

### Key Differences from Other Languages

| Concept | Node.js / Go | Rust + Tokio |
|---|---|---|
| Runtime | Built-in, implicit | Explicit, opt-in |
| Async model | Callbacks / goroutines | Zero-cost futures (state machines) |
| Blocking calls | Block the event loop / create new goroutine | Block the OS thread; use `spawn_blocking` for safety |
| Task overhead | Goroutines ~2-8KB stack | Tokio tasks are a few hundred bytes (no stack) |

### Key Rules for Backend Developers

- **Never block inside a Tokio task.** Calling `std::fs::read_to_string` or `std::thread::sleep` holds up the thread and starves other tasks. Use `tokio::task::spawn_blocking()` for blocking work.
- **`.await` is a yield point.** Each `.await` pauses the current task and lets others run. Avoid heavy CPU work between awaits.
- **Synchronous I/O is acceptable at startup** (e.g., line 28 reads the file before any concurrent work), but should be avoided in hot paths on the server side.

---

## Error Handling: `.unwrap()` and `.unwrap_or_default()`

Rust uses `Option<T>` and `Result<T, E>` instead of null pointers or exceptions.

### `.unwrap()` -- "Crash if absent"

Extracts the inner value. **Panics** on `None` or `Err`.

```rust
// host/src/main.rs line 43
let stderr = child.stderr.take().unwrap();
```

This is safe here because `.stderr(Stdio::piped())` was set three lines above, guaranteeing `Some`. In production, prefer `?` or `.expect("message")`.

### `.unwrap_or_default()` -- "Use a fallback"

Returns the inner value, or the type's `Default` implementation on `None`.

```rust
// client/src/main.rs lines 31-35
let file_name = args.file
    .file_name()              // Option<&OsStr> -- None if path ends in ".."
    .unwrap_or_default()      // falls back to "" instead of crashing
    .to_string_lossy()
    .to_string();
```

### The Unwrapping Spectrum

| Method | On `None`/`Err` | Use Case |
|---|---|---|
| `.unwrap()` | Panics | Tests, or logically impossible failure |
| `.expect("msg")` | Panics with message | Same, with a readable crash message |
| `.unwrap_or(val)` | Returns `val` | Specific fallback value |
| `.unwrap_or_default()` | Returns `Default::default()` | Type's default (0, `""`, `vec![]`) is acceptable |
| `.unwrap_or_else(\|\| ...)` | Runs a closure | Expensive fallback computation |
| `?` | Returns error to caller | Idiomatic choice in functions returning `Result`/`Option` |

---

## Execution Flow

1. **Parse CLI args** via `clap` into the `Args` struct.
2. **Read the `.cu` file** from disk using `std::fs::read_to_string`.
3. **Extract the filename** from the path for the host to use in compiler error messages.
4. **Connect to the host** via gRPC using `CudaExecutorClient::connect`.
5. **Send a `ComputeRequest`** containing source code, filename, and compiler flags.
6. **Stream `ComputeResponse` messages** from the host, printing errors in red (`colored` crate) and standard output normally.

---

## The `?` Operator and `Box<dyn Error>` Conversion

The `?` operator appears throughout `main` at every fallible call:

```rust
let source_code = std::fs::read_to_string(&args.file)
    .map_err(|e| format!("Could not read file {}: {}", args.file.display(), e))?;

let mut client = CudaExecutorClient::connect(args.server).await?;

let mut stream = client.execute_code(request).await?.into_inner();

while let Some(response) = stream.message().await? {
```

### What `?` does

The `?` operator is shorthand for: "if this is `Ok`, unwrap the value and keep going; if this is `Err`, **convert** the error and return it from the function immediately."

For example, line 40 desugars to roughly:

```rust
let mut client = match CudaExecutorClient::connect(args.server).await {
    Ok(val) => val,
    Err(e) => return Err(e.into()),  // <-- the key part
};
```

### The `.into()` -- how different errors become `Box<dyn Error>`

The `?` operator calls the `From` trait to convert the original error into the function's return error type. The standard library provides this blanket implementation:

```rust
impl<E: Error + 'static> From<E> for Box<dyn Error> {
    fn from(err: E) -> Box<dyn Error> {
        Box::new(err)   // heap-allocate and erase the concrete type
    }
}
```

This means **any** type implementing `Error` can be automatically converted into `Box<dyn Error>` by wrapping it in a `Box`. So when `?` is used:

| Line | Expression | Original error type | `?` converts to |
|---|---|---|---|
| 29 | `read_to_string(...)` | `std::io::Error` | `Box<dyn Error>` |
| 40 | `connect(...)` | `tonic::transport::Error` | `Box<dyn Error>` |
| 51 | `execute_code(...)` | `tonic::Status` | `Box<dyn Error>` |
| 53 | `stream.message()` | `tonic::Status` | `Box<dyn Error>` |

Each `?` does three things in sequence:

1. Checks if the `Result` is `Err`
2. Calls `.into()` to box the error and erase its concrete type into `dyn Error`
3. Returns `Err(Box<dyn Error>)` from `main`

Without `?`, you'd need a `match` block at every fallible call. With it, the function reads as a straight-line "happy path" -- all the error plumbing is invisible.

### `.map_err()` before `?` -- adding context

```rust
std::fs::read_to_string(&args.file)
    .map_err(|e| format!("Could not read file {}: {}", args.file.display(), e))?;
```

Here `.map_err()` transforms the `io::Error` into a `String` before `?` kicks in. `String` implements `From<String> for Box<dyn Error>`, so the conversion still works. The purpose is to add context -- instead of a bare "No such file or directory", the user sees "Could not read file kernel.cu: No such file or directory".
