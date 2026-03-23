---
estimated_steps: 6
estimated_files: 4
---

# T01: SMELT_GIT_REMOTE injection + fetch_ref() + Phase 8 kubernetes fetch

**Slice:** S03 — Push-from-Pod Result Collection
**Milestone:** M005

## Description

Three mechanical production code changes that together close the push-from-Pod collection path:

1. **`SMELT_GIT_REMOTE` env injection**: `generate_pod_spec()` adds `SMELT_GIT_REMOTE = manifest.job.repo` to the agent container env. This is the agent's only mechanism for knowing where to push — without it, the in-Pod Assay session has no remote URL.

2. **`GitOps::fetch_ref()` + `GitCli` impl**: A new trait method and implementation that shells out to `git fetch <remote> <refspec>`. The refspec `+<branch>:<branch>` (force-update, remote→local) is what creates a local branch directly from the remote ref — the critical fix documented in the research. Without the force-refspec `+`, re-runs fail if the local branch already exists.

3. **Phase 8 kubernetes fetch**: After Assay completes but before `ResultCollector::collect()`, detect `runtime == "kubernetes"` and call `fetch_ref("origin", "+<target>:<target>")`. This populates the local branch that ResultCollector reads. Without this, ResultCollector sees `HEAD == base_ref` → `no_changes = true` → no PR created.

All three changes are fully unit-testable without a cluster. The integration proof is T02.

## Steps

1. Add `fn fetch_ref(&self, remote: &str, refspec: &str) -> impl Future<Output = Result<()>> + Send;` to the `GitOps` trait in `crates/smelt-core/src/git/mod.rs` — place it after `rev_parse()`, follow the existing method pattern (no `#[doc]` required but a brief doc comment helps). The method must be object-safe-compatible with RPITIT (D019 pattern).

2. Implement `async fn fetch_ref(&self, remote: &str, refspec: &str) -> Result<()>` in `GitCli` (`crates/smelt-core/src/git/cli.rs`) with body `self.run(&["fetch", remote, refspec]).await.map(|_| ())`. Add a unit test `test_fetch_ref_creates_local_branch` in the `#[cfg(test)]` block: init a bare repo (`git init --bare /tmp/...`), clone it to a second temp dir, add a commit + push to the bare, then create a third working clone, call `git.fetch_ref("origin", "+main:fetched-main")`, and assert `git.branch_exists("fetched-main").await == true`.

3. Add `use k8s_openapi::api::core::v1::EnvVar;` to the imports in `crates/smelt-core/src/k8s.rs` (alongside the existing `Container`, `Volume`, etc. imports).

4. In `generate_pod_spec()` in `k8s.rs`, add `env: Some(vec![EnvVar { name: "SMELT_GIT_REMOTE".into(), value: Some(manifest.job.repo.clone()), ..Default::default() }])` to the `main_container` struct literal — insert the field before `..Default::default()`.

5. Update `test_generate_pod_spec_snapshot` in `k8s.rs` to assert both `json.contains("\"SMELT_GIT_REMOTE\"")` and `json.contains("\"git@github.com:example/repo.git\"")` — the test manifest uses `repo = "git@github.com:example/repo.git"` so both should appear in the serialized Pod JSON.

6. In `crates/smelt-cli/src/commands/run.rs` Phase 8 collect block, after the `let git = smelt_core::GitCli::new(git_binary, repo_path.clone());` line, insert the kubernetes branch:
   ```rust
   if manifest.environment.runtime == "kubernetes" {
       tracing::info!(branch = %manifest.merge.target, "fetching result branch from remote");
       git.fetch_ref("origin", &format!("+{t}:{t}", t = manifest.merge.target))
           .await
           .with_context(|| "Phase 8: failed to fetch result branch from remote")?;
   }
   ```
   Ensure `tracing` is already imported in `run.rs` (it is via `use tracing`).

## Must-Haves

- [ ] `GitOps` trait has `fn fetch_ref(&self, remote: &str, refspec: &str) -> impl Future<Output = Result<()>> + Send` — compiles without warnings
- [ ] `GitCli::fetch_ref()` implementation shells out to `git fetch remote refspec`; unit test `test_fetch_ref_creates_local_branch` passes (uses a real bare git repo)
- [ ] `generate_pod_spec()` produces a Pod JSON containing `"SMELT_GIT_REMOTE"` and `"git@github.com:example/repo.git"` (from the test fixture); snapshot test asserts both strings
- [ ] Phase 8 in `run.rs` calls `git.fetch_ref("origin", "+<target>:<target>")` when `runtime == "kubernetes"` — code compiles, no warnings
- [ ] `cargo test --workspace` shows 0 failures; `cargo test -p smelt-core -- k8s fetch_ref --nocapture` passes

## Verification

- `cargo test -p smelt-core -- k8s --nocapture` — all 10 existing kubernetes tests pass + snapshot assertions for `SMELT_GIT_REMOTE` pass
- `cargo test -p smelt-core -- fetch_ref --nocapture` — `test_fetch_ref_creates_local_branch` passes
- `cargo test --workspace` — 0 failures, existing tests unaffected
- `cargo build -p smelt-cli` — zero errors and zero warnings in modified files

## Observability Impact

- Signals added/changed: `tracing::info!(branch = %..., "fetching result branch from remote")` in Phase 8 kubernetes path — surfaced at `RUST_LOG=smelt_cli=info`
- How a future agent inspects this: `RUST_LOG=smelt_cli=info cargo run -- run manifest.toml` shows the fetch log line when runtime is kubernetes; `git branch -v` after a run shows the fetched local branch
- Failure state exposed: `anyhow::Error` with context `"Phase 8: failed to fetch result branch from remote"` propagates to the CLI and prints to stderr; the `git fetch` stderr (e.g., "repository not found", "Permission denied") is captured by `GitCli::run()` and included in `SmeltError::GitExecution.message`

## Inputs

- `crates/smelt-core/src/git/mod.rs` — `GitOps` trait definition; add `fetch_ref` after `rev_parse`
- `crates/smelt-core/src/git/cli.rs` — `GitCli` impl block; add `fetch_ref` following existing pattern
- `crates/smelt-core/src/k8s.rs` — `generate_pod_spec()` main container struct at line ~155; `k8s_openapi` imports block at top
- `crates/smelt-cli/src/commands/run.rs` — Phase 8 collect block; `let git = GitCli::new(...)` line before `ResultCollector` construction
- S01 decision (D090/D091): `generate_pod_spec()` main container uses `..Default::default()`; `serde_json::to_string_pretty` for snapshot assertions
- S02 decision (D086/D093): `run.rs` Phase 8 is the host-side git fetch point; `runtime == "kubernetes"` string comparison is the dispatch mechanism

## Expected Output

- `crates/smelt-core/src/git/mod.rs` — `fetch_ref` trait method added
- `crates/smelt-core/src/git/cli.rs` — `GitCli::fetch_ref` impl + `test_fetch_ref_creates_local_branch` unit test
- `crates/smelt-core/src/k8s.rs` — `EnvVar` import + `env` field on main container + updated snapshot assertion
- `crates/smelt-cli/src/commands/run.rs` — kubernetes fetch block in Phase 8
- All changes: `cargo test --workspace` green, all existing tests pass, new tests pass
