---
id: S06
parent: M001
milestone: M001
provides:
  - Integration test suite exercising every phase of the smelt pipeline end-to-end with real Docker
  - test_full_e2e_pipeline: provision → git install → mock assay binary → write manifest → exec via AssayInvoker → collect → teardown
  - test_multi_session_e2e: 2-session manifest with depends_on serialized correctly into container
  - test_e2e_assay_failure_no_orphans: assay exit 1 → teardown → zero container orphans
  - Fixed two pre-existing baseline failures (test_collect_creates_target_branch, test_cli_run_lifecycle)
requires:
  - slice: S04
    provides: ResultCollector (branch extraction pipeline), MergeConfig (target branch name)
  - slice: S05
    provides: JobMonitor, run_with_cancellation, timeout enforcement, signal handling
affects: []
key_files:
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - D039: E2E tests chain phases manually — run_with_cancellation() cannot inject mock assay setup step
  - D040: Mock assay placed at /usr/local/bin/assay on PATH; matches AssayInvoker::build_run_command() bare "assay" invocation
  - D041: Pre-clean orphan smelt containers at test start for tests asserting container absence
  - D042: Failure-path orphan check uses job-specific label value to avoid false positives from concurrent tests
patterns_established:
  - Phase-chaining directly in tests: provision → exec setup → exec workload → collect → teardown — mirrors test_collect_creates_target_branch
  - Mock binary delivery via base64-encoded exec + chmod +x; PATH placement at /usr/local/bin
  - Concurrent-safe orphan assertion: label=smelt.job=<job-name> (key=value), not label=smelt.job (key-only)
  - Install alpine packages via provider.exec before running scripts that depend on them
observability_surfaces:
  - cargo test -p smelt-cli --test docker_lifecycle --nocapture — full exec output per phase
  - docker ps -a --filter label=smelt.job=<name> -q after test → empty means no container leak
  - git log --oneline smelt/e2e-result on temp repo confirms ResultCollector succeeded
drill_down_paths:
  - .kata/milestones/M001/slices/S06/tasks/T01-SUMMARY.md
  - .kata/milestones/M001/slices/S06/tasks/T02-SUMMARY.md
  - .kata/milestones/M001/slices/S06/tasks/T03-SUMMARY.md
duration: ~43 minutes total (T01: 5m, T02: ~13m, T03: ~25m)
verification_result: passed
completed_at: 2026-03-17
---

# S06: End-to-End Integration

**Integration test suite covering the full smelt pipeline — provision → install → mock assay → exec → collect → teardown — with multi-session manifest round-trip and teardown-on-failure guarantees, all verified against a real Docker daemon.**

## What Happened

Three tasks fixed baseline failures and built out the complete S06 integration test coverage.

**T01** repaired two pre-existing failures in `docker_lifecycle.rs`. `test_collect_creates_target_branch` failed because Alpine ships without git — the fix adds `apk add --no-cache git` via `provider.exec()` immediately after provisioning. `test_cli_run_lifecycle` accumulated stale containers across runs because it asserted container absence without pre-cleaning; the fix adds a pre-clean block using `docker ps -aq --filter label=smelt.job` at test start. Both tests moved from failing to `ok`, establishing a clean 17/17 baseline.

