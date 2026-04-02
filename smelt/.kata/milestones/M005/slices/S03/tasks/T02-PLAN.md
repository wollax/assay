---
estimated_steps: 6
estimated_files: 1
---

# T02: Integration test test_k8s_push_from_pod_result_collection

**Slice:** S03 — Push-from-Pod Result Collection
**Milestone:** M005

## Description

Adds `test_k8s_push_from_pod_result_collection` to `crates/smelt-cli/tests/k8s_lifecycle.rs` — the integration test that retires the push-from-Pod risk. The test proves the complete S03 collection path:

1. Provision a K8s Pod (via the real `KubernetesProvider`)
2. Exec a shell script inside the Pod that: creates a git commit on a new branch and pushes it to `$SMELT_GIT_REMOTE` (the injected env var from T01)
3. On the host: call `git.fetch_ref("origin", "+<branch>:<branch>")` to bring the pushed branch local
4. Call `ResultCollector::collect()` and assert `no_changes == false`
5. Teardown the Pod

The test is gated on two env vars: `SMELT_K8S_TEST=1` (kind cluster) and `SMELT_TEST_GIT_REMOTE=<ssh-url>` (reachable SSH remote). Without both, the test returns early with an eprintln. This mirrors the S02 pattern exactly.

**Note on host-side repo setup:** The test needs a local git repo to `fetch_ref` into. The simplest approach is to use the temp dir approach: `git clone <SMELT_TEST_GIT_REMOTE> <tempdir>` on the host before the test to get a proper origin-configured repo. This avoids init+remote-add complexity and gives a realistic setup. If the remote already has commits, the `base_ref` for ResultCollector is the HEAD of the clone before the Pod pushes.

## Steps

1. Add `fn get_test_git_remote() -> Option<String>` free function to `k8s_lifecycle.rs` — `std::env::var("SMELT_TEST_GIT_REMOTE").ok()`. Returns `None` when unset.

2. Add `#[tokio::test] #[ignore] async fn test_k8s_push_from_pod_result_collection()`. At the top: call the existing `k8s_provider_or_skip()` guard (exits if `SMELT_K8S_TEST` not set or client fails). Then: `let Some(git_remote) = get_test_git_remote() else { eprintln!("SMELT_TEST_GIT_REMOTE not set — skipping push-from-pod test"); return; };`

3. Build a manifest with `repo = git_remote.clone()` — either use `k8s_manifest()` or create a local manifest TOML string with the test repo URL substituted in. Call `pre_clean_k8s("smelt", "s03-test")` for idempotency. Provision the Pod: `let container = provider.provision(&manifest).await.expect("provision should succeed");`

4. Exec the push script in the Pod. Use a unique branch name to avoid conflicts: `let push_branch = format!("smelt-s03-push-test-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());`. Build the shell script:
   ```
   cd /workspace &&
   git config user.email test@smelt.local &&
   git config user.name smelt &&
   git checkout -b <push_branch> &&
   echo result > result.txt &&
   git add result.txt &&
   git commit -m 'push-from-pod test' &&
   GIT_SSH_COMMAND='ssh -o StrictHostKeyChecking=accept-new' git push $SMELT_GIT_REMOTE <push_branch>:<push_branch>
   ```
   Call `provider.exec(&container, &["/bin/sh".into(), "-c".into(), script.into()]).await`. Assert `handle.exit_code == 0` — if not, teardown and panic with the stderr.

5. On the host: clone the remote into a tempdir (`std::process::Command::new("git").args(["clone", &git_remote, tmpdir.path().to_str().unwrap()])`) to get an origin-configured repo. Record `base_ref` as the HEAD of this clone before the fetch. Create `GitCli::new(git_binary, tmpdir.path().to_path_buf())`. Call `git.fetch_ref("origin", &format!("+{push_branch}:{push_branch}")).await.expect("fetch_ref should succeed")`. Create `ResultCollector::new(git, tmpdir.path().to_path_buf())`. Call `collector.collect(&base_ref, &push_branch).await.expect("collect should succeed")`. Assert `!result.no_changes`, `result.commit_count >= 1`, `result.files_changed.contains(&"result.txt".to_string())`.

6. Teardown: `provider.teardown(&container).await.expect("teardown should succeed")`. The temp dir is cleaned up automatically by `TempDir` drop. Optionally, clean up the remote push branch via `git push origin --delete <push_branch>` (best-effort, non-fatal).

## Must-Haves

- [ ] `get_test_git_remote()` helper returns `None` when env var absent, `Some(url)` when set
- [ ] Test skips gracefully (eprintln + return) when `SMELT_K8S_TEST` not set OR `SMELT_TEST_GIT_REMOTE` not set
- [ ] Test provisions a Pod successfully when both env vars are set and kind cluster is running
- [ ] Push script commits and pushes to `$SMELT_GIT_REMOTE` inside the Pod; exec exit code is 0
- [ ] Host `fetch_ref` creates the local branch; `ResultCollector::collect()` returns `no_changes == false`
- [ ] Pod is torn down after the test (idempotent even if assertions fail)
- [ ] `cargo test -p smelt-cli --test k8s_lifecycle` without env vars: 5 tests ignored, 0 failures

## Verification

- `cargo test -p smelt-cli --test k8s_lifecycle` (without env vars): `test result: ok. 0 passed; 0 failed; 5 ignored` — graceful skip, no panic
- `cargo test --workspace`: 0 failures — existing suite unaffected
- With cluster: `SMELT_K8S_TEST=1 SMELT_TEST_GIT_REMOTE=<url> SMELT_TEST_SSH_KEY=<key> cargo test -p smelt-cli --test k8s_lifecycle -- test_k8s_push_from_pod_result_collection --include-ignored --nocapture` — passes; output shows "Collected: 1 commits on branch 'smelt-s03-push-test-...'"

## Observability Impact

- Signals added/changed: `eprintln!` messages in the test for each phase (provision, exec, fetch, collect, teardown) — visible with `--nocapture`; test failure on non-zero exec exit code shows full stderr from the in-Pod script
- How a future agent inspects this: `--nocapture` output shows which phase failed; `RUST_LOG=smelt_core=debug` during the test run shows provision readiness polling, exec WebSocket details, teardown confirmation
- Failure state exposed: `panic!` with `handle.stderr` when exec exit code != 0; assertion failure message includes `result.no_changes` and `result.commit_count`; teardown called unconditionally so no Pod orphans

## Inputs

- `crates/smelt-cli/tests/k8s_lifecycle.rs` — existing `k8s_provider_or_skip()`, `pre_clean_k8s()`, `k8s_manifest()` helpers; existing 4 test pattern as reference
- T01 output: `GitOps::fetch_ref()` trait method + `GitCli::fetch_ref()` implementation (required for Step 5)
- T01 output: `generate_pod_spec()` injects `SMELT_GIT_REMOTE` (required for the push script to use `$SMELT_GIT_REMOTE`)
- S02 decision (D094): `pre_clean_k8s` deletes orphaned Pod/Secret before provisioning; follow the same pattern for job name `"s03-test"`
- Research constraint: Pod can't reach host filesystem — use a network-accessible SSH remote, not a local path; `SMELT_TEST_GIT_REMOTE` provides this

## Expected Output

- `crates/smelt-cli/tests/k8s_lifecycle.rs` — `get_test_git_remote()` helper + `test_k8s_push_from_pod_result_collection` test (5 total tests in file, all `#[ignore]`)
- Without cluster env vars: `cargo test -p smelt-cli --test k8s_lifecycle` shows 5 ignored
- With cluster env vars: test passes end-to-end, proving push-from-Pod → fetch → ResultCollector path
