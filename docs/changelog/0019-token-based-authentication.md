# 0019-token-based-authentication.md

## Context

The system was previously open to any user on the network, and the client lacked input validation, allowing non-CUDA files to be sent to the host for compilation.

## Decision

- **Metadata Interception:** Implemented a gRPC Interceptor on the Host using `with_interceptor`. Rejections happen before the execution task is spawned.
- **Client Validation (The "Double-Check"):** Added a client-side filter to verify file extensions before transmission.
  - **Supported Extensions:** `.cu`, `.cuh`, `.ptx`, `.cubin`.
- **Header Standard:** Adopted `x-ferris-token` as the custom metadata key.
- **Environment Driven:** The Host compares incoming tokens against a `FERRIS_AUTH_TOKEN` environment variable.

## Key Considerations

- **Efficiency:** Rejections now happen at two levels:
    1. The **Client** (saves network bandwidth on wrong file types).
    2. The **Interceptor** (saves Host compute on unauthorized requests).
- **Future-Proofing:** Inclusion of `.ptx` and `.cubin` allows for workflows where the client provides pre-compiled intermediate code rather than raw source.
- **Security:** Tokens are currently sent over unencrypted channels (HTTP).
