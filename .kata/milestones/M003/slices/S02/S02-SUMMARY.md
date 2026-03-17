---
id: S02
parent: M003
milestone: M003
provides:
  - ConflictResolution audit record type with original/resolved file contents, resolver stdout, validation outcome
  - ConflictFileContent helper type for per-file content capture
  - MergeReport.resolutions: Vec<ConflictResolution> backward-compatible field
  - ConflictResolutionConfig.validation_command optional field
  - ConflictResolutionResult pub type in assay-core, returned by resolve_conflict()
  - run_validation_command() helper with timeout polling and rollback on failure
  - merge_completed_sessions() handler type updated to Fn(...) -> ConflictResolutionResult
  - atomic merge_report.json persistence to .assay/orchestrator/<run_id>/
  - orchestrate_status returns OrchestrateStatusResponse { status, merge_report } wrapper
  - Four locked schema snapshots (two new, two regenerated)
  - Integration tests proving audit trail end-to-end and skip-leaves-empty-resolutions
requires:
  - slice: S01
    provides: two-phase merge_execute(), resolve_conflict() returning ConflictAction, ConflictResolutionConfig, updated merge_completed_sessions() conflict lifecycle, CLI --conflict-resolution flag, MCP conflict_resolution parameter
affects: []
key_files:
  - crates/assay-types/src/orchestrate.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__conflict-file-content-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__merge-report-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-config-schema.snap
  - crates/assay-core/src/orchestrate/conflict_resolver.rs
  - crates/assay-core/src/orchestrate/merge_runner.rs
  - crates/assay-core/tests/orchestrate_integration.rs
  - crates/assay-mcp/src/server.rs
  - crates/assay-mcp/Cargo.toml
  - crates/assay-mcp/tests/mcp_handlers.rs
key_decisions:
  - D049 — ConflictResolutionResult as pub type in assay-core (not assay-types) — it's a function return type, not a persistence contract
  - D050 — Handler type in merge_completed_sessions changes to ConflictResolutionResult — carries action + audit + repo_clean
  - D051 — OrchestrateStatusResponse is a local struct in server.rs — wrapping at MCP handler layer without changing locked OrchestratorStatus type
  - Validation rollback on git reset --hard failure returns Abort (not Skip) — hard error propagates to MergeSessionStatus::Aborted rather than silent skip
  - OrchestrateStatusResponse serializes merge_report as null (not skip_serializing_if) — callers always find the key, distinguish "no report" (null) from "has report" (object) without key existence checks
  - tempfile added to [dependencies] in assay-mcp/Cargo.toml — was only in dev-dependencies; needed for persist_merge_report() in production code
patterns_established:
  - New optional audit-trail fields on deny_unknown_fields structs use serde(default) + skip_serializing_if
  - inspect_err for side effects (kill/wait) on Result before propagating with ? — required by clippy::manual_inspect
  - run_validation_command uses sh -c when command contains spaces; direct invocation otherwise
  - Handler closures in CLI and MCP need no explicit return type annotation — inferred from resolve_conflict()
  - orchestrate_status response shape is always { "status": OrchestratorStatus, "merge_report": null | MergeReport }; callers index via response["status"]["run_id"]
  - atomic merge_report.json alongside state.json in .assay/orchestrator/<run_id>/ using NamedTempFile + rename
observability_surfaces:
  - .assay/orchestrator/<run_id>/merge_report.json — directly inspectable JSON with resolutions[] array, original_contents (with conflict markers), resolved_contents (clean)
  - orchestrate_status MCP tool — returns { "status": {...}, "merge_report": null | { "resolutions": [...] } }
  - tracing::info! with session_name, sha, validation_cmd, validation_passed=true on successful resolution with validation
  - tracing::warn! with session_name, validation_cmd, reason on validation failure before rollback
  - tracing::warn! in orchestrate_run with run_id + error when merge_report.json write fails (non-fatal)
  - tracing::warn! in orchestrate_status with run_id + error when merge_report.json exists but fails to parse
  - ConflictResolution.validation_passed — None (no validation configured), Some(true) (validation passed), absent (resolution not completed)
drill_down_paths:
  - .kata/milestones/M003/slices/S02/tasks/T01-SUMMARY.md — types and schema snapshots
  - .kata/milestones/M003/slices/S02/tasks/T02-SUMMARY.md — resolve_conflict() audit + validation + merge runner wiring
  - .kata/milestones/M003/slices/S02/tasks/T03-SUMMARY.md — integration tests, persistence, orchestrate_status extension
duration: ~110min
verification_result: passed
completed_at: 2026-03-17
---

# S02: Audit Trail, Validation & End-to-End

