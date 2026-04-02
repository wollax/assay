---
id: S04
parent: M001
milestone: M001
provides:
  - ResultCollector<G: GitOps> with collect() — reads git state and creates/updates target branch
  - BranchCollectResult struct for collection outcomes (branch, commit_count, files_changed, subjects, no_changes)
  - Collection phase wired into execute_run() between Assay completion and teardown
  - Docker integration test verifying end-to-end branch creation through bind-mount pipeline
requires:
  - slice: S03
    provides: Repo mount logic (bind-mount at /workspace), Assay invocation, output streaming, execute_run() orchestration flow
  - slice: S01
    provides: MergeConfig (target branch), JobMeta (base_ref), GitOps trait, resolve_repo_path()
affects:
  - S06
key_files:
  - crates/smelt-core/src/collector.rs
  - crates/smelt-core/src/lib.rs
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
  - crates/smelt-cli/Cargo.toml
key_decisions:
  - D031: ResultCollector generic over <G: GitOps> (RPITIT not object-safe)
  - D032: Host-side collection — read host repo directly, not via Docker exec
  - D033: Target branch force-update with delete + recreate, warn with hashes
patterns_established:
  - ResultCollector uses generics (not dyn) for GitOps due to RPITIT
  - Collection phase in execute_run() is inside async block — teardown always runs even on collection failure
  - Test helper pattern: setup_test_repo() + add_commit() + head_hash() for collector tests
  - Docker integration test pattern for collector: temp git repo → provision → mock script creates commits → collect on host → assert branch
observability_surfaces:
  - "tracing::info for HEAD/base_ref resolution, commit count, target branch creation"
  - "tracing::warn for no-changes, dirty worktree, target branch overwrite with old/new hashes"
  - "stderr lifecycle: 'Collecting results...' before, 'Collected: N commits on branch X, M files changed' after"
  - "stderr 'No new commits from Assay — target branch not created' when no changes"
  - "Collection errors surface as anyhow 'failed to collect results' with non-zero exit"
drill_down_paths:
  - .kata/milestones/M001/slices/S04/tasks/T01-SUMMARY.md
  - .kata/milestones/M001/slices/S04/tasks/T02-SUMMARY.md
duration: 15m
verification_result: passed
completed_at: 2026-03-17
---

# S04: Result Collection & Branch Output

**ResultCollector extracts git state from bind-mounted repo after Assay completion and creates the target branch on the host, with 6 unit+integration tests and stderr lifecycle messages.**

## What Happened

T01 created `ResultCollector<G: GitOps>` in `crates/smelt-core/src/collector.rs` — a generic struct that takes a `GitOps` implementation and repo path, then implements `collect(base_ref, target_branch)`. The flow: resolve HEAD and base_ref → compare (early return if equal = no changes) → check dirty worktree (warn) → count commits between base and HEAD → get changed files and commit subjects → handle pre-existing target branch (delete + recreate with warning) → create branch at HEAD → return `BranchCollectResult`. Five unit tests cover happy path, no-changes, target-already-exists, multiple commits, and dirty worktree — all using real temp git repos.

T02 wired collection into `execute_run()` as Phase 8, after Assay exit code check but inside the async block so teardown is guaranteed. The phase resolves the repo path, constructs `GitCli`, calls `collect()`, and reports results to stderr. Added a Docker integration test (`test_collect_creates_target_branch`) that provisions a container with a bind-mounted temp repo, runs a mock script that creates commits in `/workspace`, then verifies the target branch exists on the host with expected commits.

Key design choice: since bind-mount (D013) means Assay's commits are already on the host filesystem, the collector reads the host repo directly rather than extracting state from inside the container via Docker exec (D032). This is simpler and avoids container-side git dependency.

## Verification

- `cargo test -p smelt-core -- collector::tests` — **5/5 passed** (basic, no_changes, target_exists, multiple_commits, dirty_worktree)
- `cargo test -p smelt-cli --test docker_lifecycle -- collect` — **1/1 passed** (end-to-end Docker pipeline)
- `cargo test --workspace` — **121 tests passed, 0 failed**, zero regressions (was 117 after S03)

## Deviations

None.

## Known Limitations

- Collection assumes single-branch linear history — merge strategies (squash, rebase) not yet implemented
- No verification that collected commits actually came from Assay (any commits after base_ref are collected)
- `which::which("git")` used in run.rs instead of `preflight()` to avoid cwd resolution issues — slightly inconsistent with how git is found elsewhere

## Follow-ups

None — S05 and S06 are already planned and cover the remaining gaps.

## Files Created/Modified

- `crates/smelt-core/src/collector.rs` — new: BranchCollectResult, ResultCollector<G: GitOps>, collect(), 5 unit tests
- `crates/smelt-core/src/lib.rs` — added `pub mod collector` and re-exports
- `crates/smelt-cli/src/commands/run.rs` — Phase 8 (collection) in execute_run() after Assay success
- `crates/smelt-cli/tests/docker_lifecycle.rs` — `test_collect_creates_target_branch` integration test
- `crates/smelt-cli/Cargo.toml` — `which` as regular + dev dependency

## Forward Intelligence

### What the next slice should know
- Collection runs between Assay completion and teardown inside the async block — S05's signal handling must ensure collection either completes or is skipped cleanly before teardown on Ctrl+C
- The `execute_run()` flow is now: provision → exec health → mount verification → write manifest → run assay → check exit → **collect** → teardown

### What's fragile
- The async block pattern in `execute_run()` is getting long (8 phases) — S05 adding timeout/signal handling will add more complexity to this function. Consider extracting phases into helper functions if it grows further.
- `which::which("git")` vs `preflight()` inconsistency — not breaking but could confuse future readers

### Authoritative diagnostics
- `SMELT_LOG=info cargo test -p smelt-core -- collector` shows every collector decision point
- `git log --oneline <merge.target>` on host repo after `smelt run` is the definitive proof of collection
- stderr output during `smelt run` shows collection lifecycle without needing log level changes

### What assumptions changed
- No assumptions changed — bind-mount strategy (D013) worked exactly as expected for host-side collection
