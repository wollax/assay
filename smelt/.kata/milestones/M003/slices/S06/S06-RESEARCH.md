# S06: Integration Proof — Research

**Researched:** 2026-03-21
**Domain:** End-to-end integration validation, issue triage, documentation cleanup
**Confidence:** HIGH

## Summary

S06 is the integration-proof slice for M003. All prior slices (S01–S05) have shipped and the workspace compiles cleanly with 197 passing tests and 0 failures (when run via `cargo test --workspace -- --nocapture`). The `smelt-example` external crate at `/tmp/smelt-example/` compiles and passes 3 tests against the real `smelt-core` path dependency. `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps [--features forge]` exits with 0 errors in both variants. The milestone's mechanical contracts are proven; what remains is a live end-to-end run, two documentation warnings, and issue triage.

S06 has two independent tracks. **Track A** (Integration UAT) is the live proof: `smelt run manifest.toml` with a real `GITHUB_TOKEN`, a real GitHub repo, and a real Docker daemon — demonstrating the full loop ending in a merged PR with `smelt watch` exit 0. This requires manual execution with real credentials; it cannot be automated. **Track B** (Cleanup) addresses the three open `cargo doc` warnings, the 32 open issues in `.planning/issues/open/`, and updating example manifests. Track B is fully automatable and should be completed before the UAT run to ensure a clean codebase.

The milestone success criteria state that UAT is required: "Manual UAT: `smelt run` → PR created → `smelt watch` → merge → watch exits 0." S06 should produce a written UAT script that the user can follow, and the plan tasks should separate what the agent can do (cleanup, doc fixes) from what requires real credentials (the live run).

## Recommendation

Plan S06 with two task groups:

1. **T01 — Pre-UAT cleanup**: Fix the 3 `cargo doc` warnings, triage all 32 open issues (close stale ones, fix the ones that are quick wins), and update the example manifests. This ensures the codebase is in a presentable state before the live proof.

2. **T02 — UAT script and live proof**: Write a `S06-UAT.md` test script in the slice directory, execute it against a real GitHub repo (or document what the agent cannot do), and record the results. Mark the milestone complete when the user confirms the UAT.

Issues 001–008 (severity: important) that reference `worktree.rs`, `cli.rs`, and `worktree/mod.rs` are for a pre-M003 code path. `crates/smelt-cli/src/commands/worktree.rs` **does not exist** in the current codebase. These issues should be closed as "stale — worktree command not present in current CLI." Issues with `area: smelt-core` and `severity: suggestion` (009–014) may be quick fixes worth addressing in T01.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Issuing fake API calls for UAT | `wiremock` test pattern from S01 (`forge_for_server()`) | Already proven; use for any automated forge coverage added in S06 |
| External embedding proof | `/tmp/smelt-example/tests/api.rs` (3 passing tests) | Already exists and passes; no need to recreate |
| Dry-run validation of generated manifests | `smelt run --dry-run` via `cargo run --bin smelt` | Already tested in dry_run.rs; use pattern for new smoke tests |

## Existing Code and Patterns

- `crates/smelt-core/src/lib.rs` — `#![deny(missing_docs)]` is present; crate-level doc is complete; `pub use` re-exports are correct
- `crates/smelt-core/src/assay.rs:174` — `/// Maps each [`SessionDef`](crate::manifest::SessionDef)` has a redundant explicit link target; fix: remove `(crate::manifest::SessionDef)` and keep `[`SessionDef`]`
- `crates/smelt-core/src/lib.rs:11` — `[`GitHubForge`]` is an unresolved link in no-forge builds; fix: wrap in `#[cfg_attr(feature = "forge", doc = "...")]` or use `forge::GitHubForge` as the link target
- `crates/smelt-core/src/assay.rs:175` — `build_run_manifest_toml` doc links to `SmeltManifestSession` which is now `pub(crate)`; fix: remove the link from the doc comment or use a generic description instead
- `examples/job-manifest.toml` — Working dry-run example; the comment block is thorough; no `[forge]` section
- `examples/job-manifest-forge.toml` — Forge example created in S02; has a `[forge]` block with placeholder `owner/my-repo`; the comment block could be improved with notes about `smelt watch` usage
- `/tmp/smelt-example/tests/api.rs` — 3 passing tests; `test_githubforge_builds`, `test_jobmanifest_parses_minimal_manifest`, `test_docker_provider_new_does_not_panic` — R005 proven

## Constraints

- UAT requires a **real GitHub repo**, **real `GITHUB_TOKEN`** with `pull_requests: write`, and a **running Docker daemon** — the agent cannot execute this path; only the user can confirm it
- `smelt watch` currently reads from `.smelt/runs/<job_name>/state.toml` (S04 per-job path); any UAT plan must use job names matching the manifest `job.name` field
- The `_get()` + `body_to_string()` pattern in `forge.rs` polls CI status using semi-private octocrab API — no fix needed in S06, but the UAT must confirm this works against a real GitHub repo with CI checks enabled (or accept `CiStatus::Unknown` if no CI is configured on the test repo)
- Issues 001–008 referencing `worktree.rs`/`worktree/mod.rs` are stale — these files were removed in M001/M002 rearchitecting; they must be closed in triage rather than acted upon
- Let-chain MSRV issue: `let-chain-msrv-compat.md` references let-chain syntax at `manifest.rs:145-152`; current `manifest.rs` does NOT use let-chains — this issue is stale/already resolved. Close it.

