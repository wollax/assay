# S03: Push-from-Pod Result Collection

**Goal:** Assay running inside a K8s Pod pushes its result branch to the remote; `run.rs` Phase 8 detects `runtime == "kubernetes"`, runs `git fetch origin +<branch>:<branch>` on the host, and hands a populated local ref to `ResultCollector`; PR creation proceeds identically to the Docker path.
**Demo:** After executing T01 and T02, `cargo test --workspace` is green; the snapshot test confirms `SMELT_GIT_REMOTE` is injected into the agent container; `fetch_ref()` has a passing unit test; `test_k8s_push_from_pod_result_collection` passes against a real kind cluster with a live SSH remote.

## Must-Haves

- `generate_pod_spec()` sets `SMELT_GIT_REMOTE = manifest.job.repo` as an env var on the main `smelt-agent` container; snapshot test asserts the key is present in the JSON
- `GitOps` trait gains `fetch_ref(&self, remote, refspec)` with a `GitCli` implementation that shells out to `git fetch`; unit-tested with a temp bare repo
- `run.rs` Phase 8 detects `manifest.environment.runtime == "kubernetes"` and calls `git.fetch_ref("origin", &format!("+{t}:{t}", t = manifest.merge.target))` before `ResultCollector::collect()`, populating the local branch so ResultCollector finds it
- Integration test `test_k8s_push_from_pod_result_collection` in `k8s_lifecycle.rs` — skips unless both `SMELT_K8S_TEST=1` and `SMELT_TEST_GIT_REMOTE` are set; provisions a Pod, execs a script that commits and pushes to `$SMELT_GIT_REMOTE`, then runs `fetch_ref` + `ResultCollector::collect()` on the host and asserts `no_changes == false`
- `cargo test --workspace` all green; existing tests unaffected

## Proof Level

- This slice proves: **integration** (production code + unit + live-cluster integration test)
- Real runtime required: yes — `SMELT_K8S_TEST=1` + `SMELT_TEST_GIT_REMOTE` + kind cluster required for T02 full proof; T01 proves unit-level contract without a cluster
- Human/UAT required: no — automated tests cover all claims; live end-to-end Assay execution deferred to S04-UAT

## Verification

- `cargo test -p smelt-core -- k8s fetch_ref --nocapture` — all 10 existing kubernetes tests pass + updated snapshot asserts `SMELT_GIT_REMOTE`; new `fetch_ref` unit test passes
- `cargo test --workspace` — zero failures
- With cluster: `SMELT_K8S_TEST=1 SMELT_TEST_GIT_REMOTE=<url> SMELT_TEST_SSH_KEY=<key> cargo test -p smelt-cli --test k8s_lifecycle -- test_k8s_push_from_pod_result_collection --include-ignored --nocapture` — passes
- Without cluster: `cargo test -p smelt-cli --test k8s_lifecycle` — 5 tests ignored (not failed)

## Observability / Diagnostics

- Runtime signals: `tracing::info!` in Phase 8 on the kubernetes fetch path: `"fetching result branch from remote"` (branch, remote); `tracing::warn!` if fetch fails with context message
- Inspection surfaces: `RUST_LOG=smelt_core=debug,smelt_cli=debug cargo test` — shows Phase 8 fetch invocation; `git branch -v` on host after test confirms fetched branch exists locally
- Failure visibility: `SmeltError::GitExecution { operation, message }` surfaced as `anyhow::Error` from Phase 8, context: `"Phase 8: failed to fetch result branch from remote"`; test assertions on `no_changes == false` catch silent fetch failures
- Redaction constraints: none — git remote URLs and branch names are not secrets

## Integration Closure

- Upstream surfaces consumed: `generate_pod_spec()` (S01), `KubernetesProvider` (S02), `ResultCollector::collect()` (M001/S04), `GitOps`/`GitCli` (M001/S03), `run.rs` Phase 8 collect block
- New wiring introduced in this slice:
  - `SMELT_GIT_REMOTE` env var set at Pod build time → agent container knows push destination
  - `GitOps::fetch_ref()` + `GitCli` impl → reusable git fetch abstraction
  - `run.rs` Phase 8 kubernetes-branch: `fetch_ref("origin", "+<target>:<target>")` before `ResultCollector::collect()`
- What remains before the milestone is truly usable end-to-end: S04 — `AnyProvider::Kubernetes` dispatch in `run.rs` (currently kubernetes runtime hits the unsupported-runtime error at Phase 3), `--dry-run` `── Kubernetes ──` section, `examples/job-manifest-k8s.toml` end-to-end smoke test

## Tasks

