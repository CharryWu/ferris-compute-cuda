# Decision 0002: Gitignore Strategy

## Context

The project involves three different operating systems (macOS client, Windows/Linux hosts) and multiple build systems (Cargo for Rust, NVCC for CUDA, and MSVC for Windows linking). We need to prevent binary artifacts and OS-specific metadata from polluting the repository.

## Decision

We implemented a multi-layered `.gitignore` at the root of the workspace.

- **Rule:** Comments must be on their own lines to prevent pattern corruption.
- **Scope:**
  - **Rust:** Ignores the shared `/target` folder.
  - **CUDA:** Ignores intermediate compiler files (`*.ptx`, `*.cubin`, `tmpxft*`).
  - **Windows:** Ignores MSVC debug symbols (`*.pdb`, `*.ilk`) and IDE folders (`.vs/`).
  - **macOS:** Ignores `.DS_Store`.

## Key Considerations

- **Cargo.lock:** We chose NOT to ignore this file. Since this project consists of binaries (CLI/Daemon) rather than just a library, committing the lockfile ensures deterministic builds across the macOS client and the remote GPU host.