## Common Pitfalls

- **Attempting automated forge UAT** — The live PR creation/watch loop cannot be mocked end-to-end in a unit test environment (Phase 9 requires real Docker output). The UAT is a human-executed script, not an automated test. Trying to automate it in S06 would require a full Docker + GitHub E2E test harness that's out of scope.
- **Fixing stale issues that reference non-existent files** — `worktree.rs`, `session.rs`, `cli_session.rs` mentioned in issues 001–010 don't exist. Trying to fix them would add code that doesn't compile. Close them as stale.
- **Doc warning fix for `GitHubForge` unresolved link** — The link `[`GitHubForge`]` in `lib.rs:11` only resolves when the `forge` feature is enabled. The correct fix is to either condition the link on `#[cfg(feature = "forge")]` or use `forge::GitHubForge` qualified path. Do NOT remove the link — it's documentation value is real.
- **`smelt watch` state path** — When writing the UAT script, note that `smelt watch <job-name>` needs the job name to match `manifest.job.name` exactly, because it reads `.smelt/runs/<job-name>/state.toml`. If the user runs `smelt run` from the wrong directory, the state file won't exist.

## Open Risks

- **`_get()` API stability in octocrab** — The CI status fetch uses `self.client._get(url)` which is a semi-private octocrab API. If octocrab 0.50+ removes or renames `_get`, S06's live UAT will fail at CI status polling. This won't break PR creation — only CI status display. If the live run confirms `CiStatus::Unknown` on a repo without CI, this is expected behavior.
- **PR already exists on test repo** — If the user runs UAT multiple times on the same head branch, `create_pr()` will receive a 422 from GitHub. The error message is reasonable (`"Phase 9: failed to create GitHub PR"`) but not specific about "PR already exists." The workaround is to use a fresh branch name each time or delete the PR between runs.
- **`review_count` is inline diff comments not approval count** — D054 is still in effect. The UAT script should note that `review_count` in `smelt status` shows inline code comments, not formal approvals/reviews. This may surprise users.
- **3 cargo doc warnings** — None are errors (they don't block builds or `deny(missing_docs)`), but they should be cleaned up in T01 before the milestone is closed.

## Issue Triage Summary

32 open issues across `.planning/issues/open/`. Categorized:

### Close as stale (referencing non-existent files/code):
- `001-branch-is-merged-fragile-parsing.md` — references `git/cli.rs:branch_is_merged`; **file exists**, function is in `git/cli.rs`. NOT stale — valid but low-priority.
- `002-dialoguer-non-tty.md` — references `commands/worktree.rs` which **does not exist**. STALE.
- `003-execute-list-error-mapping.md` — references `commands/worktree.rs::execute_list`. STALE.
- `006-worktree-state-pub-fields.md` — references `worktree/state.rs` which **does not exist** in smelt-core/src. STALE.
- `007-untested-error-paths.md` — references `worktree/mod.rs` which **does not exist**. STALE.
- `008-mock-gitops-tests.md` — references WorktreeManager in `worktree/mod.rs`. STALE.
- `010-session-status-display.md` — likely stale (references old session command). Verify.
- `cli-session-error-handling.md` — references `session.rs` CLI command. STALE.
- `test-single-session-manifest.md` — references `cli_session.rs`. STALE.
- `let-chain-msrv-compat.md` — `manifest.rs:145-152` does NOT use let-chains. STALE.
- `test-exit-after-*` — need verification (may reference old session-based test infra).

### Actionable (fix in T01):
- `011-chrono-duration-deprecation.md` — `chrono::Duration::hours()` deprecated; fix: `chrono::TimeDelta::hours()`. Quick fix, 1 line. BUT: need to verify if `orphan.rs` even exists.
- `012-resolve-status-dead-code.md` — `WorktreeManager::resolve_status()` dead code; but `worktree/mod.rs` doesn't exist. Likely STALE.
- `004-state-deserialization-naming.md` — `SmeltError::StateDeserialization` naming issue. Low-effort rename if `StateDeserialization` exists in current `error.rs`.
- `dry-violation-git-cli-run.md` — `git/cli.rs:30-53` DRY violation between `run` and `run_in`. Minor refactor.
- `013-thiserror-display-impls.md`, `014-must-use-remove-result.md` — suggestion-level; evaluate on inspection.

### Not actionable by agent (requires human decision):
- `005-prune-toctou.md` — references `prune()`/`detect_orphans()` in `worktree/mod.rs`. STALE if worktree module gone.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust documentation | none needed — `cargo doc` is sufficient | n/a |
| GitHub API / octocrab | none needed — S01 patterns established | n/a |

## Sources

- Codebase inspection: `crates/smelt-core/src/`, `crates/smelt-cli/src/commands/`, `examples/`, `.planning/issues/open/` — direct file reads
- S01–S05 summaries (preloaded) — forward intelligence sections
- `cargo doc`, `cargo test --workspace`, `cargo build` — live verification of current state
