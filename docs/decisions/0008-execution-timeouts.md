# Decision 0008: Execution Timeouts

## Context

Remote GPU kernels can easily enter infinite loops or deadlocks. Without a timeout, a single buggy submission could hang a Host worker indefinitely, consuming VRAM and blocking other users.

## Decision

- **Mechanism:** Implemented `tokio::time::timeout` around the binary execution phase.
- **Limit:** Set a default hard limit of 30 seconds for the MVP.
- **Cancellation:** When the timeout expires, the `Command` future is dropped, which effectively sends a `SIGKILL` (on Unix) or `TerminateProcess` (on Windows) to the child process.

## Key Considerations

- **Scope:** We currently only timeout the *execution* of the binary. Compilation (`nvcc`) is generally fast, but in Phase 3, we may need to add a (longer) timeout for compilation as well.
- **User Feedback:** The client is explicitly notified via a `ComputeResponse` with `is_error: true` so they know the failure was a timeout and not a crash.
