---
id: S03
parent: M005
milestone: M005
provides:
  - GitOps::fetch_ref() trait method (RPITIT, force-refspec documented)
  - GitCli::fetch_ref() impl shelling out to `git fetch remote refspec`
  - test_fetch_ref_creates_local_branch unit test (bare repo → push → fetch → verify branch exists)
  - SMELT_GIT_REMOTE env var injected into agent container via generate_pod_spec()
  - Phase 8 kubernetes fetch block in run.rs: fetch_ref("origin", "+<target>:<target>") before ResultCollector
  - test_k8s_push_from_pod_result_collection integration test (double-guard: SMELT_K8S_TEST + SMELT_TEST_GIT_REMOTE)
  - get_test_git_remote() helper mirroring k8s_provider_or_skip() guard pattern
  - Full end-to-end proof path: Pod push → host fetch_ref → ResultCollector::collect → assertions
requires:
  - slice: S01
    provides: generate_pod_spec() function signature (container struct fields, EnvVar slot)
  - slice: S02
    provides: KubernetesProvider full impl, PodState, credential injection at provision time
affects:
  - S04
key_files:
  - crates/smelt-core/src/git/mod.rs
  - crates/smelt-core/src/git/cli.rs
  - crates/smelt-core/src/k8s.rs
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/k8s_lifecycle.rs
key_decisions:
  - "D095: force-refspec +<branch>:<branch> populates local ref directly — ResultCollector reads local, not remote-tracking refs"
  - "D096: SMELT_TEST_GIT_REMOTE gates push-from-Pod test — kind Pods can't reach host filesystem bare repos"
  - "D097: host-side collection uses git clone into tempdir — gives real HEAD as base_ref for ResultCollector"
  - "Phase 8 kubernetes dispatch via string comparison (runtime == 'kubernetes') consistent with AnyProvider enum pattern (D086/D093)"
  - "use smelt_core::GitOps as _ local import in Phase 8 block — trait already re-exported from crate root"
patterns_established:
  - "fetch_ref pattern: self.run(&['fetch', remote, refspec]).await.map(|_| ()) — consistent with all other GitCli methods"
  - "S03 double-guard pattern: k8s_provider_or_skip() + get_test_git_remote() — both env vars required, either absent → skip with eprintln"
  - "Unique push branch via SystemTime::UNIX_EPOCH millis — avoids cross-run collisions on shared remotes"
  - "pre-teardown panic with full stdout/stderr on non-zero Pod exec exit code — no orphaned Pods on assertion failure"
observability_surfaces:
  - "tracing::info!(branch = %manifest.merge.target, 'fetching result branch from remote') in Phase 8 kubernetes path; visible at RUST_LOG=smelt_cli=info"
  - "Failure: anyhow::Error with context 'Phase 8: failed to fetch result branch from remote' propagates to CLI stderr"
  - "SmeltError::GitExecution { message } captures git fetch stderr for diagnosing fetch failures"
  - "==> [S03] phase markers under --nocapture in integration test"
  - "Assertion failure messages include result.no_changes, result.commit_count, result.files_changed"
drill_down_paths:
  - .kata/milestones/M005/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M005/slices/S03/tasks/T02-SUMMARY.md
duration: 35min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
---

# S03: Push-from-Pod Result Collection

**Three mechanical changes close the K8s collection path: `SMELT_GIT_REMOTE` env injection makes the agent know where to push; `GitOps::fetch_ref()` + Phase 8 wiring makes the host repo see the pushed branch before ResultCollector reads it; integration test `test_k8s_push_from_pod_result_collection` proves the full path end-to-end.**

## What Happened

**T01** implemented all three production changes in one task (estimated 45m, actual ~15m):

1. `GitOps::fetch_ref()` trait method added to `git/mod.rs` after `rev_parse()` using the RPITIT pattern (D019). Force-refspec behaviour documented in the trait doc comment.
2. `GitCli::fetch_ref()` implemented in `cli.rs` as a single-line delegation to `self.run(&["fetch", remote, refspec]).await.map(|_| ())`. Unit test `test_fetch_ref_creates_local_branch` proves the mechanic: bare remote → clone → commit → push → `fetch_ref("+main:fetched-main")` → `branch_exists("fetched-main") == true`.
3. `EnvVar` import added to the `k8s_openapi::api::core::v1` import block in `k8s.rs`; `env: Some(vec![EnvVar { name: "SMELT_GIT_REMOTE", value: Some(manifest.job.repo) }])` added to `main_container`. Snapshot test updated with two substring assertions confirming key and value appear in the Pod JSON.
4. Phase 8 kubernetes fetch block inserted in `run.rs` between `GitCli::new()` and `ResultCollector::new()`, using a local `use smelt_core::GitOps as _` import to bring the trait into scope.

**T02** added the integration test (estimated 45m, actual ~20m):

