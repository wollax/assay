# S03: Push-from-Pod Result Collection — Research

**Date:** 2026-03-23
**Domain:** Kubernetes env injection + git fetch integration + run.rs Phase 8 branching
**Confidence:** HIGH

## Summary

S03 has two implementation points and one integration test. Both implementation points are small and mechanical. The main risk is the integration test, which needs a git remote reachable from inside a kind Pod.

**Implementation point 1 — SMELT_GIT_REMOTE injection:** `generate_pod_spec()` in `k8s.rs` must add `env: [EnvVar { name: "SMELT_GIT_REMOTE", value: manifest.job.repo }]` to the main `smelt-agent` container. `manifest.job.repo` is already an SSH remote URL (e.g., `git@github.com:owner/repo.git`) — it's the same value the init container uses for git clone. `k8s_openapi::api::core::v1::EnvVar` is already available via the existing `k8s_openapi` dep.

**Implementation point 2 — Phase 8 git fetch in run.rs:** Before calling `ResultCollector::collect()`, detect `manifest.environment.runtime == "kubernetes"` and call `git fetch origin <manifest.merge.target>` on the host. This populates the local ref that ResultCollector reads. `ResultCollector` is unchanged — it reads `manifest.merge.target` which `git fetch` has now made available locally.

**Integration test risk:** The K8s Pod needs to reach a pushable git remote. Inside a kind cluster (Pod-in-Docker), the host filesystem is not directly accessible. The test needs an SSH-accessible remote URL that works from inside a Pod. This mirrors the `SMELT_TEST_SSH_KEY` pattern — add `SMELT_TEST_GIT_REMOTE` as a second required test env var (skip if absent).

## Recommendation

**For SMELT_GIT_REMOTE injection:** Modify `generate_pod_spec()` to add the env var directly. The signature stays `generate_pod_spec(manifest, job_name, ssh_private_key)` — the value is sourced from `manifest.job.repo` which is already available.

**For Phase 8 git fetch:** Add a `fetch_ref()` method to `GitOps` + `GitCli` (consistent with the existing git abstraction; testable via the same `GitOps` mock pattern as `ResultCollector`). Call it from `run.rs` Phase 8 only on the kubernetes path.

**For the integration test:** Add `test_k8s_push_from_pod_result_collection` to `k8s_lifecycle.rs` gated on both `SMELT_K8S_TEST` and `SMELT_TEST_GIT_REMOTE`. The test execs a shell script in the Pod that creates a commit and pushes to `$SMELT_GIT_REMOTE`, then verifies the ref appears locally via `git fetch`.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Setting env vars on K8s containers | `k8s_openapi::api::core::v1::EnvVar` (already a dep) | Typed struct, no YAML hand-crafting; same dep as the rest of `generate_pod_spec()` |
| Running git commands on host | `GitCli` + `GitOps` trait (already in `smelt-core`) | Consistent abstraction; testable; shell-out to `git` is the established pattern (D015, D071) |
| Detecting kubernetes runtime in run.rs | `manifest.environment.runtime.as_str() == "kubernetes"` | Direct string comparison mirrors the Phase 3 dispatch pattern; no new flag needed |

## Existing Code and Patterns

