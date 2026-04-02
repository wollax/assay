# S04: Result Collection & Branch Output

**Goal:** After `smelt run` completes, the target branch specified in `merge.target` exists on the host repository containing the commits from Assay's work, extracted from the bind-mounted repo before teardown.
**Demo:** Run `smelt run` against a manifest with a mock Assay that creates commits, then `git log <target-branch>` on the host repo shows those commits.

## Must-Haves

- `ResultCollector` struct in `collector.rs` that reads git state from the host repo after Assay completes and creates/updates the target branch
- Handles: new commits present (success), no new commits (report "no changes"), target branch already exists (force-update with warning), dirty working tree (detect and warn)
- `execute_run()` calls collection between Assay completion and teardown
- Collection result includes branch name, commit count, and changed files
- Unit tests verify collector logic against real temp git repos (no Docker needed)
- Integration test verifies end-to-end collection through `execute_run()` with Docker

## Proof Level

- This slice proves: integration
- Real runtime required: yes (Docker for integration test; real git repos for unit tests)
- Human/UAT required: no

## Verification

- `cargo test -p smelt-core -- collector::tests` — unit tests for ResultCollector logic (no Docker)
- `cargo test -p smelt-cli --test docker_lifecycle -- collect` — integration test verifying branch creation through Docker pipeline
- `cargo test --workspace` — all tests pass, zero regressions

## Observability / Diagnostics

- Runtime signals: `tracing::info` for base_ref resolution, HEAD position, commit count, files changed, target branch created/updated; `tracing::warn` for no-changes and dirty-worktree cases; stderr lifecycle messages for "Collecting results..." and "Results collected: N commits on <target>"
- Inspection surfaces: `git log --oneline <merge.target>` on host repo after run; `SMELT_LOG=info smelt run` shows collector progress
- Failure visibility: `SmeltError::Provider` with operation "collect" for collection failures; specific messages for "no commits found", "failed to create target branch"; non-zero exit from `smelt run` on collection failure
- Redaction constraints: none (no secrets in git operations)

## Integration Closure

- Upstream surfaces consumed: `manifest.rs` (`MergeConfig.target`, `JobMeta.base_ref`, `resolve_repo_path()`), `git/mod.rs` (`GitOps` trait — `rev_parse`, `branch_exists`, `branch_create`, `diff_name_only`, `log_subjects`, `rev_list_count`, `worktree_is_dirty`), `docker.rs` (`DockerProvider` provision/exec/teardown), `assay.rs` (`AssayInvoker`), `run.rs` (`execute_run()` orchestration flow)
- New wiring introduced in this slice: `collector.rs` module with `ResultCollector`, wired into `execute_run()` between Assay completion and teardown; `BranchCollectResult` return type for collection outcome
- What remains before the milestone is truly usable end-to-end: S05 (timeout, monitoring, Ctrl+C handling), S06 (multi-session ordering, full pipeline integration test)

## Tasks

- [x] **T01: Create ResultCollector with unit tests** `est:45m`
  - Why: The core collection logic — reading git state and creating the target branch — must work correctly in isolation before wiring into the Docker pipeline
  - Files: `crates/smelt-core/src/collector.rs`, `crates/smelt-core/src/lib.rs`, `crates/smelt-core/src/error.rs`
  - Do: Create `ResultCollector` as a generic struct over `<G: GitOps>`. Implement `collect()` that: (1) resolves HEAD on the repo, (2) compares to base_ref to detect new commits, (3) creates or force-updates target branch to HEAD, (4) returns `BranchCollectResult` with branch name/commit count/files. Handle edge cases: no new commits, target already exists, dirty worktree. Add unit tests using `setup_test_repo()` pattern from `cli.rs` tests.
  - Verify: `cargo test -p smelt-core -- collector::tests` — all pass
  - Done when: `ResultCollector::collect()` correctly creates target branches in temp repos with 5+ test cases covering happy path and edge cases

- [x] **T02: Wire collector into execute_run() and add Docker integration test** `est:30m`
  - Why: The collector must be called at the right point in the orchestration flow and verified end-to-end with real Docker
  - Files: `crates/smelt-cli/src/commands/run.rs`, `crates/smelt-cli/tests/docker_lifecycle.rs`
  - Do: In `execute_run()`, after Assay exits successfully, instantiate `ResultCollector` with `GitCli` for the resolved repo path and call `collect()`. Print result summary to stderr. Add integration test that: provisions container, runs a mock script that creates commits in `/workspace`, collects results, verifies target branch exists with expected commits on the host repo.
  - Verify: `cargo test -p smelt-cli --test docker_lifecycle -- collect` passes; `cargo test --workspace` passes with zero regressions
  - Done when: `smelt run` with a mock Assay that creates commits produces the target branch on the host repo, verified by integration test

## Files Likely Touched

- `crates/smelt-core/src/collector.rs` (new)
- `crates/smelt-core/src/lib.rs`
- `crates/smelt-cli/src/commands/run.rs`
- `crates/smelt-cli/tests/docker_lifecycle.rs`
