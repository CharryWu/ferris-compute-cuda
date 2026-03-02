# Decision 0010: Windows Host Compiler Dependency

## Context

On Windows systems, the NVIDIA CUDA Compiler (`nvcc`) requires the Microsoft Visual C++ (MSVC) compiler (`cl.exe`) to be available in the system `PATH` to perform the host-side compilation of CUDA code.

## Decision

- **Requirement:** The Host machine must have "Desktop development with C++" installed via the Visual Studio Installer.
- **Environment:** The Host daemon should ideally be launched from a "Developer PowerShell" or have the MSVC bin directory explicitly added to the system `PATH`.

## Key Considerations

- **Implicit Dependency:** While the project is written in Rust, the underlying GPU toolchain relies on C++ infrastructure.
- **Portability:** This issue does not affect Linux/macOS hosts (which use `gcc` or `clang`), making Windows setup slightly more manual.