- `crates/smelt-core/src/k8s.rs` — `generate_pod_spec()` (line ~140): the `main_container` Container struct has `..Default::default()` for all unset fields. Add `env: Some(vec![EnvVar { name: "SMELT_GIT_REMOTE".into(), value: Some(manifest.job.repo.clone()), ..Default::default() }])` before the `..Default::default()`. Import `k8s_openapi::api::core::v1::EnvVar` alongside the existing imports.
- `crates/smelt-core/src/git/mod.rs` — `GitOps` trait (line ~93+): add `fn fetch_ref(&self, remote: &str, refspec: &str) -> impl Future<Output = Result<()>> + Send;` following the existing method pattern.
- `crates/smelt-core/src/git/cli.rs` — `GitCli` impl: add `async fn fetch_ref(&self, remote: &str, refspec: &str) -> Result<()>` calling `self.run(&["fetch", remote, refspec]).await.map(|_| ())`.
- `crates/smelt-cli/src/commands/run.rs` — Phase 8 collect block (around line 303): insert K8s fetch before `collector.collect()`. Pattern: match on runtime string, same as Phase 3+4 dispatch at line ~200.
- `crates/smelt-cli/tests/k8s_lifecycle.rs` — `k8s_provider_or_skip()` + `pre_clean_k8s()` helpers. The new test follows the same 4-step structure: skip, pre_clean, provision, exec+assert.
- `crates/smelt-core/src/collector.rs` — `ResultCollector::collect(base_ref, target_branch)`: reads HEAD and target branch locally; unchanged for S03. It calls `rev_parse(target_branch)` only after `branch_create` — which is fine because `git fetch origin <branch>` creates a remote-tracking ref `origin/<branch>`, not a local branch. **Important:** the `git fetch` populates `origin/<branch>`, but `ResultCollector` calls `branch_create(target_branch, "HEAD")` — it creates the target branch from HEAD, not from the remote ref. The actual "collecting" in the K8s path is different from Docker.

## Constraints

- **ResultCollector reads local HEAD, not the remote ref.** `ResultCollector::collect()` calls `rev_parse("HEAD")` and compares to `rev_parse(base_ref)`. In the K8s path, HEAD on the host is the original commit (before Assay ran) — it hasn't moved. Assay's commits are on the remote, not locally. This means a plain `git fetch origin <branch>` is NOT enough — the host HEAD won't have the new commits.
- **The real S03 collection path is different from what Phase 8 does for Docker.** For Docker, Assay runs with the bind-mounted repo, so it commits directly to the local filesystem — HEAD moves. For K8s, Assay commits and pushes from inside the Pod, but the host repo HEAD stays at the original commit. `ResultCollector` comparing `HEAD != base_ref` will return `no_changes = true` unless the host repo is updated.
- **How to reconcile:** After `git fetch origin <target_branch>`, the host needs to advance HEAD. The cleanest approach: after fetching, do `git merge --ff-only origin/<target_branch>` (or `git reset --hard origin/<target_branch>`) to bring the host repo up to date before calling `ResultCollector`. Or: skip `ResultCollector` for K8s and build a direct result from the fetched ref. The boundary map says "ResultCollector is unchanged — it reads a local ref that git fetch has populated" which implies the fetch makes the local ref available. The key insight is that `git fetch origin <branch>` creates `origin/<branch>` but NOT a local `<branch>`. The correct fetch command is `git fetch origin <branch>:<branch>` which creates the local branch directly. Then `ResultCollector` can proceed as normal — it reads local refs.
- **`git fetch origin <branch>:<branch>` pattern:** This fetches the remote branch and creates/updates the local branch of the same name. After this, `manifest.merge.target` exists locally pointing at the Assay result commits, and `ResultCollector::collect()` can read it normally via `branch_exists(target_branch)` → it already exists, so it updates it and returns the commit info.
- **SMELT_GIT_REMOTE value:** `manifest.job.repo` is the SSH clone URL. This works as the push remote if the SSH key has both read and write access. No format transformation needed.
- **`k8s_openapi::api::core::v1::EnvVar`** — not yet imported in `k8s.rs`. Must add to the `use` block alongside existing imports.
- **D085 / D086 firms:** init container git-clones from `manifest.job.repo`; agent pushes to `SMELT_GIT_REMOTE`; `run.rs` fetches from remote. These decisions are locked — S03 follows them exactly.

## Common Pitfalls

