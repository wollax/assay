---
estimated_steps: 4
estimated_files: 3
---

# T02: Wire collector into execute_run() and add Docker integration test

**Slice:** S04 — Result Collection & Branch Output
**Milestone:** M001

## Description

Wire `ResultCollector` into `execute_run()` in `run.rs` so that after Assay exits successfully, Smelt collects the git state and creates the target branch. Add a Docker integration test that exercises the full pipeline: provision → mock-Assay-that-creates-commits → collect → verify target branch → teardown.

## Steps

1. In `crates/smelt-cli/src/commands/run.rs`, modify the async block in `execute_run()`:
   - After the Assay exit code check (currently the last step in the async block), add a collection phase
   - Resolve the repo path using `smelt_core::manifest::resolve_repo_path(&manifest.job.repo)`
   - Create a `GitCli` instance using `smelt_core::preflight()` (or construct from the resolved path)
   - Instantiate `ResultCollector::new(git, repo_path)`
   - Call `collector.collect(&manifest.job.base_ref, &manifest.merge.target).await`
   - On success: print summary to stderr ("Collected: {commit_count} commits on branch '{branch}', {files} files changed")
   - On `no_changes`: print warning to stderr ("No new commits from Assay — target branch not created")
   - On error: propagate via `?` (teardown still runs because this is inside the async block)

2. Add a Docker integration test in `crates/smelt-cli/tests/docker_lifecycle.rs`:
   - `test_collect_creates_target_branch` (marked `#[ignore]` for CI, follows D024 pattern)
   - Test setup:
     a. Create a temp git repo with initial commit (reuse `setup_test_repo()` pattern or create inline)
     b. Record the initial HEAD hash as base_ref
     c. Create a manifest with `repo` pointing to the temp repo, `base_ref` set to initial HEAD, `merge.target` set to `"smelt/result"`
     d. Build a mock Assay script that creates a file and commits it in `/workspace`
   - Test execution:
     a. Provision container with bind-mount to the temp repo
     b. Write the mock script to the container via exec (base64 pattern from S03)
     c. Execute the mock script (instead of real Assay)
     d. Call `ResultCollector::collect()` on the host repo
     e. Teardown container
   - Assertions:
     a. Target branch `"smelt/result"` exists on the host repo
     b. `rev_list_count("smelt/result", base_ref) >= 1`
     c. `diff_name_only(base_ref, "smelt/result")` contains the file created by mock script
     d. `BranchCollectResult.no_changes == false`

3. Verify: `cargo test -p smelt-cli --test docker_lifecycle -- collect` passes

4. Run full workspace tests: `cargo test --workspace` — zero regressions

## Must-Haves

- [ ] `execute_run()` calls `ResultCollector::collect()` after Assay success, before teardown
- [ ] Collection failure doesn't skip teardown (already guaranteed by async-block pattern D026)
- [ ] Stderr messages for collection progress and result summary
- [ ] Integration test verifies target branch created on host repo through Docker pipeline
- [ ] All existing tests pass without regression

## Verification

- `cargo test -p smelt-cli --test docker_lifecycle -- collect` — 1/1 pass (requires Docker)
- `cargo test --workspace` — zero regressions
- `cargo test -p smelt-core -- collector::tests` — still passing (from T01)

## Observability Impact

- Signals added/changed: stderr lifecycle messages "Collecting results..." and "Collected: N commits on branch 'X', M files changed" (or "No new commits from Assay")
- How a future agent inspects this: run `smelt run` against a manifest and check stderr output; `git branch -a` on the repo after run shows target branch
- Failure state exposed: collection errors surface as `anyhow` errors in `execute_run()` with context "failed to collect results"; non-zero exit code from `smelt run`

## Inputs

- `crates/smelt-core/src/collector.rs` — `ResultCollector` and `BranchCollectResult` from T01
- `crates/smelt-cli/src/commands/run.rs` — current `execute_run()` flow (S03 state)
- `crates/smelt-cli/tests/docker_lifecycle.rs` — existing test patterns (D024 skip pattern, base64 mock script pattern from S03)
- `crates/smelt-core/src/manifest.rs` — `resolve_repo_path()` for repo path resolution
- `crates/smelt-core/src/git/mod.rs` — `preflight()` for GitCli construction

## Expected Output

- `crates/smelt-cli/src/commands/run.rs` — modified with collection phase in `execute_run()`
- `crates/smelt-cli/tests/docker_lifecycle.rs` — new `test_collect_creates_target_branch` test
- Target branch verified on host repo after Docker-based mock Assay execution
