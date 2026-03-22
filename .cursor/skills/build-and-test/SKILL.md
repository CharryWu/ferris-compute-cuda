---
name: build-and-test
description: Build and test the ferris-compute-cuda Rust workspace (client + host)
---

Use when the user asks to build, test, or verify the project compiles.

## Steps

1. **Check `protoc`** — The `common` crate needs Protocol Buffers compiler on `PATH` (e.g. `protoc --version`). If missing, say so and point to installing `protobuf-compiler` / `brew install protobuf` / CI uses `arduino/setup-protoc`.

2. **Build both packages**
   - `cargo build -p client`
   - `cargo build -p host`

3. **Run tests**
   - `cargo test -p client`
   - `cargo test -p host`

4. **Report** — Summarize pass/fail; on failure, surface the first actionable error lines.

## Notes

- Host may not build on machines without a full dev setup in edge cases; CI runs client on macOS and client+host on Linux/Windows.
- Prefer running from the **repository root** (workspace root).
