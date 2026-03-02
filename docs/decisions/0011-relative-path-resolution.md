# Decision 0011: Relative Path Resolution in Subprocesses

## Context

When spawning `nvcc` or the resulting binary, we utilized `.current_dir()` to isolate the execution to a specific UUID scratchpad. However, passing absolute or relative-to-root paths as arguments caused a "doubling" effect where the process looked for files in a non-existent nested subdirectory.

## Decision

- **Argument Refactoring:** Switched from passing full `PathBuf` references to using only file names for command arguments when a `current_dir` is specified.
- **Explicit Execution:** Used `./` (or the platform equivalent) when calling the generated binary to ensure the shell looks in the current working directory.

## Key Considerations

- **Platform Differences:** While Linux is strict about `./`, Windows is more lenient but using the filename-only approach works consistently across both when `current_dir` is set.
