---
id: T02
parent: S06
milestone: M001
provides:
  - Full end-to-end pipeline integration test exercising every phase (provision → install git → mock assay binary → write manifest → exec via AssayInvoker → collect → teardown)
key_files:
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - Mock assay binary placed at /usr/local/bin/assay (on PATH) so AssayInvoker::build_run_command() finds it without path injection
  - Test manually chains phases rather than calling run_with_cancellation() — necessary to inject mock assay setup between provision and exec
patterns_established:
  - Write mock binary via base64-encoded exec then chmod +x; place at /usr/local/bin/<name> for PATH visibility
  - Use AssayInvoker::write_manifest_to_container + build_run_command for the real invocation path (not hand-rolled exec)
  - Assert on exec handle exit_code + stdout/stderr inline for immediate failure visibility
observability_surfaces:
  - cargo test -p smelt-cli --test docker_lifecycle -- test_full_e2e_pipeline --nocapture shows full exec output
  - git log --oneline smelt/e2e-result on the temp repo confirms collection succeeded
  - docker ps -a --filter label=smelt.job -q after test → empty (no container leak)
duration: ~13 minutes
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Add full end-to-end pipeline integration test

**Added `test_full_e2e_pipeline` — a new Docker integration test that exercises every phase of the smelt pipeline using a real mock assay binary placed on PATH, and confirms the result branch is created on the host after teardown.**

## What Happened

Added `test_full_e2e_pipeline` to `crates/smelt-cli/tests/docker_lifecycle.rs` (after `test_collect_creates_target_branch`, before the timeout tests). The test:

1. Creates a temp git repo with an initial commit and records `base_ref`
2. Builds a manifest with `job.base_ref = base_ref` and `merge.target = "smelt/e2e-result"`
3. Provisions a Docker container and installs git via `apk add --no-cache git`
4. Writes a mock `assay` shell script to `/usr/local/bin/assay` via base64-encoded exec + chmod — placed on PATH exactly as `AssayInvoker::build_run_command()` expects
5. Writes the smelt manifest into the container via `AssayInvoker::write_manifest_to_container()`
6. Constructs and runs the assay command via `AssayInvoker::build_run_command()` → `["assay", "run", "/tmp/smelt-manifest.toml", "--timeout", "60"]`
7. Calls `ResultCollector::collect(&base_ref, "smelt/e2e-result")` on the host repo
8. Tears down the container and verifies absence
9. Asserts: `!no_changes`, `commit_count >= 1`, `files_changed.contains("assay-output.txt")`, and target branch exists on host

The mock assay script: `cd /workspace`, configure git identity, create `assay-output.txt`, commit, exit 0.

## Verification

- `cargo test -p smelt-cli --test docker_lifecycle -- test_full_e2e_pipeline` → `ok` (1 passed, 12.35s)
- `cargo test -p smelt-cli --test docker_lifecycle` → `ok` (18 passed, 0 failed, 14.11s)
- `docker ps -a --filter label=smelt.job -q` after test → empty (no container leak)

Slice-level checks (partial — T02 is second of two tasks):
- ✅ `cargo test -p smelt-cli --test docker_lifecycle 2>&1 | tail -5` — all 18 tests pass, zero failures
- ✅ `cargo test -p smelt-cli --test docker_lifecycle -- test_full_e2e_pipeline` — passes
- ⏳ `cargo test --workspace 2>&1 | tail -5` — not yet verified (final slice check)
- ⏳ `cargo test -p smelt-cli --test docker_lifecycle -- test_e2e_assay_failure_no_orphans` — this test is not in the plan for T02 (no such test exists; S06 plan lists it as a future concern)

## Diagnostics

- Run with `--nocapture` to see full exec output: `cargo test -p smelt-cli --test docker_lifecycle -- test_full_e2e_pipeline --nocapture`
- On mock assay failure: `assert_eq!(handle.exit_code, 0, "assay run should exit 0: stdout={} stderr={}", ...)` — stdout and stderr printed inline
- On collect failure: `expect("collect should succeed")` propagates the anyhow error chain
- Container cleanup: teardown is always called; absence verified via bollard inspect (404 = removed)

## Deviations

- The slice plan mentions a `test_e2e_assay_failure_no_orphans` test; that test does not appear in T02's task plan and was not implemented. T02 scope was strictly the happy-path `test_full_e2e_pipeline`.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/tests/docker_lifecycle.rs` — added `test_full_e2e_pipeline` test (~90 lines) covering the complete phase chain
