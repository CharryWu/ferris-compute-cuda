# 0026-ignore-filtering-and-validation.md

## Context

Refined the workspace synchronization to respect project-specific ignore rules and prevent oversized uploads.

## Decision

- **Decoupled Ignore Logic:** Created `read_ignore` in `utils.rs` to parse `.ferrisignore` and provide a consolidated vector of exclusion patterns.
- **Enhanced Recursion:** Updated `gather_files_recursive` to accept the ignore list as an argument. It now validates both directory and file names against this list before processing.
- **Resource Guardrail:** Integrated a 50MB size validation check in `handle_run` to prevent accidental high-bandwidth uploads.
- **Canonical Anchoring:** Maintained the use of `global_base` from the primary entry point to ensure all relative paths and ignore resolutions are consistent across the workspace.
- **Pattern-Aware Recursion:** Updated gather_files_recursive to perform an early-exit check against a provided list of ignored patterns. This ensures that entire subtrees (like .git/ or target/) are skipped during the workspace walk, improving performance and security.

## Key Considerations

- **Efficiency:** The ignore file is read exactly once per command run, regardless of the number of subdirectories visited.