- **`git fetch origin <branch>` creates `origin/<branch>`, NOT `<branch>`** — `ResultCollector` calls `branch_exists(target_branch)` on the local branch name, not the remote-tracking ref. Use `git fetch origin <branch>:<branch>` (refspec with local target) to create the local branch directly. This is the correct fetch invocation.
- **`git fetch` requires the remote to be configured** — the host repo must have `origin` pointing at the correct remote. For `manifest.job.repo` to be `origin`, the host repo must have been cloned from that URL. This is the normal case for `smelt run` (user runs it from their repo). Adding a check or clear error if `origin` is not set is defensive.
- **Host repo HEAD doesn't move on K8s path** — if you call `ResultCollector::collect(base_ref, target_branch)` without first doing `git fetch origin <branch>:<branch>`, HEAD == base_ref → `no_changes = true` → no PR created. The fetch refspec MUST create the local branch, not just the remote-tracking ref.
- **Integration test: Pod can't reach host filesystem** — kind runs Pods inside Docker containers. A local bare repo at `/tmp/foo.git` is NOT accessible from inside a kind Pod. The integration test must use a network-accessible SSH remote, not a local path. Use `SMELT_TEST_GIT_REMOTE` env var, skip if not set.
- **`generate_pod_spec()` snapshot tests will break** — adding `SMELT_GIT_REMOTE` env var to the main container will appear in the JSON snapshot. The existing snapshot test uses `assert!(json.contains("initContainers"))` style substring checks, so they won't break as-is. But the test should be updated to also assert `SMELT_GIT_REMOTE` is present in the JSON, proving the injection works.
- **`SMELT_GIT_REMOTE` in tests** — the existing `generate_pod_spec()` tests use `KUBERNETES_MANIFEST_TOML` with `repo = "git@github.com:example/repo.git"`. After S03, the snapshot JSON should contain `"SMELT_GIT_REMOTE"` and `"git@github.com:example/repo.git"`. Add assertion.

## Open Risks

- **Integration test reachability from kind:** The test for `test_k8s_push_from_pod_result_collection` needs a real pushable SSH remote reachable from inside a kind Pod. If `SMELT_TEST_GIT_REMOTE` is not set or not reachable, the test skips — this means the test may never run in common CI setups. This is acceptable for M005 (same caveat as the S02 live tests).
- **`git fetch origin <branch>:<branch>` force-flag:** If the local branch already exists (from a prior run), `git fetch origin <branch>:<branch>` will fail unless `+` prefix is used (`+<branch>:<branch>`). Should use `git fetch origin +<branch>:<branch>` for idempotency. Or delete local branch first — but adding `+` in the refspec is simpler.
- **SSH known_hosts inside Pod:** The init container clones via SSH. If the SSH remote's host key is not in `/root/.ssh/known_hosts`, git clone will fail interactively. The init container command should include `GIT_SSH_COMMAND="ssh -o StrictHostKeyChecking=accept-new"` to auto-accept on first connect. This is a S02 carry-forward concern that S03's integration test will expose.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Kubernetes | None checked | No dedicated Kata skill; using k8s_openapi + kube crate directly |
| Rust async | None needed | Standard tokio patterns; existing codebase conventions sufficient |

## Sources

- `crates/smelt-core/src/k8s.rs` — current `generate_pod_spec()` implementation; main container struct at line ~155; `k8s_openapi` imports at top
- `crates/smelt-cli/src/commands/run.rs` — Phase 8 collect block (~line 300); Phase 3+4 provider dispatch (~line 200); `AnyProvider` enum as dispatch model for adding Kubernetes
- `crates/smelt-core/src/collector.rs` — `ResultCollector::collect()` reads `rev_parse("HEAD")` vs `rev_parse(base_ref)`; branch_create uses "HEAD" as source; must have local HEAD advanced past base_ref
- `crates/smelt-core/src/git/mod.rs` + `cli.rs` — `GitOps` trait; `run()` delegates to `run_in()` (D071); `fetch_ref` to be added here
- `crates/smelt-cli/tests/k8s_lifecycle.rs` — test harness pattern (`k8s_provider_or_skip`, `pre_clean_k8s`, `#[ignore]`)
- D086 — push-from-Pod + host-side git fetch architecture decision (confirmed approach)
- D085 — init container git clone (SSH key has both read+write access required)
- D032 — host-side collection reads host repo directly (unchanged for S03)
