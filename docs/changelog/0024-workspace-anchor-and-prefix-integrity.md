# 0024-workspace-anchor-and-prefix-integrity.md

## Context

Initial multi-file synchronization failed for nested directories (e.g., `core/math/constants.cuh`) because the "Base Directory" was being recalculated for every input path. This caused the recursive gatherer to strip the wrong prefixes, effectively "flattening" the project structure in the gRPC map and causing `#include` failures on the Host.

## Decision

- **Global Anchor Pattern:** Refactored `handle_run` to establish a `global_base` using the parent of the first provided input (the entry point). This anchor is now used as the absolute reference for all subsequent files in the sync set.
- **Canonicalization Enforcement:** Integrated `Path::canonicalize()` to resolve absolute paths before prefix stripping. This prevents logic errors when users mix relative (`./`) and absolute (`/Users/...`) path arguments.
- **Protocol Key Normalization:** Enforced forward-slash (`/`) normalization for all keys in the `files` map. This ensures that a workspace gathered on Windows (using `\`) can be reconstructed correctly on a Linux Host (using `/`), and vice versa.
- **Relative Entry Point:** Modified the `entry_point_file` string to be relative to the `global_base`, ensuring the Host's `nvcc` command points to the correct path within the reconstructed `scratch/` workspace.

## Key Considerations

- **Structure Preservation:** This update allows for "Chained Inclusions" (e.g., `main -> wrapper -> constants`) and "Header Re-exporting" by maintaining the exact directory depth of the local project.
- **UX Consistency:** The first argument in the `run` command now dictates the root of the "virtual workspace," providing users with a predictable mental model for how files are bundled.
