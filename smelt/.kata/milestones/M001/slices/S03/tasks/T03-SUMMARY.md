---
id: T03
parent: S03
milestone: M001
provides:
  - execute_run() orchestrates full mount → write manifest → exec assay → teardown flow
  - Mock assay integration tests validating end-to-end path
  - Updated example manifest with local path guidance
key_files:
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
  - examples/job-manifest.toml
key_decisions:
  - Assay stdout/stderr streamed to stderr in execute_run() so lifecycle messages and assay output share the same channel
patterns_established:
  - base64-encoded mock scripts written into containers for integration testing
  - Lifecycle phase messages on stderr for each orchestration step (Writing manifest → Executing assay run → Assay complete)
observability_surfaces:
  - stderr lifecycle messages for each phase: "Provisioning container...", "Writing manifest...", "Executing assay run...", "Assay complete — exit code: N", "Container removed."
  - Assay stdout/stderr forwarded to stderr for visibility
  - Non-zero assay exit code surfaced as anyhow error with stderr content
duration: 1 step
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T03: Wire mount + assay invocation into CLI run and verify with mock

**Replaced health-check in `execute_run()` with full repo-mount + assay-invocation flow, verified with mock assay scripts in integration tests.**

## What Happened

Updated `execute_run()` to orchestrate the complete lifecycle: provision (with bind-mount) → write assay manifest to container → execute assay run command → teardown. The old health-check echo command was replaced with `AssayInvoker::build_manifest_toml()` → `write_manifest_to_container()` → `build_run_command()` → `provider.exec()`. Each phase prints a lifecycle message to stderr.

Added two new integration tests:
- `test_assay_mock_execution`: Creates a temp repo with a marker file, provisions a container, writes the manifest, then runs a mock shell script that validates the repo is mounted at `/workspace`, the manifest file exists at `/tmp/smelt-manifest.toml`, and session data is parseable. Asserts the mock finds 2 sessions (alpha, beta) and reads the marker file.
- `test_assay_mock_failure`: Writes a mock script that exits with code 1 and stderr output, verifies the non-zero exit code and error message are captured correctly.

Updated `test_cli_run_lifecycle` to expect the new lifecycle messages (Writing manifest, Executing assay run, Assay complete) instead of the old health-check message. Removed the success assertion since alpine doesn't have an `assay` binary.

Updated `examples/job-manifest.toml` to use `"."` as the repo path with comments explaining that `smelt run` requires an absolute local path.

## Verification

- `cargo test -p smelt-cli --test docker_lifecycle -- assay_mock` — 2 mock assay tests pass ✓
- `cargo test -p smelt-cli --test docker_lifecycle` — all 13 Docker lifecycle tests pass ✓
- `cargo test --workspace` — all 117 tests pass, zero regressions ✓
- `cargo run -- run examples/job-manifest.toml --dry-run` — dry-run works with updated manifest ✓

## Diagnostics

- `smelt run manifest.toml` stderr shows lifecycle phases: Provisioning → Writing manifest → Executing assay run → Assay complete (exit code) → Tearing down → Container removed
- `SMELT_LOG=info smelt run manifest.toml` adds structured tracing for manifest build, container write, and assay command construction
- Non-zero assay exit includes stderr content in the error message
- Manifest write failures surface as `SmeltError::Provider { operation: "write_manifest" }`

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/commands/run.rs` — replaced health-check with full assay orchestration flow
- `crates/smelt-cli/tests/docker_lifecycle.rs` — added `test_assay_mock_execution` and `test_assay_mock_failure`; updated `test_cli_run_lifecycle` for new lifecycle messages
- `crates/smelt-cli/Cargo.toml` — added `base64` dev-dependency for mock script encoding in tests
- `examples/job-manifest.toml` — changed repo to `"."` with local path guidance comments
