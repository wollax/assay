# S02: Audit Trail, Validation & End-to-End

**Goal:** `MergeReport` includes a full `ConflictResolution` audit record for every AI-resolved conflict; a configurable validation command runs after resolution and rejects bad merges; `orchestrate_status` surfaces resolution details; and an end-to-end integration test proves the assembled pipeline.

**Demo:** After running `merge_completed_sessions()` with a scripted handler that captures audit data, `MergeReport.resolutions` contains one `ConflictResolution` entry with the session name, original conflict markers, resolved content, and resolver stdout. A validation command configured as `echo ok` passes and the entry persists; `sh -c 'exit 1'` causes `ConflictSkipped` with an empty `resolutions` vec and a clean repo. `orchestrate_status` for a finished run returns both the orchestrator state and the merge report.

## Must-Haves

- `ConflictResolution` type in `assay-types::orchestrate` recording: session name, conflicting files, original contents (with markers), resolved contents, resolver stdout, validation result
- `ConflictFileContent` helper type (`path`, `content`) used by `ConflictResolution`
- `MergeReport.resolutions: Vec<ConflictResolution>` with `serde(default, skip_serializing_if = "Vec::is_empty")` — backward-compatible with pre-existing persisted reports
- `validation_command: Option<String>` on `ConflictResolutionConfig` — when `Some`, runs after commit; non-zero exit triggers `git reset --hard HEAD~1` and returns `Skip`
- `ConflictResolutionResult { action, audit, repo_clean }` pub type in `assay-core` returned by `resolve_conflict()`; merge runner checks `repo_clean` before calling `git merge --abort`
- Handler type in `merge_completed_sessions()` changed from `Fn(...) -> ConflictAction` to `Fn(...) -> ConflictResolutionResult`; `default_conflict_handler()` updated accordingly
- `orchestrate_run` persists `merge_report.json` atomically to `.assay/orchestrator/<run_id>/` after merge
- `orchestrate_status` reads `merge_report.json` when present and returns `{ status, merge_report }` wrapper
- Schema snapshots: new `conflict-resolution-schema`, updated `merge-report-schema`, updated `conflict-resolution-config-schema`
- Integration test: scripted handler populates `ConflictResolution` audit → `MergeReport.resolutions` has one entry with correct fields
- Integration test: validation failure → session `ConflictSkipped`, `resolutions` empty, repo clean

## Proof Level

- This slice proves: integration + final-assembly
- Real runtime required: no (scripted handlers, `echo ok` / `exit 1` as validation commands)
- Human/UAT required: yes — real Claude conflict resolution is manual UAT only

## Verification

- `cargo test -p assay-types schema_snapshots` — all snapshots pass (including new `conflict-resolution-schema`, regenerated `merge-report-schema`, `conflict-resolution-config-schema`)
- `cargo test -p assay-core resolve_conflict` — unit tests for resolver updated (including `run_validation_command_success`, `run_validation_command_failure`)
- `cargo test -p assay-core merge_runner` — inline tests pass with updated handler types
- `cargo test -p assay-core orchestrate_integration` — two new integration tests pass: audit trail and skip-leaves-empty-resolutions
- `cargo test -p assay-mcp orchestrate_status` — updated test passes for new `{ status, merge_report }` wrapper shape, new persistence test passes
- `just ready` — fmt ✓, lint ✓, test ✓, deny ✓

## Observability / Diagnostics

- Runtime signals: `tracing::info!` with `session_name`, `validation_cmd`, `validation_exit_code` when validation passes/fails; `tracing::warn!` on `git reset --hard HEAD~1` failure (becomes hard error, not Skip)
- Inspection surfaces: `orchestrate_status` MCP tool returns `merge_report` with `resolutions` array; `.assay/orchestrator/<run_id>/merge_report.json` on disk for direct inspection
- Failure visibility: `MergeSessionResult.error` for `ConflictSkipped` distinguishes "validation failed" from "claude not found" or "parse error"; `ConflictResolution.validation_passed: false` in audit record when validation rejects a resolution
- Redaction constraints: resolver stdout may contain AI-generated file content — no secrets expected, no redaction required

## Integration Closure

