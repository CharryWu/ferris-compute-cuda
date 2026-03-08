# 0020-configuration-priority-and-dotenv.md

## Context

Standardizing how the Host and Client receive secrets. Avoids manual environment exports by using persistent file-based configuration.

## Decision

- **Dotenv:** Used `dotenvy` to load `.env` files into the process environment.
- **Clap env Feature:** Enabled the `env` feature flag in `Cargo.toml` to fix the `E0599` error and allow declarative environment variable mapping.
- **Priority Hierarchy:** 1. Command Line Arg (`--token`)
    2. Environment Variable / `.env`
- **Host Modernization:** Converted Host to a `clap`-based CLI for better runtime control.
- **Closure Fix:** Resolved FnOnce vs FnMut trait mismatch by correctly managing token ownership within the gRPC interceptor move-closure.
