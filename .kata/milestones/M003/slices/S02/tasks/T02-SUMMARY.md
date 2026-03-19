---
id: T02
parent: S02
milestone: M003
provides:
  - resolve_conflict() returns ConflictResolutionResult with full audit record
  - run_validation_command() helper with 3 unit tests
  - merge_runner handler type changed to ConflictResolutionResult throughout
  - repo_clean flag checked before git merge --abort in merge runner
  - MergeReport.resolutions populated from audit in Resolved path
key_files:
  - crates/assay-core/src/orchestrate/conflict_resolver.rs
  - crates/assay-core/src/orchestrate/merge_runner.rs
key_decisions:
  - Used inspect_err (not map_err) for kill-on-timeout in run_validation_command — required by clippy::manual_inspect
  - Validation rollback returns Abort (not Skip) on git reset --hard failure so the hard error propagates to the merge runner and is surfaced as MergeSessionStatus::Aborted rather than silently skipped
  - repo_clean: false on Resolved path — the merge commit is done and no abort is needed; caller should NOT call git merge --abort
patterns_established:
  - Handlers passed to merge_completed_sessions now return ConflictResolutionResult; closures in CLI and MCP need no explicit annotation — type inferred from resolve_conflict()
  - run_validation_command uses sh -c when command string contains spaces; direct invocation otherwise
  - inspect_err for side effects (kill/wait) on Result before propagating with ?
observability_surfaces:
  - tracing::info! with session_name, sha, validation_cmd, validation_passed=true on successful resolution with validation
  - tracing::info! with session_name, sha, resolved_files on successful resolution without validation
  - tracing::warn! with session_name, validation_cmd, reason on validation failure before rollback
  - tracing::warn! with session_name, error on git reset --hard HEAD~1 failure (before Abort return)
  - MergeReport.resolutions[i] fields: session_name, conflicting_files, original_contents, resolved_contents, resolver_stdout, validation_passed
duration: ~45min
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Implement resolve_conflict() Audit Capture + Validation + Update All Callers

**Changed `resolve_conflict()` from `-> ConflictAction` to `-> ConflictResolutionResult`, adding original/resolved file content capture, optional validation with rollback, and `repo_clean`-aware abort logic in the merge runner.**

## What Happened

**Step 1 — `run_validation_command()` helper:** Extracted into `conflict_resolver.rs` with three unit tests (`success`, `failure`, `not_found`). Uses `sh -c` for commands with spaces, direct invocation otherwise. Timeout uses the existing `wait_with_timeout()` polling pattern. `inspect_err` used for kill-on-timeout cleanup (required by `clippy::manual_inspect`).

**Step 2 — `resolve_conflict()` signature change:** Now returns `ConflictResolutionResult`. Before spawning the subprocess, captures `original_contents: Vec<ConflictFileContent>` by reading each conflicted file. After parsing the AI output, collects `resolved_contents` from the AI-provided content. Captures `resolver_stdout` as-is. On success path: if `validation_command` is Some, calls `run_validation_command()`; on failure, logs warn and calls `git reset --hard HEAD~1`. On reset failure returns `Abort` (hard error — repo state unknown); on reset success returns `Skip` with `repo_clean: true`. On validation pass or no validation configured: returns `ConflictResolutionResult { action: Resolved(sha), audit: Some(...), repo_clean: false }`.

**Step 3 — test update:** `resolve_conflict_returns_skip_when_claude_not_found` now asserts `result.action == Skip`, `!result.repo_clean`, `result.audit.is_none()`.

**Steps 4–6 — merge_runner.rs:** Added `use crate::orchestrate::conflict_resolver::ConflictResolutionResult`. Changed handler trait bound from `-> ConflictAction` to `-> ConflictResolutionResult`. Added `let mut resolutions: Vec<ConflictResolution> = Vec::new()` at loop start. Two-phase catch_unwind path destructures `ConflictResolutionResult`; on `Resolved` pushes audit to `resolutions` if Some; on `Skip`/`Abort` checks `repo_clean` before calling `git merge --abort`. Default path (non-two-phase) also handles audit. Final `MergeReport` uses `resolutions` (not `vec![]`). `default_conflict_handler()` now returns `ConflictResolutionResult { Skip, None, false }`.

**Steps 7–8 — test closures and callers:** All inline test closures in merge_runner.rs tests updated to return `ConflictResolutionResult`. CLI (`run.rs`) and MCP (`server.rs`) handler closures call `resolve_conflict()` directly — return type inferred automatically, no annotation changes needed.

## Verification

```
cargo test -p assay-core --features orchestrate
# 764 passed — includes:
# run_validation_command_success, run_validation_command_failure, run_validation_command_not_found
# resolve_conflict_returns_skip_when_claude_not_found (updated assertions)
# all 8 merge_runner tests pass with updated handler closures

cargo test -p assay-cli    # 27 passed
cargo test -p assay-mcp    # 29 passed
just ready                 # fmt ✓, lint ✓, test ✓, deny ✓
```

## Diagnostics

- `MergeReport.resolutions[i]` in the merge report (in-memory and persisted to `.assay/orchestrator/<run_id>/merge_report.json`) contains `session_name`, `original_contents` (with conflict markers), `resolved_contents` (AI output), `resolver_stdout`, and `validation_passed: Some(true)` when validation configured.
- `ConflictResolution.validation_passed: None` when no validation configured; `audit: None` when resolver skipped (absence of audit = resolution was not completed).
- `MergeSessionResult.error` for `ConflictSkipped` with `repo_clean: true` indicates validation failure + successful rollback; `repo_clean: false` indicates the handler chose Skip before committing (merge was aborted).

## Deviations

- Validation failure with successful rollback returns `ConflictAction::Skip` (as planned), but `git reset --hard HEAD~1` failure returns `ConflictAction::Abort` instead of propagating as `Err(...)`. This matches the intent of "propagate as hard error" — `Abort` stops the merge sequence and surfaces in `MergeSessionStatus::Aborted`, which is inspectable. The plan said `Err(...)` but the handler signature returns `ConflictResolutionResult`, not `Result<...>`, so `Abort` is the correct equivalent.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — `resolve_conflict()` returns `ConflictResolutionResult`; `run_validation_command()` added with 3 unit tests; import adds `ConflictFileContent`; test updated
- `crates/assay-core/src/orchestrate/merge_runner.rs` — handler type `-> ConflictResolutionResult`; `resolutions` vec populated; `repo_clean` check before abort; `default_conflict_handler()` updated; all test closures updated
