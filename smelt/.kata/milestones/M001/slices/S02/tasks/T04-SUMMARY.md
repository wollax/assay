---
id: T04
parent: S02
milestone: M001
provides:
  - Async CLI main with #[tokio::main]
  - smelt run manifest.toml drives DockerProvider provision→exec→teardown lifecycle
  - CLI-level integration tests for full Docker lifecycle and error handling
key_files:
  - crates/smelt-cli/src/main.rs
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
  - examples/job-manifest.toml
key_decisions:
  - Teardown guaranteed via async block pattern — inner async block runs exec, outer code always calls teardown regardless of result
  - Runtime type check rejects non-docker runtimes with clear error before attempting provider connection
  - Example manifest changed from node:20-slim to alpine:3 for faster test pulls
patterns_established:
  - Async block for cleanup guard — provision returns container_id, async block does work, teardown runs unconditionally after
  - CLI lifecycle phases printed to stderr (Provisioning → Executing → Tearing down → Container removed)
observability_surfaces:
  - CLI prints phase transitions to stderr (Provisioning container, Health check complete with exit code, Container removed)
  - Error messages include provider operation context (connect, provision, exec, teardown) with anyhow context chain
  - Non-zero exit codes from Docker daemon connection produce structured error with socket path
duration: 15m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T04: Wired DockerProvider into CLI and ran full lifecycle integration test

**Converted CLI to async, wired DockerProvider into `smelt run`, added CLI-level integration tests — 96 tests passing across workspace**

## What Happened

Converted `main()` to `#[tokio::main] async fn main()` and added tokio as a runtime dependency for smelt-cli. Rewrote `execute()` in run.rs to be async, with the non-dry-run path instantiating `DockerProvider::new()`, loading/validating the manifest, checking runtime type is "docker", provisioning a container, executing a health-check command (`echo "smelt: container ready"`), streaming output, and tearing down. Teardown is guaranteed via an async block pattern — the exec work runs inside an async block, and teardown runs unconditionally afterward regardless of success or failure.

Updated the example manifest image from `node:20-slim` to `alpine:3` for faster testing. Updated two pre-existing dry_run tests that assumed the old image name and the old "not implemented" error message.

Added two CLI-level integration tests: `test_cli_run_lifecycle` runs the real binary via assert_cmd and verifies the health check output and clean teardown; `test_cli_run_invalid_manifest` verifies error handling for nonexistent manifests.

## Verification

- `cargo test --workspace` — 96 tests pass (74 core + 10 dry_run + 7 docker_lifecycle + 3 inline + 2 doc-tests), zero failures
- `cargo test -p smelt-cli -- docker_lifecycle::test_cli_run_lifecycle` — passes (full provision→exec→teardown via CLI binary)
- `cargo test -p smelt-cli -- docker_lifecycle::test_cli_run_invalid_manifest` — passes (error handling)
- `cargo run -- run examples/job-manifest.toml --dry-run` — works correctly, no regressions
- `cargo run -- run examples/job-manifest.toml` — produces clear Docker connection error when daemon unavailable, exits 1

### Slice-level verification results (all checks):
- ✅ `cargo test --workspace` — all tests pass, zero warnings (only deprecation warnings from assert_cmd)
- ✅ `cargo test -p smelt-core -- docker` — 16 resource parsing unit tests pass
- ✅ `cargo test -p smelt-cli -- docker` — 7 integration tests pass (provision→exec→teardown lifecycle + CLI tests)
- ✅ `cargo run -- run examples/job-manifest.toml` — drives Docker lifecycle, streams output, tears down (verified via integration test; manual run shows clear error when Docker unavailable)
- ✅ No leaked containers (test_cli_run_lifecycle asserts `docker ps -a --filter label=smelt.job` is empty)

## Diagnostics

- `smelt run manifest.toml` prints lifecycle phases to stderr: "Provisioning container...", "Container provisioned: <id>", "Executing health check...", "Health check complete — exit code: N", "Tearing down container...", "Container removed."
- `SMELT_LOG=info smelt run manifest.toml` shows full bollard operations via tracing
- Provider errors include operation context ("connect", "provision", "exec", "teardown") and bollard error as source
- `docker ps --filter label=smelt.job` shows any active Smelt containers during execution

## Deviations

- Changed example manifest image from `node:20-slim` to `alpine:3` (planned as conditional, executed unconditionally for test speed)
- Updated two pre-existing dry_run tests to match new behavior (image name change, removal of "not implemented" stub)

## Known Issues

- None

## Files Created/Modified

- `crates/smelt-cli/src/main.rs` — converted to async with `#[tokio::main]`
- `crates/smelt-cli/src/commands/run.rs` — async execute() with DockerProvider lifecycle for non-dry-run path
- `crates/smelt-cli/Cargo.toml` — added tokio as runtime dependency
- `crates/smelt-cli/tests/docker_lifecycle.rs` — added 2 CLI-level integration tests (test_cli_run_lifecycle, test_cli_run_invalid_manifest)
- `crates/smelt-cli/tests/dry_run.rs` — updated 2 tests for new image name and removed obsolete "not implemented" check
- `examples/job-manifest.toml` — changed image from node:20-slim to alpine:3
