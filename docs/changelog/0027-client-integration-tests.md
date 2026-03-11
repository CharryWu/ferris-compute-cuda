# 0027-client-integration-tests.md

## Context

Client crate tests were embedded inline (`#[cfg(test)] mod tests`) in `main.rs` and `utils.rs`. A dedicated test layout was needed to support future host-crate tests and clearer separation of concerns.

## Decision

- **Library Extraction:** Introduced `src/lib.rs` to expose `Args`, `handle_run`, `handle_status`, `gather_files_recursive`, and `read_ignore` as public API, enabling integration tests to exercise core logic.
- **Integration Test Directory:** Moved tests from inline modules into `crates/client/tests/` as `utils_tests.rs` and `run_tests.rs`.
- **Binary Delegation:** Refactored `main.rs` into a thin entry point that delegates to the library.
- **Test Coverage:** `utils_tests.rs` covers `read_ignore` (defaults, custom patterns, comments) and `gather_files_recursive` (extensions, ignore list, nested structure). `run_tests.rs` covers CLI parsing and `handle_run` validation (e.g., empty inputs error).

## Key Considerations

- **Extensibility:** Host crate can follow the same pattern (lib + `tests/`) in future commits.
- **Dependencies:** `tempfile` remains a dev-dependency for filesystem-based utils tests.
