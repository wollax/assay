---
id: T02
parent: S04
milestone: M001
provides:
  - ResultCollector wired into execute_run() — collection phase after Assay success
  - Docker integration test verifying end-to-end branch creation through bind-mount pipeline
key_files:
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
  - crates/smelt-cli/Cargo.toml
key_decisions:
  - Used which::which("git") in run.rs rather than preflight() to avoid repo-root resolution against cwd (collection needs the manifest's repo path, not the invocation cwd)
patterns_established:
  - Collection phase in execute_run() is inside the async block, so teardown always runs even on collection failure
  - Docker integration test pattern for collector: setup temp git repo → provision → mock script creates commits → collect on host → assert branch exists
observability_surfaces:
  - stderr "Collecting results..." before collection starts
  - stderr "Collected: N commits on branch 'X', M files changed" on success
  - stderr "No new commits from Assay — target branch not created" when no changes
  - Collection errors surface as anyhow context "failed to collect results" with non-zero exit
duration: 10m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Wire collector into execute_run() and add Docker integration test

**Wired `ResultCollector::collect()` into `execute_run()` after Assay success, with stderr lifecycle messages and a Docker integration test verifying target branch creation on the host repo.**

## What Happened

Modified `execute_run()` in `run.rs` to add a Phase 8 (collection) after Assay exit code check. The collection phase resolves the repo path from the manifest, constructs a `GitCli`, and calls `ResultCollector::collect()` with `base_ref` and `merge.target`. Results are reported to stderr. The phase runs inside the async block, so teardown is guaranteed regardless of collection outcome.

Added `test_collect_creates_target_branch` to `docker_lifecycle.rs` — creates a real temp git repo with initial commit, provisions a Docker container with bind-mount, runs a mock shell script that creates a file and commits in `/workspace`, then calls `ResultCollector::collect()` on the host and verifies: target branch exists, commit count >= 1, `result.txt` in files_changed, `no_changes == false`.

Added `which` as both a regular and dev dependency for `smelt-cli` (needed for `which::which("git")` in run.rs and the integration test).

## Verification

- `cargo test -p smelt-core -- collector::tests` — 5/5 pass (T01 unit tests)
- `cargo test -p smelt-cli --test docker_lifecycle -- collect` — 1/1 pass
- `cargo test --workspace` — 121 tests pass, 0 failures, 0 regressions

### Slice-level verification:
- ✅ `cargo test -p smelt-core -- collector::tests` — 5/5 pass
- ✅ `cargo test -p smelt-cli --test docker_lifecycle -- collect` — 1/1 pass
- ✅ `cargo test --workspace` — all pass, zero regressions

## Diagnostics

- Run `smelt run <manifest>` and check stderr for "Collecting results..." and "Collected: N commits on branch 'X', M files changed"
- After run: `git branch -a` on the repo shows the target branch; `git log --oneline <merge.target>` shows collected commits
- `SMELT_LOG=info smelt run` shows tracing-level collector progress (HEAD/base resolution, commit count, branch creation)
- Collection failures surface as `anyhow` errors with context "failed to collect results" and non-zero exit

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/commands/run.rs` — Added Phase 8 (collection) in execute_run() async block after Assay success
- `crates/smelt-cli/tests/docker_lifecycle.rs` — Added `test_collect_creates_target_branch` integration test
- `crates/smelt-cli/Cargo.toml` — Added `which` as regular + dev dependency
