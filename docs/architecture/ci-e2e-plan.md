# CI / E2E Testing Plan

Plan to enable GitHub Actions CI that runs on every push, with a path to real end-to-end (e2e) tests.

---

## Current State

- **Unit tests**: Client (15) and Host (12) tests run via `cargo test -p client` and `cargo test -p host`
- **E2E flow**: Client sends `.cu` files → Host compiles with `nvcc` → Host runs binary on GPU → Client receives streamed output
- **E2E requirements**: NVIDIA GPU, CUDA toolkit (`nvcc`), `nvidia-smi`; on Windows also MSVC (`cl.exe`)

---

## Phase 1: Build + Unit Tests (Immediate, Free)

**Goal:** Run on every push. No GPU required. Catches build breaks and unit test failures.

**Workflow:** `.github/workflows/ci.yml`

| Runner | Client | Host |
|--------|--------|------|
| `ubuntu-latest` | build + test | build + test |
| `windows-latest` | build + test | build + test |
| `macos-latest` | build + test | **skipped** (no CUDA on macOS) |

**Details:**

- Use `dtolnay/rust-toolchain` for Rust
- Cache with `Swatinem/rust-cache`
- Host is skipped on macOS since macOS devices never contain a CUDA environment.
- **Windows caveat:** Host unit tests pass (no nvcc needed). E2E would fail on Windows without MSVC; Phase 1 skips e2e.

**Outcome:** Fast feedback on every push. No cost on public repos.

---

## Phase 2: E2E on GPU Runners

**Goal:** Real e2e: start host, run client with sample `.cu`, assert output.

### Option A: GitHub Larger Runners (GPU)

- **Availability:** GitHub Team or Enterprise Cloud only
- **Runner label:** e.g. `ubuntu-latest-4-cores` or org-configured GPU runner
- **Pre-installed:** NVIDIA drivers, CUDA 12.x, `nvidia-smi`
- **Cost:** Per-minute billing (no free tier for GPU minutes)

**Workflow addition:** `.github/workflows/e2e.yml` (conditional on org having GPU runners)

```yaml
# Pseudocode - actual label depends on org config
jobs:
  e2e-gpu:
    runs-on: [org-gpu-runner]  # e.g. ubuntu-22.04-gpu
    steps:
      - uses: actions/checkout@v4
      - run: cargo build -p host -p client
      - run: |
          FERRIS_AUTH_TOKEN=ci-test-token cargo run -p host -- --token ci-test-token &
          sleep 3
          cargo run -p client -- run --server http://localhost:50051 --token ci-test-token ./samples/helloworld/hello_world.cu
```

### Option B: Self-Hosted Runner

- **Setup:** Machine with NVIDIA GPU + CUDA toolkit (+ MSVC on Windows)
- **Config:** Add self-hosted runner to repo/org, label it e.g. `gpu`
- **Workflow:** Same e2e steps, `runs-on: [self-hosted, gpu]`
- **Cost:** Machine cost only; no GitHub Actions minute charges for self-hosted

**Recommended for most projects:** Self-hosted if you have a GPU dev machine or lab server.

---

## Phase 3: E2E Test Structure

**Suggested layout:**

```
.github/workflows/
  ci.yml          # Phase 1: build + unit tests (all pushes)
  e2e.yml         # Phase 2: e2e (optional, on self-hosted or GPU runner)

scripts/
  e2e-test.sh     # Local e2e script: start host, run client, assert output
```

**E2E script logic:**

1. Build host and client
2. Start host in background with known token
3. Wait for host to listen (poll `:50051` or sleep)
4. Run `client run --server http://localhost:50051 --token <token> ./samples/helloworld/hello_world.cu`
5. Assert stdout contains expected string (e.g. `Hello World from GPU`)
6. Kill host process
7. Exit 0 on success, 1 on failure

**Sample assertion:** `hello_world.cu` prints `Hello World from GPU!` → grep for that in client output.

---

## Implementation Order

| Step | Action | Effort |
|------|--------|--------|
| 1 | Add `.github/workflows/ci.yml` (Phase 1) | Low |
| 2 | Add Rust cache action | Low |
| 3 | Document self-hosted runner setup in README or `docs/` | Low |
| 4 | Add `scripts/e2e-test.sh` for local e2e | Medium |
| 5 | Add `e2e.yml` (runs-on: self-hosted when available) | Medium |
| 6 | If org has GPU runners: configure `e2e.yml` for them | Low |

---

## Security Notes

- **FERRIS_AUTH_TOKEN in CI:** Use a non-secret value for CI (e.g. `ci-test-token`). No production secrets.
- **Secrets:** Store in GitHub Actions secrets only if needed for external services.
- **E2E network:** Host binds `0.0.0.0:50051`; in CI it runs only on localhost.

---

## Summary

| Phase | Trigger | Runner | What | Cost |
|-------|---------|--------|------|------|
| 1 | Every push | `ubuntu-latest`, `windows-latest`, `macos-latest` | Client: all 3. Host: ubuntu + windows only | Free (public repos) |
| 2 | Every push (or `workflow_dispatch`) | Self-hosted GPU or org GPU runner | Full e2e | Machine or per-minute |

**Note:** Pre-commit hooks (`.githooks/pre-commit`) run locally before each commit. CI (`.github/workflows/ci.yml`) runs on GitHub after each push.
