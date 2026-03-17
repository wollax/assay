# S03: Sequential Merge Runner & Conflict Contract — Research

**Date:** 2026-03-17

## Summary

S03 adds the merge execution layer that runs after parallel session completion. The core challenge is: given N completed session branches from the executor, merge them into the base branch in the correct order using `git merge --no-ff`, re-checking for conflicts after each merge (because merging A changes the base for B). Two ordering strategies are needed: completion-time (default, simple) and file-overlap (greedy algorithm minimizing conflict probability by merging sessions with the most shared files last).

The existing `merge.rs` module provides `merge_check()` — a read-only conflict detector using `git merge-tree --write-tree`. S03 extends this with `merge_execute()` that actually performs `git merge --no-ff` via `Command` (consistent with D008), plus a `merge_runner.rs` in the orchestrate module that sequences merges, invokes the conflict handler closure, and produces a `MergeReport`. The conflict handler follows D001/D026 — a closure receiving `(session_name, conflicting_files, conflict_scan, work_dir)` returning `ConflictAction::Resolved/Skip/Abort`. The default handler returns `Skip`.

This is medium-risk work. The merge execution itself is straightforward (`git merge --no-ff`), but ordering sensitivity (A-then-B ≠ B-then-A) and conflict-abort propagation need careful integration testing with real git repos.

## Recommendation

Build three components:

1. **`merge_execute()` in `crates/assay-core/src/merge.rs`** — performs `git merge --no-ff <branch>` with a structured commit message. Returns a `MergeExecuteResult` with the merge commit SHA, files changed, and success/conflict status. On conflict, aborts the merge (`git merge --abort`) and returns conflict details. Also add `scan_conflict_markers()` and `scan_files_for_markers()` for post-merge validation.

2. **`ordering.rs` in `crates/assay-core/src/orchestrate/`** — pure functions that take completed sessions and produce a merge order. Two strategies: `CompletionTime` (sort by completion timestamp, topological tiebreak) and `FileOverlap` (greedy: sessions with fewer file overlaps with already-merged set go first). Returns `(Vec<CompletedSession>, MergePlan)` for observability.

3. **`merge_runner.rs` in `crates/assay-core/src/orchestrate/`** — the sequencing loop: takes `OrchestratorResult`, filters for `SessionOutcome::Completed`, orders via the chosen strategy, then for each session: merge-check against current base, invoke conflict handler if conflicts, execute merge if clean or resolved, record result. Produces `MergeReport` with per-session merge status.

All new types should go in `assay-types` (serializable report types) or `assay-core` (non-serializable operational types). Feature-gate new orchestrate module files behind the existing `orchestrate` feature.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Git merge execution | `git merge --no-ff` via `std::process::Command` | Consistent with D008 — all git ops shell out. `merge.rs` already has `git_command()` and `git_raw()` helpers. |
| Conflict marker scanning | `grep -rn "<<<<<<< "` or manual file read | Simple string search — `<<<<<<<`, `=======`, `>>>>>>>` markers. ~20 lines of Rust. No regex needed. |
| Merge ordering | Hand-roll sort comparators | Two simple sort strategies on small vectors (≤20 items). No library needed. |

## Existing Code and Patterns

- `crates/assay-core/src/merge.rs` — **Primary extension point.** Has `git_raw()` and `git_command()` private helpers for running git commands. `merge_check()` is the read-only conflict detector using `git merge-tree --write-tree`. `parse_conflicts()` extracts conflict details from merge-tree output. Add `merge_execute()` here following the same pattern. Note: `git_raw()` and `git_command()` are private — either make them pub(crate) or add `merge_execute()` in the same file.

- `crates/assay-types/src/merge.rs` — Merge types: `MergeCheck`, `MergeConflict`, `ConflictType`, `FileChange`, `ChangeType`. All with `deny_unknown_fields`, schemars, inventory. New merge result types should follow this exact pattern.

- `crates/assay-core/src/orchestrate/executor.rs` — `SessionOutcome::Completed` has `branch_name: String`, `changed_files: Vec<String>`, `worktree_path: PathBuf`, and `result: Box<PipelineResult>`. **Important:** Currently the executor populates `branch_name` as `String::new()` and `changed_files` as `Vec::new()` — these are placeholders. The merge runner must tolerate empty branch names or S06 must fix the executor to populate them from the `SetupResult`/`PipelineResult`. The `PipelineResult` does contain `merge_check: Option<MergeCheck>` which has file lists, and the session name maps to a worktree branch name pattern (`assay/<spec-slug>`).

- `crates/assay-core/src/orchestrate/dag.rs` — `DependencyGraph::topological_groups()` returns `Vec<Vec<usize>>` — needed for topological ordering in merge sequence. `name_of(idx)` maps index to session name.

- `crates/assay-types/src/orchestrate.rs` — `OrchestratorStatus`, `SessionRunState`, `FailurePolicy`, `OrchestratorPhase`, `SessionStatus`. The merge report types should follow the same style (full derives, deny_unknown_fields, inventory registration, schema snapshots).

