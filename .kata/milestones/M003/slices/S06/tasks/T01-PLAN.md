---
estimated_steps: 6
estimated_files: 7
---

# T01: Fix doc warnings, triage issues, and apply quick-win code fixes

**Slice:** S06 — Integration Proof
**Milestone:** M003

## Description

Three `cargo doc` warnings have been present since S05 shipped: an unresolved link to `GitHubForge` in `lib.rs` (only when the `forge` feature is disabled), a redundant explicit link target in `assay.rs`, and a public doc comment linking to a now-private type (`SmeltManifestSession`). Fix all three. Then apply two small but clear code quality improvements in `git/cli.rs`: DRY-up the `run()` / `run_in()` duplication and fix the `branch_is_merged()` parsing fragility. Finally, triage all 32 open issues: move stale ones (referencing non-existent files) to a new `.planning/issues/closed/` directory, and update the forge example manifest with post-run usage notes.

After this task, `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps` produces 0 warnings in both feature variants, `cargo test --workspace` is still green, and the issues directory reflects actual project state.

## Steps

1. **Fix `crates/smelt-core/src/lib.rs` doc link** — on the `forge` feature-flag bullet point, change `` [`GitHubForge`] `` to `` `GitHubForge` `` (backtick-only, no doc link). The prose already explains it's in the `forge` module; the link is not needed and doesn't resolve without the feature.

2. **Fix `crates/smelt-core/src/assay.rs` doc comments** — in the `build_run_manifest_toml` doc comment: (a) remove the explicit disambiguation `(crate::manifest::SessionDef)` from the `[`SessionDef`]` reference so it becomes just `` [`SessionDef`] `` (rustdoc resolves it automatically); (b) change `` [`SmeltManifestSession`] `` to `` `SmeltManifestSession` `` — this struct is `pub(crate)` (D067) and cannot be linked from public docs.

3. **Fix `run()` DRY violation in `crates/smelt-core/src/git/cli.rs`** — replace the body of the `run()` method with a single delegation: `self.run_in(&self.repo_root, args).await`. Delete the duplicated `Command::new` / output handling block that was previously there. Confirm `run_in` signature accepts `&Path` — `&self.repo_root` satisfies this.

4. **Fix `branch_is_merged()` fragility in `crates/smelt-core/src/git/cli.rs`** — replace the `.trim().trim_start_matches("* ")` chain with a pattern using `strip_prefix`:
   ```rust
   Ok(output.lines().any(|line| {
       let name = line.trim();
       let name = name.strip_prefix("* ").unwrap_or(name);
       name == branch_name
   }))
   ```
   `strip_prefix` removes at most one occurrence of `"* "` (exact string, not a char pattern), which is semantically correct for the `git branch --merged` output format. This replaces `trim_start_matches("* ")` which is a repeated-stripping operation over individual chars, not an exact prefix.