- Upstream surfaces consumed: `ConflictResolutionConfig` and `ConflictAction` from S01 (`assay-types`); `resolve_conflict()` from S01 (`conflict_resolver.rs`); `merge_completed_sessions()` from S01 (`merge_runner.rs`); `persist_state()` atomic pattern from `executor.rs`; `orchestrate_run` and `orchestrate_status` MCP handlers from S01 (`server.rs`)
- New wiring introduced in this slice: `resolve_conflict()` return type changed to `ConflictResolutionResult`; merge runner handler type changed; `orchestrate_run` persists `merge_report.json`; `orchestrate_status` reads `merge_report.json` and wraps response
- What remains before the milestone is truly usable end-to-end: manual UAT (real Claude resolving a genuine conflict via `assay run --conflict-resolution auto` in a real project)

## Tasks

- [x] **T01: Add ConflictResolution type, ConflictResolutionResult struct, and update schemas** `est:30m`
  - Why: Foundation — adds all new types to assay-types and defines `ConflictResolutionResult` in assay-core without changing any function signatures (non-breaking additions only). Regenerates locked schema snapshots.
  - Files: `crates/assay-types/src/orchestrate.rs`, `crates/assay-types/src/lib.rs`, `crates/assay-core/src/orchestrate/conflict_resolver.rs`, `crates/assay-types/tests/schema_snapshots.rs`, snapshot files
  - Do: (1) Add `ConflictFileContent` and `ConflictResolution` structs to `assay-types::orchestrate` with `deny_unknown_fields`, full derives, inventory registration for `conflict-resolution` schema. (2) Add `resolutions: Vec<ConflictResolution>` to `MergeReport` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. (3) Add `validation_command: Option<String>` to `ConflictResolutionConfig` with `#[serde(default, skip_serializing_if = "Option::is_none")]`. (4) Re-export `ConflictResolution`, `ConflictFileContent` from `assay-types/src/lib.rs`. (5) Define `pub struct ConflictResolutionResult { pub action: ConflictAction, pub audit: Option<ConflictResolution>, pub repo_clean: bool }` in `conflict_resolver.rs` (no changes to `resolve_conflict()` yet). (6) Add `conflict_resolution_schema_snapshot()` test to `schema_snapshots.rs`, then run `INSTA_UPDATE=always cargo test -p assay-types schema_snapshots` to regenerate all invalidated snapshots.
  - Verify: `cargo test -p assay-types schema_snapshots` all pass; `just ready` green
  - Done when: All schema tests pass with regenerated snapshots; `MergeReport` deserializes old JSON without `resolutions` field; `ConflictResolutionConfig` deserializes old JSON without `validation_command` field

- [x] **T02: Implement resolve_conflict() audit capture + validation + update all callers** `est:60m`
  - Why: Core behavior — changes `resolve_conflict()` to return `ConflictResolutionResult` with audit data and validation command support; updates all callers (merge runner, CLI, MCP) to consume the new type; updates all inline tests.
  - Files: `crates/assay-core/src/orchestrate/conflict_resolver.rs`, `crates/assay-core/src/orchestrate/merge_runner.rs`, `crates/assay-cli/src/commands/run.rs`, `crates/assay-mcp/src/server.rs`
  - Do: (1) In `conflict_resolver.rs`: capture original file contents (before writing resolved ones) as `Vec<ConflictFileContent>`; make resolver stdout accessible (return it from `spawn_resolver()` or capture it before parsing); extract `run_validation_command(cmd: &str, work_dir: &Path, timeout_secs: u64) -> Result<(), String>` helper that runs the command synchronously with try_wait() polling; after git commit, if `config.validation_command` is `Some(cmd)`, call it — on failure run `git reset --hard HEAD~1` and return `ConflictResolutionResult { action: Skip, audit: None, repo_clean: true }`; on success return `ConflictResolutionResult { action: Resolved(sha), audit: Some(ConflictResolution { session_name, conflicting_files, original_contents, resolved_contents, resolver_stdout, validation_passed: Some(true) }), repo_clean: false }`; on any pre-commit error return `{ action: Skip, audit: None, repo_clean: false }`. (2) Add unit tests for `run_validation_command`: `run_validation_command_success` (`echo ok`), `run_validation_command_failure` (`sh -c 'exit 1'`). (3) Update `resolve_conflict_returns_skip_when_claude_not_found` assertion from `assert_eq!(result, ConflictAction::Skip)` to `assert_eq!(result.action, ConflictAction::Skip); assert!(!result.repo_clean); assert!(result.audit.is_none())`. (4) In `merge_runner.rs`: change handler type to `Fn(...) -> ConflictResolutionResult`; in the two-phase path, destructure `ConflictResolutionResult`; on `Resolved(sha)`, push `audit` to `report.resolutions` when `Some`; on `Skip`, check `result.repo_clean` — if `false` call `git merge --abort`, if `true` skip the abort; update `default_conflict_handler()` to return `ConflictResolutionResult`; update all inline handler closures in tests to return `ConflictResolutionResult`. (5) In `run.rs` (CLI): update the `auto` handler closure — it calls `resolve_conflict()` which now returns `ConflictResolutionResult`, so the closure return type changes from `ConflictAction` to `ConflictResolutionResult` (just pass through). (6) In `server.rs` (MCP): same — the handler closure wrapping `resolve_conflict()` changes return type.
  - Verify: `cargo test -p assay-core resolve_conflict` ✓; `cargo test -p assay-core merge_runner` ✓; `cargo test -p assay-cli run` ✓; `cargo test -p assay-mcp orchestrate_run` ✓; `just ready` green
  - Done when: All existing tests pass with handler type updated; `run_validation_command_success` and `_failure` pass; `resolve_conflict_returns_skip_when_claude_not_found` assertion updated and passing

