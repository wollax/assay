---
estimated_steps: 7
estimated_files: 2
---

# T03: Integration Tests, MergeReport Persistence, and orchestrate_status Extension

**Slice:** S02 — Audit Trail, Validation & End-to-End
**Milestone:** M003

## Description

Completes the slice. Adds two integration tests to `orchestrate_integration.rs` that prove the audit trail: one where a scripted handler returns a full `ConflictResolution` record and `MergeReport.resolutions` is populated, one where the default handler skips and `resolutions` is empty. Then makes the MergeReport durable: `orchestrate_run` persists `merge_report.json` atomically alongside `state.json`, and `orchestrate_status` reads it when present and wraps the response as `{ status: OrchestratorStatus, merge_report: Option<MergeReport> }`.

The `orchestrate_run` persistence goes inside the `spawn_blocking` closure where `orch_result.run_id` and `pipeline_config.project_root` are available. The run directory was already created by the executor's `persist_state()`, so no `create_dir_all` is needed. The `orchestrate_status` response shape change is additive — existing consumers that only read `run_id`, `phase`, `sessions` still work because those fields are nested under `status`.

## Steps

1. **Add `test_merge_resolutions_audit_trail` to `orchestrate_integration.rs`**:
   - Set up conflicting branches (same as `test_merge_runner_conflict_resolution_with_live_tree` in merge_runner.rs): main has `shared.rs`, branch A has "version A", branch B has "version B"
   - Use a scripted handler that:
     - Reads `shared.rs` before writing (original content)
     - Strips conflict markers
     - Writes resolved content
     - Stages and commits
     - Gets SHA
     - Returns `ConflictResolutionResult { action: Resolved(sha), audit: Some(ConflictResolution { session_name: name.to_string(), conflicting_files: files.to_vec(), original_contents: vec![ConflictFileContent { path: "shared.rs".into(), content: original }], resolved_contents: vec![ConflictFileContent { path: "shared.rs".into(), content: resolved }], resolver_stdout: "scripted".to_string(), validation_passed: None }), repo_clean: false }`
   - Assert: `report.resolutions.len() == 1`; `report.resolutions[0].session_name == "session-b"`; `report.resolutions[0].conflicting_files == ["shared.rs"]`; `original_contents[0].content` contains conflict markers (`<<<<<<<`); `resolved_contents[0].content` does NOT contain markers; `resolver_stdout == "scripted"`
   - Assert `report.sessions_merged == 2`

2. **Add `test_merge_skip_leaves_empty_resolutions` to `orchestrate_integration.rs`**:
   - Set up three sessions with no-conflict file changes (each writes its own unique file)
   - Use `default_conflict_handler()` with `conflict_resolution_enabled: false`
   - Assert `report.resolutions.is_empty()`
   - Assert `report.sessions_merged == 3`

3. **Add `persist_merge_report()` private function to `server.rs`**:
   - Copy the atomic-write pattern from `persist_state()` in `executor.rs`:
     ```rust
     fn persist_merge_report(run_dir: &Path, report: &assay_types::MergeReport) -> std::io::Result<()> {
         let final_path = run_dir.join("merge_report.json");
         let json = serde_json::to_string_pretty(report)
             .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
         let mut tmpfile = tempfile::NamedTempFile::new_in(run_dir)?;
         tmpfile.write_all(json.as_bytes())?;
         tmpfile.as_file().sync_all()?;
         tmpfile.persist(&final_path)
             .map_err(|e| e.error)?;
         Ok(())
     }
     ```
   - Note: `server.rs` already uses `tempfile` (check Cargo.toml — if not, add it); look for existing `use tempfile::NamedTempFile` in server.rs or its deps

4. **Call `persist_merge_report()` inside `spawn_blocking` in `orchestrate_run`**:
   - After `merge_completed_sessions()` returns `merge_report`, before the `Ok(...)` return:
     ```rust
     let run_dir = pipeline_config.project_root
         .join(".assay")
         .join("orchestrator")
         .join(&orch_result.run_id);
     if let Err(e) = persist_merge_report(&run_dir, &merge_report) {
         tracing::warn!(run_id = %orch_result.run_id, error = %e, "failed to persist merge report");
     }
     ```
   - Log warning on failure but don't fail the whole operation — the merge succeeded; the report is still returned in the MCP response even if persistence fails

