# Decision 0014: Dynamic MSVC Toolchain Discovery

## Context

Manual configuration of the MSVC `ccbin` path is error-prone due to frequent Visual Studio updates and varying installation directories across Windows environments.

## Decision

- **Tool:** Adopted `vswhere.exe` as the source of truth for Visual Studio installation paths.
- **Logic:** Implemented a discovery helper that traverses the `VC/Tools/MSVC` directory to find the latest versioned toolset.
- **Architecture:** Standardized on the `Hostx64/x64` compiler to prevent 32-bit `cl.exe` mismatches that cause `ACCESS_VIOLATION` errors.

## Key Considerations

- **Resilience:** The Host can now self-heal after Visual Studio updates without requiring code changes.
- **Platform Awareness:** This discovery logic is conditionally compiled/skipped on Linux/macOS, keeping the Host cross-platform.