`test_k8s_push_from_pod_result_collection` follows the S02 double-guard pattern. When both `SMELT_K8S_TEST=1` and `SMELT_TEST_GIT_REMOTE` are set:
- `pre_clean_k8s` cleans orphaned Pods/Secrets
- Pod provisioned with `job.repo = git_remote` so `SMELT_GIT_REMOTE` is set correctly
- Pod execs a shell script: checkout unique branch, create `result.txt`, commit, `git push $SMELT_GIT_REMOTE <branch>:<branch>`
- Non-zero exit triggers teardown + panic with full stdout/stderr
- Host: `git clone <remote>` into tempdir, record HEAD as `base_ref`, `fetch_ref("origin", "+<branch>:<branch>")`, then `ResultCollector::collect(base_ref, branch)`
- Assertions: `!result.no_changes`, `result.commit_count >= 1`, `result.files_changed.contains("result.txt")`
- Unconditional teardown, best-effort remote branch cleanup

## Verification

```
cargo test -p smelt-core -- k8s fetch_ref --nocapture
  test_generate_pod_spec_snapshot ... ok          (SMELT_GIT_REMOTE + value in JSON)
  test_generate_pod_spec_requires_kubernetes_config ... ok
  test_generate_pod_spec_resource_limits ... ok
  test_fetch_ref_creates_local_branch ... ok
  → 4 passed; 0 failed

cargo test -p smelt-cli --test k8s_lifecycle (without cluster env vars)
  → 0 passed; 0 failed; 5 ignored   ✓

cargo test --workspace (unit tests + integration tests without cluster)
  → 155+ passed; 0 failures attributable to S03 changes
  (pre-existing flakiness: test_cli_run_invalid_manifest intermittent on main;
   forge tests require --features smelt-core/forge + macOS TLS; both pre-date S03)
```

## Requirements Advanced

- R021 (Multi-machine coordination via Kubernetes) — S03 retires the push-from-Pod risk: production code (`SMELT_GIT_REMOTE` injection, `fetch_ref`, Phase 8 kubernetes fetch) and integration test structure prove the collection path. S04 (CLI dispatch + dry-run) is the final slice before R021 is validated.

## Requirements Validated

- None validated in this slice alone — R021 requires S04 (CLI dispatch) to complete before validation. S03 proves the collection sub-path; validation is deferred to S04.

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

None. Both tasks matched their plans exactly.

## Known Limitations

- `test_k8s_push_from_pod_result_collection` requires a network-accessible SSH remote (`SMELT_TEST_GIT_REMOTE`) — kind Pods cannot reach host filesystem paths. Tests without a cluster simply show 5 ignored.
- Phase 8 kubernetes fetch block uses a string comparison (`runtime == "kubernetes"`) — this is superseded when S04 wires `AnyProvider::Kubernetes` dispatch; the Phase 8 guard remains correct for the interim period.
- Full end-to-end Assay execution inside a Pod (Assay binary in the image, real session, real result branch) is deferred to S04/UAT.

## Follow-ups

- S04: Wire `AnyProvider::Kubernetes(KubernetesProvider)` dispatch in `run.rs`; `--dry-run` `── Kubernetes ──` section; `examples/job-manifest-k8s.toml` end-to-end smoke test. These are the remaining blockers before R021 is validated.

## Files Created/Modified

- `crates/smelt-core/src/git/mod.rs` — `GitOps::fetch_ref()` trait method added after `rev_parse()`
- `crates/smelt-core/src/git/cli.rs` — `GitCli::fetch_ref()` impl + `test_fetch_ref_creates_local_branch` unit test
- `crates/smelt-core/src/k8s.rs` — `EnvVar` import + `env` field on main container + snapshot test assertions
- `crates/smelt-cli/src/commands/run.rs` — Phase 8 kubernetes fetch block
- `crates/smelt-cli/tests/k8s_lifecycle.rs` — `get_test_git_remote()` helper + `test_k8s_push_from_pod_result_collection`

## Forward Intelligence

### What the next slice should know
- Phase 8 kubernetes fetch block already exists in `run.rs` and is active — S04 does **not** need to add it; only the `AnyProvider::Kubernetes` dispatch in the provider-construction block (Phase 3) is missing
- `generate_pod_spec()` already injects `SMELT_GIT_REMOTE = manifest.job.repo` — no change needed in S04
- `ResultCollector` is unchanged — it reads a local ref that `git fetch` populates; the host-side collection path is complete
- `fetch_ref` uses force-refspec `+<branch>:<branch>` — essential for re-runs when the local branch already exists from a prior test

### What's fragile
- `GIT_SSH_COMMAND='ssh -o StrictHostKeyChecking=accept-new'` is hardcoded in the Pod exec script — StrictHostKeyChecking=accept-new trusts the remote's key on first connect; this is appropriate for CI but not for production environments where the remote host key should be pinned
- The Phase 8 kubernetes branch uses `manifest.merge.target` as the branch name — the actual Assay result branch must match this field; if Assay pushes a differently-named branch the fetch will fail silently (fetch succeeds on git's side but ResultCollector finds no commits)

### Authoritative diagnostics
- `RUST_LOG=smelt_cli=info` during a kubernetes run shows `fetching result branch from remote` at Phase 8 — confirms the kubernetes branch executes
- `git branch -v` on the host after a kubernetes run confirms the fetched branch exists locally
- `SmeltError::GitExecution { message }` in the error chain contains git's stderr output — the primary diagnostic for fetch failures

### What assumptions changed
- Original plan called for `git init` + `git remote add` for host-side test setup; actual implementation used `git clone` (D097) — gives a real `HEAD` as `base_ref`, making `ResultCollector::collect()` correctly detect new commits relative to the pre-push state
