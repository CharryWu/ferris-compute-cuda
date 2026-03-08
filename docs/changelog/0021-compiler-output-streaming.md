# 0021-compiler-output-streaming.md

## Context

The Host previously only reported whether compilation succeeded or failed. Users had no way to debug syntax errors or linker issues in their CUDA source code.

## Decision

- **Output Capture:** Switched from `Command::status()` to `Command::output()` for the `nvcc` process to capture `stdout` and `stderr`.
- **Pre-emptive Streaming:** Implemented a logic gate that sends compiler output strings back to the client immediately after the compiler finishes, regardless of the success exit code.

## Key Considerations

- **Developer Loop:** This drastically shortens the "Fix-Run" cycle by providing immediate feedback on CUDA syntax errors.
- **Buffers:** Using `utf8_lossy` ensures the client doesn't crash if the compiler output contains non-UTF8 characters (though rare for nvcc).