5. **Triage issues** — create `.planning/issues/closed/` directory. Move the following stale issues into it (these reference files that don't exist in the current codebase):
   - `002-dialoguer-non-tty.md` — references `commands/worktree.rs` (removed)
   - `003-execute-list-error-mapping.md` — references `commands/worktree.rs` (removed)
   - `005-prune-toctou.md` — references `worktree/mod.rs` (removed)
   - `006-worktree-state-pub-fields.md` — references `worktree/state.rs` (removed)
   - `007-untested-error-paths.md` — references `worktree/mod.rs` (removed)
   - `008-mock-gitops-tests.md` — references `WorktreeManager` in `worktree/mod.rs` (removed)
   - `009-session-name-validation.md` — references `WorktreeManager::create()` in `worktree/mod.rs` (removed)
   - `010-session-status-display.md` — references old session command (removed)
   - `011-chrono-duration-deprecation.md` — references `worktree/orphan.rs` (removed)
   - `012-resolve-status-dead-code.md` — references `WorktreeManager::resolve_status()` (removed)
   - `014-must-use-remove-result.md` — references `WorktreeManager::remove()` (removed)
   - `cli-session-error-handling.md` — references `session.rs` CLI command (removed)
   - `test-single-session-manifest.md` — references `cli_session.rs` (removed)
   - `let-chain-msrv-compat.md` — `manifest.rs:145-152` does not use let-chains (already fixed)
   - `004-state-deserialization-naming.md` — `StateDeserialization` variant doesn't exist in `error.rs` (never added or already renamed)
   - `test-manifest-load-from-file.md` — `manifest.rs` already has `test_load_from_file` and `test_load_nonexistent_file` tests (already done)
   - `test-worktree-create-failure.md` — references `WorktreeManager` (removed)
   - `test-exit-after-zero.md`, `test-exit-after-exceeds-steps.md`, `test-exit-after-negative-assertion.md` — reference `script.rs` session execution infrastructure (removed)
   - `partial-failure-single-file-step.md` — references `script.rs` (removed)
   - `extract-shared-test-utils.md` — references `runner.rs` and `script.rs` (removed)
   - `unnecessary-clone-base-ref.md` — references `runner.rs` (removed)
   - `unnecessary-clone-script-content.md` — references `script.rs` (removed)
   - `platform-compat-process-kill.md` — references `process.rs` (removed)
   
   Leave open (valid for future work): `001-branch-is-merged-fragile-parsing.md` (fixed in this task — move to closed after applying fix), `013-thiserror-display-impls.md` (suggestion-level, valid), `dry-violation-git-cli-run.md` (fixed in this task — move to closed after applying fix), `content-source-enum-type-safety.md` (architectural suggestion, valid), `validate-session-name-format.md` (low priority, valid), `test-empty-content-default.md` (check if `script.rs` exists — if not, close as stale), `task-source-enum-type-safety.md` (check if relevant).
   
   After applying the git/cli.rs fixes above, also move `001-branch-is-merged-fragile-parsing.md` and `dry-violation-git-cli-run.md` to `closed/`.

6. **Update `examples/job-manifest-forge.toml`** — add inline comments above the `[forge]` section explaining the post-run workflow: after `smelt run` completes, use `smelt status add-user-auth` to view the PR section, and `smelt watch add-user-auth` to block until the PR is merged. Add `smelt watch --interval-secs 60` as an example for reducing poll frequency. Keep the existing placeholder `owner/my-repo` and `GITHUB_TOKEN` values.

## Must-Haves

- [ ] `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep warning` → empty output
- [ ] `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge 2>&1 | grep warning` → empty output
- [ ] `cargo test -p smelt-core -q` → all pass (no regressions from code fixes)
- [ ] `cargo test --workspace -q` → all pass
- [ ] `grep -n "run_in" crates/smelt-core/src/git/cli.rs` shows `run()` delegates via `run_in`
- [ ] `grep -n "strip_prefix" crates/smelt-core/src/git/cli.rs` shows fix in `branch_is_merged`
- [ ] `ls .planning/issues/closed/ | wc -l` → ≥20 files
- [ ] `grep -A3 '\[forge\]' examples/job-manifest-forge.toml` shows watch/status usage comments

## Verification

- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep -c warning` should return 0
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge 2>&1 | grep -c warning` should return 0
- `cargo test --workspace -q 2>&1 | tail -5` — confirm all suites show `ok`
- `ls .planning/issues/open/` — inspect remaining issues are only the valid ones

## Observability Impact

- Signals added/changed: None — this is a pure cleanup task
- How a future agent inspects this: `RUSTDOCFLAGS="-D missing_docs" cargo doc` is the single diagnostic for doc health; `ls .planning/issues/open/` shows remaining actionable issues
- Failure state exposed: Any regression in `cargo test` after the `run()` delegation change would be immediately visible; `branch_is_merged` tests in `cli.rs` cover the fix

## Inputs

- `crates/smelt-core/src/lib.rs` — line 11 has `[`GitHubForge`]` that needs to become `` `GitHubForge` ``
- `crates/smelt-core/src/assay.rs` — lines ~174-175 have the two doc link issues
- `crates/smelt-core/src/git/cli.rs` — lines ~29-53 have the DRY duplication; lines ~166-176 have the fragile `branch_is_merged` parsing
- `examples/job-manifest-forge.toml` — existing forge example without watch/status notes
- S06-RESEARCH.md issue triage analysis — canonical authority on which issues are stale

## Expected Output

- `crates/smelt-core/src/lib.rs` — 1-line change: backtick-only `GitHubForge` reference
- `crates/smelt-core/src/assay.rs` — 2-line change: cleaned doc comments on `build_run_manifest_toml`
- `crates/smelt-core/src/git/cli.rs` — `run()` body replaced with one-liner delegation; `branch_is_merged()` uses `strip_prefix`
- `.planning/issues/closed/` — new directory with ≥20 stale issue files
- `examples/job-manifest-forge.toml` — updated with watch/status usage comments above `[forge]`
