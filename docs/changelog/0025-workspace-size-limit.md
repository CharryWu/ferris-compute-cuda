# 0025-workspace-size-limit.md

## Context

Limit workspace size to 50MB to improve Denial of Service attempts.

## Decision

- Payload Guardrails: Added a 50MB hard limit to client, on the total size of the workspace to be synced to prevent resource exhaustion on the host

## Key COnsiderations

- Security: Size limits mitigate basic Denial of Service attempts via large file uploads.
