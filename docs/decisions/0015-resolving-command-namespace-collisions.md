# Decision 0015: Resolving Command Namespace Collisions

## Context

When implementing dynamic toolchain discovery, we introduced `std::process::Command` to run `vswhere.exe`. However, this conflicted with the existing `tokio::process::Command` used for asynchronous gRPC execution. The compiler produced the following errors due to the name collision and the difference between synchronous results and asynchronous futures:

> **Error Output:**
>
> ```text
> error[E0252]: the name `Command` is defined multiple times
>  --> crates\host\src\main.rs:7:5
>   |
> 5 | use std::process::Command;
>   |     --------------------- previous import of the type `Command` here
> 7 | use tokio::process::Command;
>   |     ^^^^^^^^^^^^^^^^^^^^^^^ `Command` reimported here
> 
> error[E0277]: `Result<ExitStatus, std::io::Error>` is not a future
>    --> crates\host\src\main.rs:108:47
>     |
> 108 |              let compile_status = cmd.status().await;
>     |                                               -^^^^^
>     |                                               |
>     |                                               `Result<ExitStatus, std::io::Error>` is not a future
> ```

## Decision

- **Aliasing:** We adopted explicit aliasing for the two different command types to clarify their execution model.
  - `use std::process::Command as SyncCommand;`
  - `use tokio::process::Command as AsyncCommand;`
- **Logic Separation:** - `SyncCommand` is used for initialization and "pre-flight" checks (like finding MSVC) where blocking the thread briefly is acceptable or preferred.
  - `AsyncCommand` is used exclusively inside the gRPC stream to ensure the server remains non-blocking during long compilation or execution tasks.

## Key Considerations

- **Future Awareness:** In Rust, `.await` can only be called on types implementing the `Future` trait. Since `std::process::Command::status()` returns a `Result` immediately, it cannot be awaited. The alias helps developers avoid trying to `await` synchronous operations.
- **Code Clarity:** Using `Sync` and `Async` prefixes makes the intention of the code visible at the call site, reducing the likelihood of future threading bugs.