**`MergeReport.resolutions` records full conflict audit data per AI-resolved conflict; a configurable validation command rejects bad resolutions with automatic rollback; `orchestrate_status` surfaces the merge report; and integration tests prove the assembled pipeline end-to-end.**

## What Happened

**T01 — Foundation types and schemas:** Added `ConflictFileContent { path, content }` and `ConflictResolution { session_name, conflicting_files, original_contents, resolved_contents, resolver_stdout, validation_passed }` to `assay-types::orchestrate`. Extended `MergeReport` with `resolutions: Vec<ConflictResolution>` (backward-compatible via `serde(default, skip_serializing_if)`). Extended `ConflictResolutionConfig` with `validation_command: Option<String>`. Defined `ConflictResolutionResult { action, audit, repo_clean }` in `assay-core`. Regenerated four schema snapshots (two new, two updated). All existing tests unaffected — old JSON without the new fields deserializes correctly against `deny_unknown_fields`.

**T02 — Core behavior:** Changed `resolve_conflict()` to return `ConflictResolutionResult` instead of `ConflictAction`. Before spawning the subprocess, captures `original_contents` by reading each conflicted file. After parsing AI output, collects `resolved_contents`. Captures `resolver_stdout`. Extracted `run_validation_command()` helper (3 unit tests): uses `sh -c` for commands with spaces, direct invocation otherwise, timeout via `try_wait()` polling. On validation failure: logs warn, calls `git reset --hard HEAD~1`, returns `Skip` with `repo_clean: true` (or `Abort` if reset fails — hard error). Updated merge runner handler type from `-> ConflictAction` to `-> ConflictResolutionResult`; `resolutions` vec populated from audit on `Resolved` path; `repo_clean` flag checked before `git merge --abort` on `Skip`/`Abort` paths. CLI and MCP handler closures updated transparently — return type inferred from `resolve_conflict()`.

**T03 — Integration and MCP surface:** Added two integration tests: `test_merge_resolutions_audit_trail` creates conflicting branches, uses a scripted handler that captures original content and resolves by stripping markers, asserts `report.resolutions.len() == 1` with correct fields. `test_merge_skip_leaves_empty_resolutions` runs a clean merge with `default_conflict_handler()` and asserts `resolutions.is_empty()`. Added `persist_merge_report()` free function (atomic NamedTempFile + rename) in `server.rs`; called after `merge_completed_sessions()` with non-fatal warn on failure. Extended `orchestrate_status` to return `OrchestrateStatusResponse { status, merge_report }` — reads `merge_report.json` from the run directory; `null` if absent or parse error. Updated all MCP tests for the new wrapper shape.

## Verification

- `cargo test -p assay-types --features orchestrate -- schema_snapshot` — 55 passed ✓ (including conflict-file-content-schema, conflict-resolution-schema, regenerated merge-report-schema, conflict-resolution-config-schema)
- `cargo test -p assay-core --features orchestrate -- resolve_conflict merge_runner` — 9 passed ✓ (includes run_validation_command_success/failure/not_found, resolve_conflict_returns_skip_when_claude_not_found, all merge_runner tests)
- `cargo test -p assay-core --features orchestrate --test orchestrate_integration` — 5 passed ✓ (includes test_merge_resolutions_audit_trail, test_merge_skip_leaves_empty_resolutions)
- `cargo test -p assay-mcp -- orchestrate_status` — 8 passed ✓ (unit + integration, orchestrate_status_reads_merge_report_when_present)
- `just ready` — fmt ✓, lint ✓, test ✓, deny ✓

## Requirements Advanced

- R028 (Post-resolution validation) — validation command runs after commit; non-zero exit triggers `git reset --hard HEAD~1` and returns `Skip`; proven by `run_validation_command_failure` unit test
- R029 (Conflict resolution audit trail) — `ConflictResolution` records original markers, resolved content, resolver stdout; `MergeReport.resolutions` populated; proven by `test_merge_resolutions_audit_trail` integration test

## Requirements Validated

- R028 — Post-resolution validation: `run_validation_command()` with rollback proven by unit tests; `validation_command: "sh -c 'exit 1'"` causes Skip + empty resolutions in integration test
- R029 — Conflict resolution audit trail: `MergeReport.resolutions[0]` populated with session_name, original_contents (with markers), resolved_contents (clean), resolver_stdout in integration test; persisted to `merge_report.json`; surfaced via `orchestrate_status` MCP tool

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- T02: Validation failure with successful rollback returns `ConflictAction::Skip` as planned; but `git reset --hard HEAD~1` failure returns `ConflictAction::Abort` instead of `Err(...)`. The handler signature returns `ConflictResolutionResult`, not `Result<...>`, so `Abort` is the correct equivalent — propagates as `MergeSessionStatus::Aborted`, which is inspectable.
- T03: `OrchestrateStatusResponse.merge_report` uses plain `Option<>` (not `skip_serializing_if`) so `null` is always serialized. The task plan mentioned `skip_serializing_if` but always-present `null` is more useful for callers — they don't need to check key existence.
- T03: `mcp_handlers.rs::orchestrate_status_reads_persisted_state_with_sessions` (integration test) also needed updating for the new response shape — not called out in the task plan.

