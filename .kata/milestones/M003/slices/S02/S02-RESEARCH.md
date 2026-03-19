# S02: Audit Trail, Validation & End-to-End — Research

**Date:** 2026-03-17
**Note:** Originally drafted before S01 executed. Updated to reflect S01 is complete.

## Summary

S02 adds three capabilities on top of S01's conflict-resolution infrastructure: (1) a `ConflictResolution` audit record type captured in `MergeReport.resolutions`, (2) a `validation_command` field on `ConflictResolutionConfig` that runs after AI resolution and rejects bad merges, and (3) an end-to-end CLI integration test proving the full pipeline. The slice also extends `orchestrate_status` to surface resolution details.

**S01 is complete.** All S01 deliverables are present on main: `abort_on_conflict` parameter on `merge_execute()`, `conflict_resolver.rs` with `resolve_conflict() -> ConflictAction`, `ConflictResolutionConfig`, `conflict_resolution_enabled` on `MergeRunnerConfig`, `--conflict-resolution auto|skip` CLI flag, and `conflict_resolution` MCP parameter.

**Key interface note for S02:** S01's `resolve_conflict()` returns `ConflictAction` (not a richer result type). S02 must change this to return a `ConflictResolutionResult` struct to carry the audit trail and `repo_clean` flag. See "Recommendation" below.

The primary design risk is the validation-command rollback path. After `resolve_conflict()` commits via `git commit --no-edit`, `MERGE_HEAD` is consumed. If validation then fails, the only cleanup needed is `git reset --hard HEAD~1` (to undo the merge commit) — `git merge --abort` must NOT be called because there is no merge in progress. This differs from the pre-commit Skip path (where the merge runner calls `git merge --abort`). The merge runner must be able to distinguish "resolve returned Skip, MERGE_HEAD still exists" from "resolve returned Skip because validation failed, repo already clean after reset." The cleanest solution is a `ConflictResolutionResult` return type from `resolve_conflict()` that carries `action: ConflictAction` plus `repo_clean: bool`.

The second major concern is persistence: `orchestrate_run` currently returns `MergeReport` in its JSON response but never writes it to disk. `orchestrate_status` reads only `state.json` from the run directory. S02 must make `orchestrate_run` persist `merge_report.json` alongside `state.json`, and extend `orchestrate_status` to read and include it.

## Recommendation

**resolve_conflict() return type:** Change from `ConflictAction` to a new `ConflictResolutionResult` struct: `{ action: ConflictAction, audit: Option<ConflictResolution>, repo_clean: bool }`. `action` carries the original `Resolved`/`Skip`/`Abort` semantics. `audit` is `Some(ConflictResolution)` on success (for audit trail). `repo_clean: true` when validation failed and `git reset --hard HEAD~1` already restored the tree (so the merge runner skips `git merge --abort`). This change is internal to `assay-core` — `ConflictResolution` (the audit type) goes in `assay-types`.

**ConflictResolution type placement:** `assay-types::orchestrate` alongside `MergeReport`. This is a persistence and audit type — serialized in the merge report and read by `orchestrate_status`. Not crate-local like `ConflictResolutionOutput` (D046).

**MergeReport.resolutions field:** Add `resolutions: Vec<ConflictResolution>` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. This is backward-compatible: existing persisted MergeReports without the field still deserialize. The `merge-report-schema.snap` snapshot will be invalidated and must be regenerated (`INSTA_UPDATE=always cargo test -p assay-types`).

**Validation command:** Add `validation_command: Option<String>` to `ConflictResolutionConfig` (in `assay-types`). When `Some`, run synchronously after commit. On non-zero exit: `git reset --hard HEAD~1`, return `ConflictResolutionResult { action: Skip, audit: None, repo_clean: true }`. The merge runner must check `result.repo_clean` before deciding whether to call `git merge --abort`.

**MergeReport persistence:** In the `orchestrate_run` MCP handler, after `merge_completed_sessions()` returns, write `merge_report` to `.assay/orchestrator/<run_id>/merge_report.json` using the same atomic-tempfile pattern as `persist_state()` in `executor.rs`. The `run_id` is available from `orch_result.run_id` and the CWD is available via `resolve_cwd()`.

