# Decision 0028: Client UX Improvements

## Context

Every invocation of the client required the full `cargo run -p client -- run --server "http://..." --token "..."` command. The `--server` flag had to be typed on every call with no persistence, env fallback, or discovery mechanism.

## Decision

Implemented a four-phase improvement to the client CLI ergonomics:

### Phase 1: Quick Ergonomic Wins

- Added `FERRIS_SERVER` env var support to the `--server` argument via clap's `env` attribute, consistent with how `FERRIS_AUTH_TOKEN` already works.
- Created `.cargo/config.toml` with cargo aliases (`cargo ferris-run`, `cargo ferris-status`).
- Documented `cargo install --path crates/client` for a global `ferris-run` binary.

### Phase 2: Persistent Config & Connection History

- New module `config.rs` with support for `~/.ferris-compute/config.toml` (default server) and `~/.ferris-compute/history.json` (connection history).
- Changed `--server` from `String` with a default to `Option<String>` to distinguish explicit vs. absent.
- Introduced `resolve_server()` with priority chain: CLI > env var > config file.
- History is saved automatically after each successful connection (capped at 20 entries, deduplicated, sorted by recency).

### Phase 3: Interactive Connection Prompt

- Added `dialoguer` for terminal-based interactive selection with arrow-key navigation.
- When no server is resolvable (no CLI, env, or config), prompts the user with a list of recent connections plus a "enter new address" option.
- Validates new addresses (scheme, host, port).
- Offers to save the chosen server as the default in `config.toml`.
- Falls back to `localhost:50051` with a warning in non-TTY contexts (CI, piped input).

### Phase 4: LAN Discovery (mDNS)

- Host now advertises via mDNS (`_ferris-compute._tcp.local.`) using the `mdns-sd` crate. Registration is non-fatal -- the server starts regardless.
- Client can discover hosts on the local network with a `discover` subcommand (3-second scan).
- Discovered hosts are integrated into the interactive selection prompt, shown above history entries.

## Key Considerations

- **Backward compatibility:** All changes are additive. Users who always pass `--server` explicitly see no change.
- **Security:** Tokens are never stored in config files. Only server addresses are persisted.
- **Non-interactive safety:** The interactive prompt is skipped when stdin is not a TTY, preventing hangs in CI pipelines.
- **mDNS is best-effort:** If mDNS registration or browsing fails (firewall, unsupported OS), the system falls back gracefully.

## Configuration Priority (Updated)

| Priority | Method | Example |
| --- | --- | --- |
| 1 (Highest) | CLI Argument | `--server "http://..."` |
| 2 | Environment Variable | `FERRIS_SERVER=http://...` |
| 3 | Config File | `~/.ferris-compute/config.toml` |
| 4 (Lowest) | Default | `http://[::1]:50051` |

## Files Changed

- `crates/client/src/lib.rs` — `Args` enum, `resolve_server()`, `Discover` subcommand
- `crates/client/src/main.rs` — `resolve_or_prompt()` integration
- `crates/client/src/config.rs` — New module for config/history persistence
- `crates/client/src/interactive.rs` — New module for TUI prompts
- `crates/client/src/discovery.rs` — New module for mDNS client browsing
- `crates/client/Cargo.toml` — Added serde, toml, dirs, chrono, dialoguer, atty, mdns-sd
- `crates/host/src/lib.rs` — mDNS service advertisement
- `crates/host/Cargo.toml` — Added mdns-sd, hostname
- `.cargo/config.toml` — Cargo aliases
- `README.md` — Updated Quick Start, configuration table, install instructions
- `crates/client/tests/run_tests.rs` — Updated for `Option<String>` server
