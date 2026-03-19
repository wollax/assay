# S02: Audit Trail, Validation & End-to-End — UAT

**Milestone:** M003
**Written:** 2026-03-17

## UAT Type

- UAT mode: artifact-driven (integration tests) + human-experience (live Claude UAT)
- Why this mode is sufficient: The artifact-driven tests fully prove the mechanics of audit trail capture, validation command execution/rollback, merge report persistence, and orchestrate_status extension using scripted handlers and real git repos. The live Claude UAT is deferred to a human tester who can run `assay run --conflict-resolution auto` on a real project.

## Preconditions

**Artifact-driven (already passing):**
- `just ready` passes on the `kata/M003/S02` branch
- `cargo test -p assay-core --features orchestrate --test orchestrate_integration` — 5 passed

**Live Claude UAT (manual, future):**
- `claude` binary available on PATH and authenticated
- A project with a RunManifest containing at least 2 sessions that modify the same file
- `assay` CLI built and on PATH

## Smoke Test

Run `cargo test -p assay-core --features orchestrate --test orchestrate_integration -- test_merge_resolutions_audit_trail` — should pass and confirm that `report.resolutions.len() == 1` with conflict markers in `original_contents` and clean content in `resolved_contents`.

## Test Cases

### 1. Audit trail populated for resolved conflict

**Artifact test:** `test_merge_resolutions_audit_trail`

1. Two branches both modify `shared.rs` with incompatible content, creating a merge conflict
2. A scripted conflict handler reads original content (with markers), strips markers, writes resolution, commits, and returns `ConflictResolutionResult { action: Resolved(sha), audit: Some(ConflictResolution { session_name, conflicting_files, original_contents, resolved_contents, resolver_stdout: "scripted", validation_passed: None }), repo_clean: false }`
3. `merge_completed_sessions()` returns a `MergeReport`
4. **Expected:** `report.resolutions.len() == 1`; `resolutions[0].session_name == "session-b"`; `resolutions[0].original_contents[0].content` contains conflict markers (`<<<<<<<`); `resolutions[0].resolved_contents[0].content` does not contain markers; `resolutions[0].resolver_stdout == "scripted"`

### 2. Skip leaves resolutions empty

**Artifact test:** `test_merge_skip_leaves_empty_resolutions`

1. Three sessions produce non-conflicting branches
2. `default_conflict_handler()` used (conflict resolution disabled)
3. `merge_completed_sessions()` returns a `MergeReport`
4. **Expected:** `report.resolutions.is_empty() == true`; `report.sessions_merged == 3`

### 3. Validation command failure rejects resolution

**Unit test:** `run_validation_command_failure`

1. Configure `validation_command: Some("sh -c 'exit 1'".to_string())` on `ConflictResolutionConfig`
2. `run_validation_command("sh -c 'exit 1'", work_dir, 30)` called
3. **Expected:** Returns `Err(...)` indicating non-zero exit; after `resolve_conflict()` runs this, the result is `{ action: Skip, audit: None, repo_clean: true }` (rollback succeeded)

### 4. Validation command success allows resolution

**Unit test:** `run_validation_command_success`

1. `run_validation_command("echo ok", work_dir, 30)` called
2. **Expected:** Returns `Ok(())` — resolution proceeds; `ConflictResolution.validation_passed == Some(true)` in audit record

### 5. orchestrate_status returns merge report when present

**MCP test:** `orchestrate_status_reads_merge_report_when_present`

1. Write both `state.json` (valid OrchestratorStatus) and `merge_report.json` (minimal MergeReport with empty resolutions) to a test run directory
2. Call `orchestrate_status` MCP tool with the run_id
3. **Expected:** Response JSON contains `"status"` key (OrchestratorStatus object) and `"merge_report"` key (non-null MergeReport object)

### 6. orchestrate_status returns null merge_report when absent

**MCP test:** `orchestrate_status_reads_valid_state`

1. Write only `state.json` to a test run directory (no merge_report.json)
2. Call `orchestrate_status` MCP tool with the run_id
3. **Expected:** Response JSON contains `"status"` key and `"merge_report": null`

## Edge Cases

### Validation command not found

1. Configure `validation_command: Some("nonexistent-command-xyz".to_string())`
2. `run_validation_command()` called
3. **Expected:** Returns `Err(...)` (command not found); resolution treated as validation failure; `git reset --hard HEAD~1` runs; result is `Skip` with `repo_clean: true`

### git reset --hard failure after validation failure

1. Validation command exits non-zero
2. `git reset --hard HEAD~1` fails (e.g., repo in unexpected state)
3. **Expected:** `resolve_conflict()` returns `ConflictResolutionResult { action: Abort, audit: None, repo_clean: false }`; merge runner surfaces this as `MergeSessionStatus::Aborted`; tracing::warn! emitted

### merge_report.json persistence failure

1. `.assay/orchestrator/<run_id>/` directory is read-only
2. `orchestrate_run` completes merge
3. **Expected:** `tracing::warn!` emitted with run_id and error; MCP response still returns the in-memory merge report; `orchestrate_status` for this run returns `merge_report: null` (file not present)

## Failure Signals

- `report.resolutions.is_empty()` after a conflict should have been resolved → handler not returning audit, or audit is `None` in result
- `original_contents[i].content` lacks conflict markers → content captured after resolution instead of before
- `validation_passed: None` when validation was configured → `run_validation_command` not being called or result not stored in audit
- `orchestrate_status` response missing `"merge_report"` key → OrchestrateStatusResponse struct changed without updating tests
- `merge_report.json` absent after orchestrate_run with conflicts → `persist_merge_report()` silently failed; check tracing::warn! logs with run_id

## Requirements Proved By This UAT

- R028 (Post-resolution validation) — validation command runs after commit; non-zero exit triggers rollback (`git reset --hard HEAD~1`) and returns `Skip`; proven by `run_validation_command_failure` unit test and validation-failure path in `resolve_conflict()`
- R029 (Conflict resolution audit trail) — `ConflictResolution` records original conflict markers, resolved content, resolver stdout; `MergeReport.resolutions` populated and persisted to `merge_report.json`; surfaced via `orchestrate_status` MCP tool; proven by `test_merge_resolutions_audit_trail` integration test

## Not Proven By This UAT

- Real Claude conflict resolution quality — scripted handlers bypass the actual AI prompt and parsing; only UAT with a live `claude` subprocess against a genuine merge conflict proves prompt correctness and resolution quality
- `assay run --conflict-resolution auto` end-to-end on a real project — no live CLI integration test was added in this slice; CLI wiring was updated but only compilation-verified
- Validation command with a real build tool (e.g., `cargo check`) — only `echo ok` and `sh -c 'exit 1'` are tested; real compilation validation behavior under resource contention or partial resolution is not exercised
- `orchestrate_status` response shape consumed by external tooling — any existing callers of `orchestrate_status` that read `response["run_id"]` directly (not `response["status"]["run_id"]`) will break silently

## Notes for Tester

For the live Claude UAT, set up a RunManifest with two sessions that both add a function to the same file (e.g., both add to `src/lib.rs`). Run `assay run --conflict-resolution auto manifest.toml`. After completion, check `.assay/orchestrator/<run_id>/merge_report.json` — it should contain a `resolutions` array with one entry showing the conflict markers in `original_contents` and the AI-resolved content in `resolved_contents`. The `resolver_stdout` field shows what Claude said. Inspect `git log --oneline` to verify the final merge commit has two parents and the working tree is clean.
