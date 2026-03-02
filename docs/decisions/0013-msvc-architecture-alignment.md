# Decision 0013: MSVC Architecture Alignment

## Context

The host encountered a `0xC0000005` Access Violation during the `cudafe++` stage of compilation. This was identified as a mismatch between the 64-bit CUDA toolkit and the 32-bit MSVC host compiler (`cl.exe`).

## Decision

- **Architecture Enforcement:** Standardized on the `Hostx64/x64` toolchain.
- **Path Priority:** Updated documentation to require that the 64-bit MSVC binary path appears before the 32-bit version in the system environment variables.
- **Verification:** Added a recommendation to use `where.exe cl.exe` to verify the active compiler before starting the Host daemon.

## Key Considerations

- **Implicit Defaults:** Windows "Developer Command Prompts" often default to x86 for legacy reasons, which is incompatible with modern CUDA versions.
