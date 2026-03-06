# 0018-windows-path-execution-fix.md

## Context

After successful compilation, the Host failed to execute the resulting binary, returning `os error 2 (The system cannot find the file specified)`. This occurred despite the binary being present in the designated `working_dir`.

## Decision

- **Absolute Path Execution:** Switched from relative path string formatting (e.g., `./app.exe`) to using the previously defined absolute `bin_path` (built via `working_dir.join(bin_name)`) inside `AsyncCommand::new()`.
- **Logic Cleanup:** Removed unused variable warnings by properly utilizing `bin_path` during the execution phase.

## Key Considerations

- **Windows API Behavior:** On Windows, the `CreateProcess` call does not resolve the executable path relative to the `lpCurrentDirectory` parameter. It resolves relative to the parent process's environment. Using absolute paths is the most reliable cross-platform pattern in Rust's `std::process` / `tokio::process`.
