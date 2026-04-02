---
id: T01
parent: S06
milestone: M001
provides:
  - Clean docker_lifecycle baseline with both previously-failing tests passing
key_files:
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - Pre-clean uses label=smelt.job (all smelt containers) not a job-specific filter, so orphans from any test run are cleared
patterns_established:
  - Install alpine packages in-container via provider.exec before running scripts that depend on them
  - Pre-clean stale labeled containers at test start as a defensive guard against daemon accumulation
observability_surfaces:
  - none (test-only changes)
duration: 5m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Fix two pre-existing Docker integration test failures

**Surgical fixes to `docker_lifecycle.rs`: git installed in alpine before mock assay runs; stale smelt containers pre-cleaned before container-absence assertion.**

## What Happened

Both fixes were applied to `crates/smelt-cli/tests/docker_lifecycle.rs`:

1. **`test_collect_creates_target_branch`** — Added a `provider.exec()` call immediately after `provision()` to run `apk add --no-cache git` inside the alpine:3 container. Alpine does not ship with git, so the mock assay script (which calls `git config`, `git add`, `git commit`) was failing with "not found". The install step takes ~2s and exits 0, after which the mock script runs successfully.

2. **`test_cli_run_lifecycle`** — Added a pre-clean block at test start (after the docker-skip guard) that runs `docker ps -aq --filter label=smelt.job` and calls `docker rm -f` on every returned container ID. The filter uses `label=smelt.job` (key-only, no value) so orphans from any prior smelt test run are removed, not just those from this specific job name.

## Verification

```
cargo test -p smelt-cli --test docker_lifecycle -- collect
# test test_collect_creates_target_branch ... ok

cargo test -p smelt-cli --test docker_lifecycle -- cli_run_lifecycle
# test test_cli_run_lifecycle ... ok

cargo test -p smelt-cli --test docker_lifecycle
# 17 passed; 0 failed
```

## Diagnostics

Run `cargo test -p smelt-cli --test docker_lifecycle` — a clean 17/17 pass is the health signal for this baseline.

If `apk add --no-cache git` fails (network issue), the test fails with:
```
assertion `left == right` failed: git install should succeed
  left: <non-zero exit code>
```

## Deviations

The pre-clean filter was tightened from the initial partial attempt (`label=smelt.job=cli-lifecycle`) to the plan-specified `label=smelt.job` (all smelt containers) to match the task plan intent.

## Known Issues

none

## Files Created/Modified

- `crates/smelt-cli/tests/docker_lifecycle.rs` — git install added to `test_collect_creates_target_branch`; pre-clean block (all smelt containers) added to `test_cli_run_lifecycle`
