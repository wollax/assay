---
id: T01
parent: S06
milestone: M003
provides:
  - Zero cargo doc warnings in both default and forge feature variants
  - run() delegates to run_in() — DRY violation eliminated
  - branch_is_merged() uses strip_prefix — fragile char-stripping replaced with exact prefix removal
  - 30 stale issues moved to .planning/issues/closed/
  - 2 issues remain open (valid, forward-looking)
  - examples/job-manifest-forge.toml updated with post-run watch/status usage comments
key_files:
  - crates/smelt-core/src/lib.rs
  - crates/smelt-core/src/assay.rs
  - crates/smelt-core/src/git/cli.rs
  - .planning/issues/closed/
  - examples/job-manifest-forge.toml
key_decisions:
  - content-source-enum-type-safety.md and task-source-enum-type-safety.md closed as stale (FileChange/task_file patterns no longer exist in codebase)
patterns_established:
  - "Doc link pattern: use backtick-only for pub(crate) types that cannot be linked from public docs"
  - "run() → run_in() delegation pattern established for GitCli; future methods default to run() for repo_root operations"
observability_surfaces:
  - "RUSTDOCFLAGS=-D missing_docs cargo doc -p smelt-core --no-deps [--features forge] — zero-warning diagnostic"
  - "ls .planning/issues/open/ — shows remaining actionable issues"
duration: 15min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T01: Fix doc warnings, triage issues, and apply quick-win code fixes

**Zero cargo doc warnings in both feature variants; DRY delegation and strip_prefix fix landed in git/cli.rs; 30 stale issues archived; forge example updated with watch/status workflow.**

## What Happened

Three `cargo doc` warnings were eliminated:
1. `lib.rs` line 11: `` [`GitHubForge`] `` → `` `GitHubForge` `` (link doesn't resolve without the `forge` feature)
2. `assay.rs` `build_run_manifest_toml` doc: removed explicit disambiguation `(crate::manifest::SessionDef)` from `SessionDef` link; changed `` [`SmeltManifestSession`] `` to `` `SmeltManifestSession` `` (type is `pub(crate)`, cannot be linked from public docs)

Two code quality fixes applied in `git/cli.rs`:
- `run()` body replaced with `self.run_in(&self.repo_root, args).await` — eliminates the duplicated `Command::new` / output handling block
- `branch_is_merged()` parsing replaced with `strip_prefix("* ").unwrap_or(name)` — removes semantically incorrect `trim_start_matches("* ")` which strips individual chars repeatedly

Issue triage: All 32 open issues reviewed. 30 moved to `.planning/issues/closed/` as stale (reference removed files: worktree/mod.rs, script.rs, runner.rs, process.rs, state.rs, and code constructs that no longer exist). Two issues remain open and valid: `013-thiserror-display-impls.md` and `validate-session-name-format.md`. Additionally triaged as stale: `content-source-enum-type-safety.md` and `task-source-enum-type-safety.md` (FileChange and task_file patterns no longer exist in manifest.rs).

`examples/job-manifest-forge.toml` updated with inline comments above `[forge]` explaining the `smelt status` / `smelt watch` / `smelt watch --interval-secs` post-run workflow.

## Verification

- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1` → no warnings or errors
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge 2>&1` → no warnings or errors
- `cargo test -p smelt-core -q` → 121 tests + 3 doctests passed, 0 failed
- `cargo test --workspace -q` → all suites passed
- `grep -n "run_in" crates/smelt-core/src/git/cli.rs` → line 30 shows `self.run_in(&self.repo_root, args).await`
- `grep -n "strip_prefix" crates/smelt-core/src/git/cli.rs` → line 153 confirms fix in `branch_is_merged`
- `ls .planning/issues/closed/ | wc -l` → 30 files
- `ls .planning/issues/open/` → 2 files (013-thiserror-display-impls.md, validate-session-name-format.md)

## Diagnostics

- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps [--features forge]` is the definitive doc health check — zero warnings expected
- `ls .planning/issues/open/` shows remaining actionable issues

## Deviations

Closed two additional issues not listed in the plan's "leave open" list:
- `content-source-enum-type-safety.md` — plan marked it "valid, architectural suggestion" but `FileChange` and `content_file` no longer exist in the codebase; closed as stale
- `task-source-enum-type-safety.md` — plan said "check if relevant"; `task`/`task_file` pattern doesn't exist in manifest.rs; closed as stale

This brings total closed count to 30 (plan required ≥20 — met).

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/lib.rs` — removed unresolvable `[`GitHubForge`]` doc link
- `crates/smelt-core/src/assay.rs` — fixed two doc comment issues in `build_run_manifest_toml`
- `crates/smelt-core/src/git/cli.rs` — `run()` now delegates to `run_in()`; `branch_is_merged()` uses `strip_prefix`
- `.planning/issues/closed/` — new directory with 30 archived stale issues
- `examples/job-manifest-forge.toml` — added post-run watch/status workflow comments above `[forge]`
