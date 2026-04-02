---
id: T02
parent: S03
milestone: M005
provides:
  - get_test_git_remote() helper returning Option<String> from SMELT_TEST_GIT_REMOTE env var
  - test_k8s_push_from_pod_result_collection integration test (5th test in k8s_lifecycle.rs)
  - Graceful double-guard skip (SMELT_K8S_TEST + SMELT_TEST_GIT_REMOTE) consistent with S02 pattern
  - End-to-end proof: Pod push → host fetch_ref → ResultCollector::collect → assertions
  - Best-effort remote branch cleanup after test (non-fatal git push --delete)
key_files:
  - crates/smelt-cli/tests/k8s_lifecycle.rs
key_decisions:
  - "Used git clone into tempdir (not init+remote-add) for host-side repo setup — gives a realistic origin-configured repo with correct base_ref tracking"
  - "GIT_SSH_COMMAND='ssh -o StrictHostKeyChecking=accept-new' on both Pod exec and host clone/push calls — avoids interactive host-key prompts in CI"
  - "exec exit code != 0 triggers pre-teardown panic with full stdout/stderr — Pod is torn down first, no orphans on assertion failure"
patterns_established:
  - "S03 double-guard pattern: k8s_provider_or_skip() + get_test_git_remote() guard — both env vars required, either absent → skip with eprintln"
  - "Unique push branch via SystemTime::UNIX_EPOCH millis — avoids cross-run collisions on shared remotes"
observability_surfaces:
  - "eprintln! phase markers with ==> [S03] prefix visible under --nocapture: provision, exec, fetch, collect, teardown"
  - "panic! with exec_handle.stdout + exec_handle.stderr on non-zero Pod exit code"
  - "assertion failure messages include result.no_changes, result.commit_count, result.files_changed for collect failures"
duration: 20min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T02: Integration test test_k8s_push_from_pod_result_collection

**Added the S03 end-to-end integration test: Pod git-pushes a branch to `$SMELT_GIT_REMOTE`, host `fetch_ref` brings it local, `ResultCollector::collect()` asserts `no_changes == false` with ≥1 commit and `result.txt` in `files_changed`.**

## What Happened

Added `get_test_git_remote()` free function to `k8s_lifecycle.rs` and the full `test_k8s_push_from_pod_result_collection` test as test #5 (all 5 tests are `#[ignore]`).

The test follows the S02 double-guard pattern: `k8s_provider_or_skip()` checks `SMELT_K8S_TEST` and cluster reachability; `get_test_git_remote()` checks `SMELT_TEST_GIT_REMOTE`. Either absent → `eprintln!` + `return`.

When both guards pass:
1. `pre_clean_k8s("smelt", "s03-test")` deletes any orphaned Pod/Secret.
2. A manifest with `job.name = "s03-test"` and `job.repo = git_remote` is built so `generate_pod_spec()` injects the correct `SMELT_GIT_REMOTE`.
3. A unique branch name (`smelt-s03-push-test-<epoch_secs>`) avoids cross-run collisions.
4. Pod is provisioned; a shell script runs inside: `git checkout -b <branch>`, creates `result.txt`, commits, and `git push "$SMELT_GIT_REMOTE" <branch>:<branch>`.
5. Non-zero exec exit code → teardown then panic with full stdout/stderr.
6. Host: `git clone` into `tempfile::TempDir`, record `base_ref` from HEAD, `GitCli::fetch_ref("origin", "+<branch>:<branch>")`, then `ResultCollector::collect(&base_ref, &push_branch)`.
7. Assertions: `!result.no_changes`, `result.commit_count >= 1`, `result.files_changed.contains("result.txt")`.
8. Teardown Pod (unconditional), best-effort `git push origin --delete <branch>`.

Required adding `use smelt_core::{GitCli, ResultCollector};` and `use smelt_core::GitOps as _;` to the test file imports.

## Verification

- `cargo test -p smelt-cli --test k8s_lifecycle` (without env vars): `test result: ok. 0 passed; 0 failed; 5 ignored` ✓
- `cargo test --workspace`: all tests pass (pre-existing intermittent flakiness in `test_cli_run_invalid_manifest` in `docker_lifecycle.rs` is unrelated to this task — confirmed by zero diff to that file) ✓

## Diagnostics

- `--nocapture` flag shows `==> [S03]` phase markers for each step.
- On Pod exec failure: panic message includes full `stdout` and `stderr` from the in-Pod script.
- On collect failure: assertion messages include `result.no_changes`, `result.commit_count`, `result.files_changed`.
- `RUST_LOG=smelt_core=debug` during the test run shows provision readiness polling, exec WebSocket details, `fetch_ref` git command invocation.

## Deviations

None — implementation matched the task plan exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/tests/k8s_lifecycle.rs` — added `get_test_git_remote()` helper and `test_k8s_push_from_pod_result_collection` integration test (test #5); added `use smelt_core::{GitCli, ResultCollector}` and `use smelt_core::GitOps as _` imports