**T02** added `test_full_e2e_pipeline` — the core S06 deliverable. The test manually chains all eight pipeline phases: create temp git repo with initial commit; record `base_ref`; build manifest with `merge.target = "smelt/e2e-result"`; provision container; install git; write mock assay binary to `/usr/local/bin/assay` via base64-encoded exec + chmod (placed on PATH so `AssayInvoker::build_run_command()`'s bare `"assay"` invocation resolves without path injection); write smelt manifest via `AssayInvoker::write_manifest_to_container()`; exec assay via `AssayInvoker::build_run_command()`; collect via `ResultCollector::collect()`; teardown and verify container removal; assert `!no_changes`, `commit_count >= 1`, `files_changed.contains("assay-output.txt")`, and target branch exists on host. The test passes in ~12s.

**T03** added `test_multi_session_e2e` and `test_e2e_assay_failure_no_orphans`. The multi-session test builds a 2-session manifest where `session-two` depends on `session-one`, writes it to the container via `AssayInvoker::write_manifest_to_container()`, reads it back with `cat /tmp/smelt-manifest.toml`, and asserts both session names and the `depends_on = ["session-one"]` relationship appear in the serialized TOML. The failure-path test provisions a container, writes a mock assay that immediately exits 1, asserts exit code 1 from `provider.exec()`, calls teardown, and confirms no container orphans via both bollard inspect (404) and `docker ps --filter label=smelt.job=failure-no-orphans -q`. An early iteration used `label=smelt.job` (key-only) for the orphan check, which returned containers from other concurrent tests and caused spurious failures in `cargo test --workspace`; the fix scopes the filter to the job-specific label value.

## Verification

```
cargo test -p smelt-cli --test docker_lifecycle 2>&1 | tail -5
# test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.36s

cargo test -p smelt-cli --test docker_lifecycle -- test_full_e2e_pipeline
# test test_full_e2e_pipeline ... ok

cargo test -p smelt-cli --test docker_lifecycle -- test_e2e_assay_failure_no_orphans
# test test_e2e_assay_failure_no_orphans ... ok

cargo test --workspace 2>&1 | grep "^test result"
# smelt-core:         ok. 10 passed; 0 failed
# smelt-core doctest: ok. 0 passed; 0 failed
# docker_lifecycle:   ok. 20 passed; 0 failed
# dry_run:            FAILED. 9 passed; 1 failed  ← pre-existing, confirmed before S06
```

The `run_without_dry_run_attempts_docker` failure in `dry_run.rs` is pre-existing — confirmed by running against the commit before any S06 changes. It is outside S06 scope and does not affect the milestone definition of done.

## Requirements Advanced

No `.kata/REQUIREMENTS.md` exists. Operating in legacy compatibility mode per M001-ROADMAP.md guidance.

## Requirements Validated

No `.kata/REQUIREMENTS.md` exists.

## New Requirements Surfaced

No `.kata/REQUIREMENTS.md` exists.

## Requirements Invalidated or Re-scoped

No `.kata/REQUIREMENTS.md` exists.

## Deviations

- `test_e2e_assay_failure_no_orphans` orphan check scoped to `label=smelt.job=<job-name>` instead of the task-plan's `label=smelt.job`. The plan-specified key-only filter caused spurious failures under concurrent test execution. Job-specific key=value filter is strictly correct and does not weaken the guarantee.

## Known Limitations

- `run_without_dry_run_attempts_docker` in `dry_run.rs` remains failing (pre-existing). That test asserts Docker is unavailable but the test environment has Docker running; the assertion logic in the test is incorrect. Not introduced by S06.
- `run_with_cancellation()` is not exercised end-to-end by any integration test — the E2E tests deliberately bypass it to inject mock assay between provision and exec phases (D039). The function is covered by S05's cancellation and timeout tests.
- No test exercises credential injection into a real container — credential passthrough is tested at the unit level in S01/S02 but not at the integration level.

## Follow-ups

- Fix `run_without_dry_run_attempts_docker` (pre-existing failure; test logic is incorrect — not S06 introduced)
- Add an integration test that runs `run_with_cancellation()` with a real mock assay that runs long, then cancels via the oneshot channel — covers the top-level entrypoint path
- Consider adding credential injection integration test once a real Assay binary is available in CI

## Files Created/Modified

- `crates/smelt-cli/tests/docker_lifecycle.rs` — T01: git install + pre-clean fixes; T02: test_full_e2e_pipeline; T03: test_multi_session_e2e + test_e2e_assay_failure_no_orphans

## Forward Intelligence

### What the next slice should know
- M001 is complete. All six slices are done. The next work unit is a new milestone.
- The one pre-existing test failure (`run_without_dry_run_attempts_docker`) should be fixed before starting a new milestone — it adds noise to `cargo test --workspace`.

### What's fragile
- `apk add --no-cache git` in tests depends on network reachability to Alpine CDN — tests that install packages will fail in air-gapped CI environments. Consider using a pre-built test image with git already installed.
- The base64 mock binary write pattern is verbose but reliable. Any expansion of mock binary complexity (multi-line scripts, heredocs) should stay with base64 to avoid shell quoting issues.

### Authoritative diagnostics
- `docker ps -a --filter label=smelt.job -q` — authoritative check for container leak across all smelt tests
- `cargo test -p smelt-cli --test docker_lifecycle --nocapture` — full exec output per phase; most useful for debugging mock assay failures

### What assumptions changed
- Pre-existing test isolation assumption: tests were assumed to run sequentially, but `cargo test --workspace` runs integration test binaries in parallel. The orphan-check scope change (D042) was required because of this reality.
