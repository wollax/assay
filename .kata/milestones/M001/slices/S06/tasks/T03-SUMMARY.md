---
id: T03
parent: S06
milestone: M001
provides:
  - Multi-session manifest round-trip integration test (test_multi_session_e2e)
  - Error-path teardown guarantee integration test (test_e2e_assay_failure_no_orphans)
key_files:
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - Orphan-check in failure test uses job-specific label filter (label=smelt.job=failure-no-orphans) rather than the generic label key, avoiding false positives when other tests run concurrently
patterns_established:
  - Mock assay for manifest-only tests (no git needed): write script that checks /tmp/smelt-manifest.toml exists and exits 0; place at /usr/local/bin/assay via base64 exec
  - Concurrent-safe orphan check: filter by label=smelt.job=<job-name> not just label=smelt.job to isolate per-job container assertions
observability_surfaces:
  - test_e2e_assay_failure_no_orphans asserts docker ps output directly; leaked container IDs appear in assertion message on failure
  - test_multi_session_e2e prints full manifest TOML on assertion failure so missing fields are immediately visible
duration: ~25 minutes
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T03: Add multi-session and error-path integration tests

**Added `test_multi_session_e2e` and `test_e2e_assay_failure_no_orphans` to `docker_lifecycle.rs`, completing the S06 integration test coverage for manifest serialization and teardown-on-failure.**

## What Happened

Two tests were added to `crates/smelt-cli/tests/docker_lifecycle.rs` after `test_full_e2e_pipeline`:

**`test_multi_session_e2e`** (~60 lines): Provisions a container, builds a 2-session manifest with `session-two` depending on `session-one`, writes it via `AssayInvoker::write_manifest_to_container`, reads it back with `cat /tmp/smelt-manifest.toml`, and asserts both session names and the `depends_on = ["session-one"]` relationship appear in the serialized TOML. Then executes a mock assay binary (that verifies the manifest file exists) via `build_run_command`, confirming exit 0.

**`test_e2e_assay_failure_no_orphans`** (~50 lines): Provisions a container, writes a mock assay that immediately `exit 1`, writes the smelt manifest, executes assay (asserting exit code 1), calls teardown (asserting `Ok(())`), and verifies via bollard inspect that the container is removed. Then runs `docker ps --filter label=smelt.job=failure-no-orphans -q` and asserts empty output.

**Concurrent-test fix**: The initial orphan check used `label=smelt.job` (key-only filter), which returned containers from all other concurrently-running tests — causing a spurious failure in `cargo test --workspace`. Switched to `label=smelt.job=failure-no-orphans` (key=value filter) so the check is scoped to only the job created by this specific test.

## Verification

```
cargo test -p smelt-cli --test docker_lifecycle -- multi_session
# → test test_multi_session_e2e ... ok (10.4s)

cargo test -p smelt-cli --test docker_lifecycle -- failure_no_orphans
# → test test_e2e_assay_failure_no_orphans ... ok (10.6s)

cargo test --workspace 2>&1 | grep "^test result"
# docker_lifecycle: test result: ok. 20 passed; 0 failed
# smelt-core: test result: ok. 10 passed; 0 failed
# smelt-core (doctests): test result: ok. 0 passed
# dry_run: test result: FAILED (pre-existing, unrelated to T03 — run_without_dry_run_attempts_docker)
```

The `dry_run` failure is pre-existing (confirmed by checking against T02 HEAD) and outside T03 scope.

## Diagnostics

- Run with `--nocapture` to see exec output: `cargo test -p smelt-cli --test docker_lifecycle -- test_multi_session_e2e --nocapture`
- On manifest assertion failure: full TOML content printed in assertion message
- On orphan check failure: leaked container IDs printed in assertion message with format `"no failure-no-orphans containers should remain after teardown, got:\n{remaining}"`

## Deviations

- Orphan check scoped to job-specific label (`label=smelt.job=failure-no-orphans`) instead of global label key — required for correctness under concurrent test execution; does not weaken the guarantee (only tests the exact job created by this test).

## Known Issues

- `run_without_dry_run_attempts_docker` in `dry_run` test suite fails (pre-existing, not introduced by T03).

## Files Created/Modified

- `crates/smelt-cli/tests/docker_lifecycle.rs` — added `test_multi_session_e2e` and `test_e2e_assay_failure_no_orphans`
