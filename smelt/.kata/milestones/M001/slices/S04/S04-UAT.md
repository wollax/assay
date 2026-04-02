# S04: Result Collection & Branch Output — UAT

**Milestone:** M001
**Written:** 2026-03-17

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: Result collection is fully testable via automated git operations — the output is a branch with commits, verifiable by `git log` and `git branch`. No human experience or live-runtime observation needed beyond what the integration test already exercises.

## Preconditions

- Docker daemon running (`docker info` succeeds)
- Git installed and on PATH
- Working directory is the smelt repo root
- `cargo build` succeeds

## Smoke Test

Run `cargo test -p smelt-core -- collector::tests::test_collect_basic` — if it passes, the core collection logic works.

## Test Cases

### 1. Unit tests cover all collector edge cases

1. `cargo test -p smelt-core -- collector::tests`
2. **Expected:** 5/5 pass — basic, no_changes, target_exists, multiple_commits, dirty_worktree

### 2. Docker integration test verifies end-to-end pipeline

1. `cargo test -p smelt-cli --test docker_lifecycle -- collect`
2. **Expected:** 1/1 pass — container provisioned, mock script creates commits, target branch exists on host

### 3. Full workspace regression

1. `cargo test --workspace`
2. **Expected:** 121+ tests pass, 0 failures

## Edge Cases

### No new commits from Assay

1. Run collector where HEAD == base_ref
2. **Expected:** `no_changes: true` returned, no branch created, stderr says "No new commits"

### Target branch already exists

1. Run collector where target branch pre-exists pointing to different commit
2. **Expected:** Branch force-updated to new HEAD, warning logged with old/new hashes

### Dirty working tree

1. Run collector with uncommitted changes in repo
2. **Expected:** Collection proceeds for committed changes only, warning logged about dirty worktree

## Failure Signals

- `cargo test` failures in collector tests
- Target branch missing after `smelt run` completes
- Collection errors not surfacing in stderr
- Teardown running before collection completes (ordering bug)

## Requirements Proved By This UAT

- None — no `.kata/REQUIREMENTS.md` exists; M001 defines capabilities directly

## Not Proven By This UAT

- Multi-session result merging (S06 scope)
- Collection behavior under timeout/Ctrl+C (S05 scope)
- Real `assay orchestrate` producing commits (mock used; real integration in S06)
- Merge strategies beyond simple branch creation (deferred)

## Notes for Tester

- The Docker integration test requires a running Docker daemon — it skips gracefully if Docker is unavailable
- All tests use temp directories and temp git repos — no cleanup needed
- The `which` deprecation warnings in `docker_lifecycle.rs` are cosmetic (from `assert_cmd`) and don't affect test correctness