**orchestrate_status extension:** After successfully reading and parsing `state.json`, check if `merge_report.json` exists in the same directory. If so, read and include it as a `merge_report` field in a wrapper response object. No change to `OrchestratorStatus` type needed — return a new local response struct `OrchestrateStatusResponse { status: OrchestratorStatus, merge_report: Option<MergeReport> }` serialized to JSON.

**End-to-end integration test:** Extend `crates/assay-core/tests/orchestrate_integration.rs` with a test proving: conflicting branches → `merge_completed_sessions` with `conflict_resolution_enabled: true` + validation command (`echo ok`) → `MergeReport.resolutions` has one entry with expected fields. Add a second test: validation command fails (`sh -c 'exit 1'`) → session shows `ConflictSkipped`, repo is clean, `MergeReport.resolutions` is empty.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Validation command execution | Follow `try_wait()` polling pattern established in S01's `resolve_conflict()` | Same sync subprocess with timeout — reuse 100ms poll loop pattern |
| Git undo of merge commit | `git_raw(&["reset", "--hard", "HEAD~1"], work_dir)` using `git_raw()` from `merge.rs` | `git_raw()` handles subprocess + exit code check; `reset --hard HEAD~1` removes one commit and restores clean tree |
| Audit record serialization | Follow `MergeSessionResult` pattern in `assay-types/src/orchestrate.rs` | `deny_unknown_fields` + optional fields with `serde(default, skip_serializing_if)` — established pattern |
| MergeReport persistence | Follow `persist_state()` atomic pattern in `executor.rs` (NamedTempFile + rename) | Atomic writes prevent partially-written files from being read by `orchestrate_status` |
| Schema snapshot registration | Follow `inventory::submit!` pattern in `orchestrate.rs` | 12 existing schema entries; add `conflict-resolution` and regenerate `merge-report` |
| Integration test for validation | Shell commands `echo ok` / `sh -c 'exit 1'` as validation commands | Real subprocesses, no mocking needed, deterministic pass/fail |

## Existing Code and Patterns

- `crates/assay-types/src/orchestrate.rs` — `MergeReport` struct with `deny_unknown_fields`, 13 `inventory::submit!` entries. Add `ConflictResolution` type here and `resolutions: Vec<ConflictResolution>` field to `MergeReport`. The field must use `#[serde(default, skip_serializing_if = "Vec::is_empty")]` for backward compatibility with pre-existing persisted reports.
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` (S01 deliverable) — `resolve_conflict()` currently returns `ConflictAction`. S02 changes this to return `ConflictResolutionResult { action: ConflictAction, audit: Option<ConflictResolution>, repo_clean: bool }`. The validation command runs after `git commit --no-edit`, before returning `Resolved`. On validation failure: `git reset --hard HEAD~1`, return `{ action: Skip, audit: None, repo_clean: true }`.
- `crates/assay-core/src/orchestrate/merge_runner.rs` — S01 adds `conflict_resolution_enabled` to `MergeRunnerConfig` and two-phase lifecycle. S02 changes the conflict branch to call `resolve_conflict()` and receive `ConflictResolutionResult`. On `action: Skip`, check `result.repo_clean`: if `true`, skip `git merge --abort`; if `false`, call it. Populate `MergeReport.resolutions` when `audit` is `Some`.
- `crates/assay-mcp/src/server.rs` — `orchestrate_run` handler calls `merge_completed_sessions()` and returns report in `OrchestrateRunResponse`. S02 adds persistence: after merge, write `merge_report.json` to `cwd/.assay/orchestrator/<run_id>/`. S02 also extends `orchestrate_status` to read `merge_report.json` when present.
- `crates/assay-core/src/orchestrate/executor.rs` — `persist_state()` uses `NamedTempFile::new_in()` + atomic rename. Copy this pattern for merge report persistence.
- `crates/assay-types/tests/schema_snapshots.rs` — all `assert_json_snapshot!` calls. S02 adds `conflict_resolution_schema_snapshot()` and regenerates `merge_report_schema_snapshot()`.
- `crates/assay-types/tests/snapshots/schema_snapshots__merge-report-schema.snap` — **will be invalidated** by the `resolutions` field. Must regenerate with `INSTA_UPDATE=always cargo test -p assay-types schema_snapshots` before committing.

## Constraints

- **`MERGE_HEAD` is consumed by commit** — after `git commit --no-edit` in `resolve_conflict()`, MERGE_HEAD no longer exists. The validation rollback is `git reset --hard HEAD~1` only — calling `git merge --abort` would fail. The merge runner must not call `git merge --abort` when `ConflictResolutionResult.repo_clean = true`.
- **`deny_unknown_fields` on MergeReport** — The `resolutions` field requires `#[serde(default, skip_serializing_if = "Vec::is_empty")]` to remain backward-compatible with persisted MergeReports predating this field. Without both attributes, either old reports fail to deserialize or the schema changes for empty-resolution reports.
- **Validation command is optional** — `validation_command: Option<String>` on `ConflictResolutionConfig`. When `None`, validation step is skipped entirely.
- **Sync core convention (D007)** — Validation command runs synchronously. Same `std::process::Command` + `try_wait()` polling pattern. No tokio runtime.
- **ConflictResolutionOutput is crate-local (D046)** — The AI response schema struct stays in `assay-core`. `ConflictResolution` (the audit record) goes in `assay-types` because it's a persistence type surfaced by `orchestrate_status`.
- **orchestrate_status must not modify OrchestratorStatus schema** — `OrchestratorStatus` has `deny_unknown_fields` and a locked snapshot. Add merge report as a separate `merge_report.json` file; return both as a local response wrapper in `orchestrate_status`.
- **ConflictResolutionConfig schema snapshot will change** — Adding `validation_command` invalidates the S01-locked snapshot. Must regenerate.

