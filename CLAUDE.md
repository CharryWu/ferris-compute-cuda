# ferris-compute-cuda — AI / contributor context

**What this is:** A personal Rust + CUDA learning project — a remote-execution CLI that sends CUDA code from your machine to a GPU host over gRPC. Not a work/team repo; keep tooling lightweight.

## Architecture (Rust workspace)

| Crate | Role |
|-------|------|
| `crates/client` | CLI: `run`, `status`, `discover`; file gathering; server resolution; interactive prompts |
| `crates/host` | Daemon on the GPU machine: auth interceptors, NVCC workspace prep, streaming logs |
| `crates/common` | Shared types; **Protobuf + gRPC** definitions (`proto/compute.proto`, codegen via `build.rs`) |

Binaries are thin (`main.rs`); logic lives in each crate’s `lib.rs` and modules.

## Build & test

```bash
cargo build -p client
cargo build -p host
cargo test -p client
cargo test -p host
```

Cargo aliases (see `.cargo/config.toml`):

```bash
cargo ferris-run <args>      # same as: cargo run -p client -- run <args>
cargo ferris-status          # same as: cargo run -p client -- status
```

**Prerequisite:** `protoc` on `PATH` — required for `tonic-build` / `prost` codegen in `crates/common`.

## Key behavior (don’t guess)

- **gRPC:** `ExecuteCode` is **server-streaming** (live stdout/stderr-style output); `GetGpuStatus` is unary.
- **Auth:** `FERRIS_AUTH_TOKEN` via CLI (`--token`), env, or `.env`. Host validates gRPC metadata `x-ferris-token`.
- **Server URL:** priority is **CLI `--server` > `FERRIS_SERVER` env > `.env` > `~/.ferris-compute/config.toml`** (see README for full table).
- **Host vs client:** Host needs **NVCC** (Linux/Windows); client builds on **macOS** too. CI mirrors this (host skipped on macOS).
- **Workspace sync:** Capped at **50 MB** for uploads.

## Docs & decisions

- Numbered decision logs: `docs/changelog/NNNN-slug.md`
- Deeper design: `docs/architecture/` (client CLI, host engine, protocol, build script, CI notes)

## Style

- Rust **edition 2024**; `rustfmt.toml` sets `max_width = 120`.
- `Cargo.lock` is **gitignored** in this repo.

## Learning note

The maintainer is **learning Rust**. When you introduce or change non-obvious Rust (ownership, lifetimes, async, trait bounds, error types), **briefly explain** the idiom so it stays a learning project, not a black box.

## Commits

Prefer conventional one-liners: `feat(scope): …`, `fix(scope): …`. Ask before committing if the user prefers to review first.