- [x] **T01: SMELT_GIT_REMOTE injection + fetch_ref() + Phase 8 kubernetes fetch** `est:45m`
  - Why: Production code for both S03 implementation points — env var injection makes the agent know where to push; `fetch_ref` + Phase 8 wiring makes the host repo see the pushed branch before ResultCollector reads it
  - Files: `crates/smelt-core/src/k8s.rs`, `crates/smelt-core/src/git/mod.rs`, `crates/smelt-core/src/git/cli.rs`, `crates/smelt-cli/src/commands/run.rs`
  - Do:
    1. Add `fn fetch_ref(&self, remote: &str, refspec: &str) -> impl Future<Output = Result<()>> + Send;` to `GitOps` trait in `git/mod.rs` — follow the existing method signature pattern; place after `rev_parse`
    2. Implement `async fn fetch_ref(&self, remote: &str, refspec: &str) -> Result<()>` in `GitCli` (`git/cli.rs`) — body is `self.run(&["fetch", remote, refspec]).await.map(|_| ())`. Add a unit test `test_fetch_ref_creates_local_branch`: set up a bare remote with `git init --bare`, clone it, add a commit, push to bare, then call `fetch_ref("origin", "+main:fetched-main")` and assert `branch_exists("fetched-main")` returns true
    3. Add `use k8s_openapi::api::core::v1::EnvVar;` to the imports in `k8s.rs`. In `generate_pod_spec()`, add `env: Some(vec![EnvVar { name: "SMELT_GIT_REMOTE".into(), value: Some(manifest.job.repo.clone()), ..Default::default() }])` to `main_container` before the `..Default::default()` terminator
    4. Update `test_generate_pod_spec_snapshot` in `k8s.rs` to assert `json.contains("\"SMELT_GIT_REMOTE\"")` and `json.contains("\"git@github.com:example/repo.git\"")` — confirms both key and value appear in the Pod JSON
    5. In `run.rs` Phase 8 collect block (after constructing `git: GitCli`), insert the kubernetes fetch branch: `if manifest.environment.runtime == "kubernetes" { tracing::info!(branch = %manifest.merge.target, "fetching result branch from remote"); git.fetch_ref("origin", &format!("+{t}:{t}", t = manifest.merge.target)).await.with_context(|| "Phase 8: failed to fetch result branch from remote")?; }`
    6. Run `cargo test -p smelt-core` — all tests pass including updated snapshot; `cargo test --workspace` green
  - Verify: `cargo test -p smelt-core -- k8s fetch_ref --nocapture` passes; JSON snapshot contains `"SMELT_GIT_REMOTE"` and `"git@github.com:example/repo.git"`; `cargo test --workspace` shows 0 failures
  - Done when: `cargo test --workspace` all green; snapshot assertions for `SMELT_GIT_REMOTE` pass; `fetch_ref` unit test with bare remote passes

- [x] **T02: Integration test test_k8s_push_from_pod_result_collection** `est:45m`
  - Why: Retires the push-from-Pod risk by proving the full collection path end-to-end: Pod execs a push → host fetches → ResultCollector finds the branch
  - Files: `crates/smelt-cli/tests/k8s_lifecycle.rs`
  - Do:
    1. Add helper `fn get_test_git_remote() -> Option<String>` — reads `std::env::var("SMELT_TEST_GIT_REMOTE").ok()` — mirrors `k8s_provider_or_skip()` guard pattern
    2. Add `#[tokio::test] #[ignore] async fn test_k8s_push_from_pod_result_collection()`. At the top: call `k8s_provider_or_skip()` (skips if `SMELT_K8S_TEST` not set); then `let Some(git_remote) = get_test_git_remote() else { eprintln!("SMELT_TEST_GIT_REMOTE not set — skipping"); return; };`
    3. Build a manifest with `repo = git_remote.clone()` (reuse `k8s_manifest()` but override repo field or create a variant). Call `pre_clean_k8s(namespace, "s03-test")`. Provision the Pod via `provider.provision(&manifest).await`.
    4. Exec the push script in the Pod: `let branch = "smelt-s03-push-test"; let script = format!("cd /workspace && git config user.email test@smelt.local && git config user.name smelt && git checkout -b {branch} && echo result > result.txt && git add result.txt && git commit -m 'push-from-pod test' && GIT_SSH_COMMAND='ssh -o StrictHostKeyChecking=accept-new' git push $SMELT_GIT_REMOTE {branch}:{branch}");` exec via `provider.exec(&container, &["/bin/sh".into(), "-c".into(), script.clone()]).await`. Assert `exit_code == 0`.
    5. On the host: create a temp dir, `git init`, `git remote add origin <git_remote>`, create a `GitCli` pointing at this temp repo, call `git.fetch_ref("origin", &format!("+{branch}:{branch}"))`. Assert it succeeds. Create a `ResultCollector` with the temp repo path; call `collector.collect(base_ref, branch)` where `base_ref` is the SHA fetched before the push (or use `git rev-parse origin/HEAD` equivalent). Assert `!result.no_changes`. Teardown.
    6. Teardown the Pod with `provider.teardown(&container).await`. Assert success.
  - Verify: `SMELT_K8S_TEST=1 SMELT_TEST_GIT_REMOTE=<url> SMELT_TEST_SSH_KEY=<key> cargo test -p smelt-cli --test k8s_lifecycle -- test_k8s_push_from_pod_result_collection --include-ignored --nocapture` passes; without env vars, `cargo test -p smelt-cli --test k8s_lifecycle` shows the test ignored (not failed)
  - Done when: test skips gracefully when env vars absent; passes against real kind cluster + SSH remote when both env vars set; `cargo test --workspace` unaffected

## Files Likely Touched

- `crates/smelt-core/src/git/mod.rs` — `GitOps::fetch_ref` trait method
- `crates/smelt-core/src/git/cli.rs` — `GitCli::fetch_ref` impl + unit test
- `crates/smelt-core/src/k8s.rs` — `EnvVar` import, env var in main container, snapshot test assertion
- `crates/smelt-cli/src/commands/run.rs` — Phase 8 kubernetes fetch block
- `crates/smelt-cli/tests/k8s_lifecycle.rs` — `get_test_git_remote()`, `test_k8s_push_from_pod_result_collection`
