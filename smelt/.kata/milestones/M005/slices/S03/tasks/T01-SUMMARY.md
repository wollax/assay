---
id: T01
parent: S03
milestone: M005
provides:
  - GitOps::fetch_ref() trait method with RPITIT signature
  - GitCli::fetch_ref() implementation shelling out to `git fetch remote refspec`
  - test_fetch_ref_creates_local_branch unit test (bare repo → push → fetch → verify branch exists)
  - SMELT_GIT_REMOTE env var injected into the agent container via generate_pod_spec()
  - EnvVar import added to k8s.rs
  - Snapshot test assertions for "SMELT_GIT_REMOTE" and "git@github.com:example/repo.git"
  - Phase 8 kubernetes fetch block in run.rs calling fetch_ref("origin", "+<target>:<target>")
key_files:
  - crates/smelt-core/src/git/mod.rs
  - crates/smelt-core/src/git/cli.rs
  - crates/smelt-core/src/k8s.rs
  - crates/smelt-cli/src/commands/run.rs
key_decisions:
  - Used `use smelt_core::GitOps as _` as local import in run.rs Phase 8 block (trait already re-exported from crate root); avoids a new top-level use statement
  - force-refspec `+<branch>:<branch>` documented in fetch_ref trait doc comment — required for re-runs when local branch already exists
patterns_established:
  - fetch_ref pattern: self.run(&["fetch", remote, refspec]).await.map(|_| ()) — consistent with existing GitCli methods
  - Phase 8 kubernetes dispatch: string comparison `manifest.environment.runtime == "kubernetes"` (D086/D093)
observability_surfaces:
  - tracing::info!(branch = %manifest.merge.target, "fetching result branch from remote") in Phase 8 kubernetes path; visible at RUST_LOG=smelt_cli=info
  - Failure: anyhow::Error with context "Phase 8: failed to fetch result branch from remote" propagates to CLI stderr; git fetch stderr captured in SmeltError::GitExecution.message
duration: 15min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T01: SMELT_GIT_REMOTE injection + fetch_ref() + Phase 8 kubernetes fetch

**Three mechanical production changes that close the push-from-Pod collection path: `SMELT_GIT_REMOTE` env injection into the agent container, `GitOps::fetch_ref()` + `GitCli` implementation with force-refspec support, and Phase 8 kubernetes fetch in `run.rs`.**

## What Happened

All three changes were implemented as specified in the task plan:

1. **`fetch_ref` trait method** added to `GitOps` in `git/mod.rs` after `rev_parse()`, using the same RPITIT pattern (D019) as all other trait methods. Doc comment explains the `+` force-refspec requirement for re-runs.

2. **`GitCli::fetch_ref` implementation** added in `cli.rs` — single line delegating to `self.run(&["fetch", remote, refspec]).await.map(|_| ())`. Unit test `test_fetch_ref_creates_local_branch` sets up a bare repo, clones it, adds a commit + push, creates a third working clone, calls `fetch_ref("origin", "+<default_branch>:fetched-main")`, and asserts `branch_exists("fetched-main") == true`.

3. **`EnvVar` import** added to the existing `k8s_openapi::api::core::v1` import block in `k8s.rs`.

4. **`env` field** added to `main_container` struct literal in `generate_pod_spec()` with `SMELT_GIT_REMOTE = manifest.job.repo`. Field placed before `..Default::default()` as specified.

5. **Snapshot test updated** with two new assertions: `json.contains("\"SMELT_GIT_REMOTE\"")` and `json.contains("\"git@github.com:example/repo.git\"")`.

6. **Phase 8 kubernetes fetch block** inserted in `run.rs` between `GitCli::new()` and `ResultCollector::new()`. Uses `use smelt_core::GitOps as _` local import to bring the trait into scope.

## Verification

```
cargo build -p smelt-core       → 0 errors, 0 warnings
cargo build -p smelt-cli        → 0 errors, 0 warnings
cargo test -p smelt-core -- k8s --nocapture
  → test_generate_pod_spec_snapshot ... ok
  → test_generate_pod_spec_requires_kubernetes_config ... ok
  → test_generate_pod_spec_resource_limits ... ok
cargo test -p smelt-core -- fetch_ref --nocapture
  → test_fetch_ref_creates_local_branch ... ok
cargo test --workspace
  → 155 passed; 0 failed; 0 ignored
```

## Diagnostics

- `RUST_LOG=smelt_cli=info cargo run -- run manifest.toml` shows `fetching result branch from remote` log line when `runtime = "kubernetes"`.
- `git branch -v` after a kubernetes run shows the fetched local branch.
- On fetch failure: `anyhow::Error` context `"Phase 8: failed to fetch result branch from remote"` + `SmeltError::GitExecution { message }` containing git's stderr.

## Deviations

None. All changes match the task plan exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/git/mod.rs` — `fetch_ref` trait method added after `rev_parse()`
- `crates/smelt-core/src/git/cli.rs` — `GitCli::fetch_ref` impl + `test_fetch_ref_creates_local_branch` unit test
- `crates/smelt-core/src/k8s.rs` — `EnvVar` import + `env` field on main container + updated snapshot assertions
- `crates/smelt-cli/src/commands/run.rs` — Phase 8 kubernetes fetch block
