# 0017-startup-environment-validation.md

## Context

Running a GPU execution host on Windows requires a specific host compiler (MSVC x64). Previously, the server would start even if the compiler was missing, only failing when a client actually submitted a job. This created a poor user experience and delayed error detection.

## Decision

- **Fail-Fast Mechanism:** Integrated a pre-flight check in `main()` using `utils::find_msvc_x64_bin()`.
- **Platform Conditional:** The check is scoped to Windows using `cfg!(windows)` to ensure Linux/macOS environments (which typically rely on the system `PATH` for `gcc` or `clang`) are not negatively impacted.
- **Process Exit:** If the required toolchain is not detected, the program prints a descriptive error message and exits with status code `1` before binding the network port.

## Key Considerations

- **Operational Clarity:** Server administrators are notified immediately of configuration issues during deployment.
- **Resource Protection:** Prevents the server from accepting gRPC connections that it is physically unable to fulfill.
