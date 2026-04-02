# S03: Push-from-Pod Result Collection — UAT

**Milestone:** M005
**Written:** 2026-03-23

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All S03 claims are proven by automated tests. The unit test (`test_fetch_ref_creates_local_branch`) proves `GitOps::fetch_ref()` correctness without a cluster. The integration test (`test_k8s_push_from_pod_result_collection`) proves the full push-from-Pod path end-to-end when `SMELT_K8S_TEST=1 SMELT_TEST_GIT_REMOTE=<url>` are set. No human judgment is required to verify the collection mechanics; live Assay execution inside a Pod is deferred to S04-UAT.

## Preconditions

- `cargo build --workspace` succeeds
- For unit tests only: none beyond Rust toolchain
- For integration test: kind cluster running, `smelt` namespace exists, `SMELT_K8S_TEST=1`, `SMELT_TEST_GIT_REMOTE=<ssh-url>`, `SMELT_TEST_SSH_KEY=<path-to-key>` set

## Smoke Test

```
cargo test -p smelt-core -- k8s fetch_ref --nocapture
```

Expected: 4 tests pass — `test_generate_pod_spec_snapshot`, `test_generate_pod_spec_requires_kubernetes_config`, `test_generate_pod_spec_resource_limits`, `test_fetch_ref_creates_local_branch`.

## Test Cases

### 1. SMELT_GIT_REMOTE appears in generated Pod spec

```
cargo test -p smelt-core -- test_generate_pod_spec_snapshot --nocapture
```

1. Run the command above
2. **Expected:** Test passes. JSON snapshot contains both `"SMELT_GIT_REMOTE"` and `"git@github.com:example/repo.git"`.

### 2. fetch_ref creates local branch from bare remote

```
cargo test -p smelt-core -- test_fetch_ref_creates_local_branch --nocapture
```

1. Run the command above
2. **Expected:** Test passes. Bare repo is created, commit pushed, `fetch_ref("origin", "+main:fetched-main")` called, `branch_exists("fetched-main")` returns true.

### 3. k8s_lifecycle tests skip gracefully without cluster

```
cargo test -p smelt-cli --test k8s_lifecycle
```

1. Run without setting `SMELT_K8S_TEST`
2. **Expected:** `test result: ok. 0 passed; 0 failed; 5 ignored` — all 5 tests (including `test_k8s_push_from_pod_result_collection`) are ignored, not failed.

### 4. Full push-from-Pod integration test (cluster required)

```
SMELT_K8S_TEST=1 SMELT_TEST_GIT_REMOTE=<ssh-url> SMELT_TEST_SSH_KEY=<key-path> \
  cargo test -p smelt-cli --test k8s_lifecycle \
  -- test_k8s_push_from_pod_result_collection --include-ignored --nocapture
```

1. Ensure kind cluster is running with `smelt` namespace
2. Set env vars as shown
3. Run the command
4. **Expected:** Test passes. Output shows `==> [S03]` phase markers for provision, exec, fetch, collect, teardown. Final assertions: `result.no_changes == false`, `result.commit_count >= 1`, `result.files_changed` contains `"result.txt"`.

## Edge Cases

### Missing SMELT_TEST_GIT_REMOTE with cluster present

```
SMELT_K8S_TEST=1 cargo test -p smelt-cli --test k8s_lifecycle \
  -- test_k8s_push_from_pod_result_collection --include-ignored --nocapture
```

1. Set `SMELT_K8S_TEST=1` but do NOT set `SMELT_TEST_GIT_REMOTE`
2. **Expected:** Test prints `SMELT_TEST_GIT_REMOTE not set — skipping` and returns without failure.

### Phase 8 fetch path visible in log output

```
RUST_LOG=smelt_cli=info smelt run manifest-k8s.toml
```

1. Run a kubernetes manifest (S04 required for full dispatch)
2. **Expected:** Log line `fetching result branch from remote` appears in stderr at Phase 8.

## Failure Signals

- `test_generate_pod_spec_snapshot` fails → `SMELT_GIT_REMOTE` env var was not injected into `generate_pod_spec()`
- `test_fetch_ref_creates_local_branch` fails → `GitCli::fetch_ref()` implementation or force-refspec is broken
- Integration test non-zero Pod exec exit code → `git push` inside Pod failed; check `stdout`/`stderr` in panic message; likely SSH key or remote URL issue
- Integration test `result.no_changes == true` → `fetch_ref` succeeded but `ResultCollector::collect()` did not see the pushed commit; check that `base_ref` predates the push and that the local branch was actually created by `fetch_ref`
- 5 tests appear as FAILED (not ignored) without cluster → `#[ignore]` annotation missing from test; S03 changes should not cause this

## Requirements Proved By This UAT

- R021 (partial) — S03 UAT proves the push-from-Pod collection sub-path: agent container knows where to push (`SMELT_GIT_REMOTE`), host can fetch the pushed branch (`fetch_ref`), and `ResultCollector` detects the new commits. Full R021 validation requires S04 (CLI dispatch) to complete.

## Not Proven By This UAT

- Full `smelt run` end-to-end with `runtime = "kubernetes"` dispatching through `AnyProvider::Kubernetes` — deferred to S04 (CLI dispatch not yet wired)
- Real Assay session executing inside a Pod and pushing its result branch — deferred to S04-UAT
- `--dry-run` showing `── Kubernetes ──` section — deferred to S04
- SSH credential lifecycle (Secret created, pod clones, Secret deleted) — proven in S02-UAT

## Notes for Tester

- The push-from-Pod integration test requires a network-accessible SSH git remote (e.g. a GitHub repo or a self-hosted Gitea instance reachable from inside the kind cluster). A host-local bare repo at `/tmp/foo.git` will NOT work — kind Pods run in a separate network namespace.
- The test creates a uniquely-named branch (`smelt-s03-push-test-<epoch_secs>`) to avoid cross-run collisions. After the test the remote branch is cleaned up with a best-effort `git push origin --delete <branch>`.
- If the test is interrupted mid-run, a Pod named `smelt-s03-test` may be left running in the `smelt` namespace. Clean up with `kubectl delete pod smelt-s03-test -n smelt` before re-running.