- `crates/assay-core/src/error.rs` — `AssayError` is `#[non_exhaustive]` with `thiserror`. Currently has `MergeCheckRefError`. Add `MergeExecuteError` and `MergeRunnerError` variants for merge execution failures.

## Constraints

- **D001 (closures, not traits):** Conflict handler must be a closure `Fn(&str, &[String], &ConflictScan, &Path) -> ConflictAction`, not a trait. D026 explicitly mandates this.
- **D008 (shell out to git):** `merge_execute()` uses `git merge --no-ff` via `std::process::Command`, same as all other git operations.
- **D019 (topological merge order):** Always merge in topological order. Each merge re-checks against the updated base. This is the core correctness invariant.
- **D025 (port Smelt ordering strategies):** Completion-time and file-overlap strategies. Pure functions, ~150 lines total.
- **D026 (conflict handler contract):** Handler receives `(session_name, conflicting_files, scan, work_dir)` → `Resolved`/`Skip`/`Abort`. Default returns `Skip`. AI resolution deferred to M003.
- **Feature gate:** New files in `orchestrate/` are behind `cfg(feature = "orchestrate")`. The `merge.rs` base extension (`merge_execute`) is NOT feature-gated — it's a general-purpose merge capability.
- **`deny_unknown_fields`:** Required on all new persisted/serializable types.
- **`SessionOutcome::Completed` branch_name is currently empty:** The executor sets `branch_name: String::new()`. The merge runner needs actual branch names. Options: (a) derive from worktree metadata at merge time, (b) fix the executor to populate properly (leaks into S02 scope), (c) accept branch name as derived from session name pattern `assay/<spec-slug>`. Option (c) is cleanest — the merge runner can derive branch names from session/spec names using the same pattern as `worktree::create()`.

## Common Pitfalls

- **Merge on wrong branch:** `git merge --no-ff` merges INTO the current branch. The merge runner must `git checkout <base_branch>` in the project root (not a worktree) before each merge. The project root's working tree must be clean.
- **Dirty working tree blocks merge:** If the project root has uncommitted changes, `git merge` fails. The merge runner should check `git status --porcelain` before starting and fail with an actionable error if dirty.
- **Merge --abort leaves state:** If `git merge --no-ff` conflicts and we call `git merge --abort`, the repo is clean again. But if the process crashes between conflict detection and abort, the repo is in a merge state. The runner should check for in-progress merges at startup (`git merge HEAD` or check `.git/MERGE_HEAD`).
- **File-overlap ordering is NP-hard in general:** The greedy approximation (merge the session with least overlap first) is O(n²) but n ≤ 20, so fine. Don't try to optimize.
- **Branch names from worktrees vs main repo:** Worktree branches exist in the main repo's refspace. `git merge --no-ff assay/auth-flow` works from the main repo as long as the branch exists. No need to reference worktree paths.
- **Re-check after each merge is mandatory:** After merging A into base, the base has changed. B's merge-check from the executor phase is stale. Must re-run `merge_check()` (or just attempt `git merge --no-ff` and handle conflict) for each subsequent session.

## Open Risks

- **`SessionOutcome::Completed` has placeholder values:** `branch_name` and `changed_files` are `String::new()` and `Vec::new()` in the current executor. The merge runner needs branch names. Mitigation: derive from session name using the `assay/<spec-slug>` pattern, or read from `PipelineResult`/worktree metadata. This is a known gap — document the derivation logic clearly.
- **Concurrent merge runner invocation:** Two orchestrator runs merging simultaneously into the same base branch would corrupt state. Mitigation: the merge runner should acquire a file lock (`.assay/merge.lock`) before starting. Low priority for M002 (single user), but worth a TODO.
- **Large merge conflicts:** If a session produces thousands of conflicting files, the `scan_conflict_markers()` function could be slow. Mitigation: cap file scanning at a reasonable limit (e.g., 100 files) with truncation flag. Same pattern as `merge_check`'s `max_conflicts`.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Git merge/conflict handling | N/A — standard git CLI operations | No skill needed |
| Rust testing with real git repos | N/A — use `tempfile` + `git init` pattern already in codebase | No skill needed |

No external skills are relevant. The work is standard git CLI operations and pure Rust logic.

## Sources

- `git merge --no-ff` behavior: standard git documentation. `--no-ff` always creates a merge commit even for fast-forward-able merges, which is desirable for auditability.
- `git merge --abort`: resets to pre-merge state. Safe to call after a conflicted merge.
- Existing `merge.rs` in assay-core: proven pattern for git shell-out with error handling.
- S02 summary forward intelligence: `OrchestratorResult.outcomes` is `Vec<(String, SessionOutcome)>` indexed by session position. Filter for `Completed` variants.
- D019, D025, D026 from DECISIONS.md: define merge ordering and conflict handler contracts.