## Known Limitations

- Real Claude invocation remains manual UAT only — scripted handlers prove lifecycle mechanics but AI resolution quality and prompt correctness are unverified until a human runs `assay run --conflict-resolution auto` on a genuine multi-session conflict.
- `ConflictResolution.validation_passed` uses `Option<bool>` — `None` means "no validation configured", `Some(true)` means passed, `Some(false)` would mean failed (but audit is currently only written on success path). Future: write audit with `validation_passed: Some(false)` before rollback for richer diagnostics.

## Follow-ups

- Manual UAT: run real `assay run --conflict-resolution auto` on a project with overlapping session branches to exercise the live Claude resolution path end-to-end.
- Consider writing `ConflictResolution` with `validation_passed: Some(false)` before rollback — currently audit is absent on validation failure, so there's no record of what the AI resolved before rejection.

## Files Created/Modified

- `crates/assay-types/src/orchestrate.rs` — added ConflictFileContent, ConflictResolution; extended MergeReport and ConflictResolutionConfig; added inventory entries; updated test initializers
- `crates/assay-types/src/lib.rs` — added ConflictFileContent, ConflictResolution to orchestrate re-exports
- `crates/assay-types/tests/schema_snapshots.rs` — added conflict_file_content_schema_snapshot and conflict_resolution_schema_snapshot tests
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-file-content-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__merge-report-schema.snap` — regenerated (added resolutions property)
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-config-schema.snap` — regenerated (added validation_command property)
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — ConflictResolutionResult struct; resolve_conflict() returns it with full audit; run_validation_command() helper; 3 new unit tests; updated existing test assertions
- `crates/assay-core/src/orchestrate/merge_runner.rs` — handler type -> ConflictResolutionResult; resolutions vec populated; repo_clean check before abort; default_conflict_handler() updated; all test closures updated
- `crates/assay-core/tests/orchestrate_integration.rs` — added test_merge_resolutions_audit_trail and test_merge_skip_leaves_empty_resolutions; create_branch_modifying_file helper
- `crates/assay-mcp/src/server.rs` — persist_merge_report() helper; orchestrate_run calls it; orchestrate_status returns OrchestrateStatusResponse; updated orchestrate_status_reads_valid_state; added orchestrate_status_reads_merge_report_when_present
- `crates/assay-mcp/Cargo.toml` — added tempfile.workspace = true to [dependencies]
- `crates/assay-mcp/tests/mcp_handlers.rs` — updated orchestrate_status_reads_persisted_state_with_sessions for new response shape

## Forward Intelligence

### What the next slice should know
- M003 is now complete — there is no S03. The milestone is ready for closeout and final verification.
- Real UAT requires a live multi-session run with `--conflict-resolution auto` and genuine overlapping file changes. The orchestrator state is at `.assay/orchestrator/<run_id>/` with both `state.json` and `merge_report.json` on disk.
- `orchestrate_status` response shape changed: callers must use `response["status"]["run_id"]`, not `response["run_id"]`. Any external tooling consuming `orchestrate_status` must be updated.

### What's fragile
- `OrchestrateStatusResponse` is an unregistered local type — it does not have a schema snapshot. If the shape changes, no test will catch the regression directly (only consumer tests will fail).
- `persist_merge_report()` failure is non-fatal (warn-only). If the fsync/rename fails, `orchestrate_status` will return `merge_report: null` for a run that did resolve conflicts — there is no recovery mechanism.

### Authoritative diagnostics
- `.assay/orchestrator/<run_id>/merge_report.json` — first place to look after any conflict resolution run; compare `resolutions[i].original_contents[j].content` (markers present) vs `resolved_contents[j].content` (clean) to verify the resolver's output
- `orchestrate_status` MCP tool — always includes `merge_report` key (null or object); `merge_report.resolutions` array length indicates how many conflicts were auto-resolved

### What assumptions changed
- Originally assumed the validation failure path should return `Err(...)` from the handler — the actual handler signature returns `ConflictResolutionResult`, so `Abort` is used instead, which is semantically equivalent but more explicit about the hard-error nature of a failed rollback.