- [x] **T03: Integration tests, MergeReport persistence, and orchestrate_status extension** `est:45m`
  - Why: Completes the slice — proves the audit trail end-to-end in integration tests, persists the merge report to disk, and surfaces it via `orchestrate_status`.
  - Files: `crates/assay-core/tests/orchestrate_integration.rs`, `crates/assay-mcp/src/server.rs`
  - Do: (1) In `orchestrate_integration.rs`: add `test_merge_resolutions_audit_trail` — create conflicting branches, use a scripted handler that strips markers, stages, commits, and returns `ConflictResolutionResult { action: Resolved(sha), audit: Some(ConflictResolution { session_name, conflicting_files, original_contents: (read before writing), resolved_contents: (read after), resolver_stdout: "scripted".to_string(), validation_passed: None }), repo_clean: false }`; assert `report.resolutions.len() == 1` and field values. (2) Add `test_merge_skip_leaves_empty_resolutions` — use `default_conflict_handler()` with `conflict_resolution_enabled: false` on a clean merge; assert `report.resolutions.is_empty()`. (3) In `server.rs`: add `persist_merge_report()` private function using `NamedTempFile::new_in` + atomic rename (copy pattern from `persist_state()` in `executor.rs`) — write `merge_report.json` to `.assay/orchestrator/<run_id>/`; call it inside the `spawn_blocking` closure after `merge_completed_sessions()` returns (the `run_id` comes from `orch_result.run_id`, the run dir already exists from executor). (4) In `orchestrate_status`: define local `#[derive(Serialize)] struct OrchestrateStatusResponse { status: OrchestratorStatus, merge_report: Option<assay_types::MergeReport> }`; after parsing `state.json`, check if `merge_report.json` exists in the same dir — if so read and parse it, on parse error log a warning and set `None`; serialize the response struct instead of `status` directly. (5) Update `orchestrate_status_reads_valid_state` test to parse the response as `OrchestrateStatusResponse` shape (check that `status` key is present and `merge_report` key is present/null). (6) Add `orchestrate_status_reads_merge_report_when_present` test — write both `state.json` and a valid `merge_report.json` (minimal MergeReport with empty resolutions), assert `merge_report` key is non-null in response. (7) `just ready`
  - Verify: `cargo test -p assay-core orchestrate_integration` two new tests pass; `cargo test -p assay-mcp orchestrate_status` updated + new tests pass; `just ready` fully green
  - Done when: `MergeReport.resolutions` populated in integration test; `merge_report.json` written to disk after orchestrate_run; `orchestrate_status` returns `{ status: ..., merge_report: ... }` wrapper

## Files Likely Touched

- `crates/assay-types/src/orchestrate.rs`
- `crates/assay-types/src/lib.rs`
- `crates/assay-types/tests/schema_snapshots.rs`
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-schema.snap` (new)
- `crates/assay-types/tests/snapshots/schema_snapshots__merge-report-schema.snap` (regenerated)
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-config-schema.snap` (regenerated)
- `crates/assay-core/src/orchestrate/conflict_resolver.rs`
- `crates/assay-core/src/orchestrate/merge_runner.rs`
- `crates/assay-core/tests/orchestrate_integration.rs`
- `crates/assay-cli/src/commands/run.rs`
- `crates/assay-mcp/src/server.rs`