## Common Pitfalls

- **Calling `git merge --abort` after committed merge** — After `git commit --no-edit`, there is no merge in progress. Calling `git merge --abort` returns exit code 128 with "not in merge state". The merge runner's S01 cleanup path (`git merge --abort` on Skip) must check `ConflictResolutionResult.repo_clean` to avoid this.
- **Schema snapshot invalidation breaks CI** — Adding `resolutions` to `MergeReport` breaks the locked `merge-report-schema.snap`. Must run `INSTA_UPDATE=always cargo test -p assay-types schema_snapshots` before committing.
- **orchestrate_run doesn't persist MergeReport today** — `orchestrate_run` in `server.rs` constructs `OrchestrateRunResponse { ..., merge_report: Some(merge_report) }` and serializes to the tool response, but never writes to disk. Path reconstruction: `resolve_cwd()?.join(".assay").join("orchestrator").join(&run_id).join("merge_report.json")`.
- **ConflictResolution type must NOT go in assay-core** — If placed crate-local like `ConflictResolutionOutput`, it won't be accessible to `assay-mcp` for `orchestrate_status` deserialization. Must be in `assay-types`.
- **Validation rollback leaves partial state if reset fails** — If `git reset --hard HEAD~1` itself fails, the repo has a committed-but-invalid merge. Return a hard error (not Skip) with the git output. The merge runner should propagate this as `MergeSessionStatus::Failed`, not `ConflictSkipped`.

## Open Risks

- **resolve_conflict() return type change** — S01 defines `resolve_conflict() -> ConflictAction`. S02 changes this to `-> ConflictResolutionResult`. Since these are sequential, S02 must update both the function signature and all callers (merge_runner.rs conflict branch).
- **MergeReport schema regeneration** — The snapshot file for `merge-report-schema` is locked. Regeneration requires `INSTA_UPDATE=always cargo test -p assay-types schema_snapshots`. Document in task plan.
- **orchestrate_status response shape change** — Currently `orchestrate_status` returns `OrchestratorStatus` JSON directly. S02 wraps it in a new local response struct `{ status: ..., merge_report: Option<...> }`. Use additive JSON structure to avoid breaking existing MCP consumers.