5. **Extend `orchestrate_status` with `OrchestrateStatusResponse`**:
   - Define local struct near the handler:
     ```rust
     #[derive(Serialize)]
     struct OrchestrateStatusResponse {
         status: assay_types::OrchestratorStatus,
         #[serde(skip_serializing_if = "Option::is_none")]
         merge_report: Option<assay_types::MergeReport>,
     }
     ```
   - After parsing `state.json`, construct the merge_report_path: same dir + `merge_report.json`
   - Try to read and parse it; on `NotFound`: `None`; on read/parse error: log `tracing::warn!` and use `None` (don't fail the status query for a missing or corrupt merge report)
   - Serialize `OrchestrateStatusResponse { status, merge_report }` instead of just `status`

6. **Update `orchestrate_status_reads_valid_state` test**:
   - Parse the result text as `serde_json::Value`; assert `result["status"]["run_id"] == run_id`; assert `result.get("merge_report")` is present (it will be `null` since no merge_report.json was written — that's correct)

7. **Add `orchestrate_status_reads_merge_report_when_present` test**:
   - Write both `state.json` (valid `OrchestratorStatus`) and `merge_report.json` (minimal `MergeReport` with empty `resolutions`, `plan`, `results`, `duration_secs: 0.0`, all counts 0)
   - Call `orchestrate_status`; assert success; parse result text as `serde_json::Value`; assert `result["status"]["run_id"] == run_id`; assert `result["merge_report"]` is an object (not null); assert `result["merge_report"]["sessions_merged"] == 0`
   - Run `just ready`

## Must-Haves

- [ ] `test_merge_resolutions_audit_trail` passes with `MergeReport.resolutions` containing one entry with correct fields (session_name, original content with markers, resolved content without markers)
- [ ] `test_merge_skip_leaves_empty_resolutions` passes confirming empty resolutions on clean merge
- [ ] `persist_merge_report()` uses atomic NamedTempFile + rename pattern (no partial writes)
- [ ] Persistence failure is a warning, not a hard error (merge result still returned)
- [ ] `orchestrate_status` returns `{ "status": {...}, "merge_report": null_or_object }` wrapper shape
- [ ] `orchestrate_status_reads_merge_report_when_present` passes
- [ ] All existing MCP orchestrate tests still pass
- [ ] `just ready` green

## Verification

- `cargo test -p assay-core orchestrate_integration` — `test_merge_resolutions_audit_trail` and `test_merge_skip_leaves_empty_resolutions` pass; existing 3 tests still pass
- `cargo test -p assay-mcp orchestrate_status` — `orchestrate_status_reads_valid_state` (updated), `orchestrate_status_reads_merge_report_when_present` (new), `orchestrate_status_missing_run_id` (unchanged) all pass
- `just ready` — fully green; all 5 test suites clean

## Observability Impact

- Signals added/changed: `tracing::warn!` in `orchestrate_run` with `run_id` and `error` when persistence fails; `tracing::warn!` in `orchestrate_status` with `run_id` and `error` when merge report file exists but can't be parsed
- How a future agent inspects this: `.assay/orchestrator/<run_id>/merge_report.json` on disk for direct inspection without MCP; `orchestrate_status` response includes `merge_report.resolutions` array with `session_name`, `original_contents`, `resolved_contents`, `resolver_stdout`, `validation_passed` for each resolved conflict
- Failure state exposed: `merge_report: null` in orchestrate_status output indicates either no conflicts were resolved or persistence failed; distinguish with on-disk file presence check

## Inputs

- T01 outputs: `ConflictResolution`, `ConflictFileContent`, `MergeReport.resolutions` field defined
- T02 outputs: `ConflictResolutionResult` returned by handlers, `MergeReport.resolutions` populated by merge runner, `default_conflict_handler()` updated
- `crates/assay-core/tests/orchestrate_integration.rs` — existing test helpers (`setup_git_repo`, `make_pipeline_config`, `make_manifest`, `mock_success_runner`) to reuse
- `crates/assay-core/src/orchestrate/executor.rs` — `persist_state()` as pattern for atomic write
- `crates/assay-mcp/src/server.rs` — `orchestrate_status` handler; `orchestrate_run` `spawn_blocking` closure

## Expected Output

- `crates/assay-core/tests/orchestrate_integration.rs` — 2 new integration tests; 5 total integration tests pass
- `crates/assay-mcp/src/server.rs` — `persist_merge_report()` helper; `orchestrate_run` calls it; `orchestrate_status` returns `OrchestrateStatusResponse` wrapper; 2 updated/new tests
