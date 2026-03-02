# Decision 0007: Async Concurrency Model (Tokio & .await)

## Context

A remote CUDA executor must handle long-running tasks (compilation, networking, and GPU execution) without blocking the entire server or client UI.

## Decision

- **Runtime:** Adopted `Tokio` as the asynchronous runtime.
- **Syntax:** Used `async/await` for readable, non-blocking code.
- **Error Handling:** Utilized the `?` operator in conjunction with `.await` to ensure clean error propagation.

## Key Considerations

- **Non-blocking I/O:** Using `.await` allows the Host to handle multiple client requests simultaneously on a single thread pool.
- **Chaining:** Chaining `.await?` allows us to write complex sequences (Connect -> Request -> Stream) that look like synchronous code but perform asynchronously.

[Rust official documentation on `Futures and the Async Syntax`](https://doc.rust-lang.org/book/ch17-01-futures-and-syntax.html)
