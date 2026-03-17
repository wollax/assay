---
id: T03
parent: S02
milestone: M003
provides:
  - Two integration tests proving audit trail (test_merge_resolutions_audit_trail, test_merge_skip_leaves_empty_resolutions)
  - persist_merge_report() atomic helper in server.rs
  - orchestrate_run calls persist_merge_report() after merge (non-fatal on failure)
  - orchestrate_status returns OrchestrateStatusResponse { status, merge_report } wrapper shape
  - orchestrate_status_reads_merge_report_when_present test
key_files:
  - crates/assay-core/tests/orchestrate_integration.rs
  - crates/assay-mcp/src/server.rs
  - crates/assay-mcp/Cargo.toml
key_decisions:
  - OrchestrateStatusResponse serializes merge_report as null (not skip_serializing_if) so the key is always present — callers can distinguish "run has no merge report" (null) from "merge report present" (object) without checking key existence
  - persist_merge_report() is a module-level free function (not impl AssayServer method) since it needs no server state
  - tempfile added to [dependencies] in assay-mcp/Cargo.toml (was only in dev-dependencies)
patterns_established:
  - orchestrate_status response shape is now always { "status": OrchestratorStatus, "merge_report": null | MergeReport }; callers must index via response["status"]["run_id"] not response["run_id"]
  - atomic merge_report.json alongside state.json in .assay/orchestrator/<run_id>/ using NamedTempFile + rename
observability_surfaces:
  - tracing::warn! in orchestrate_run with run_id + error when merge_report.json write fails
  - tracing::warn! in orchestrate_status with run_id + error when merge_report.json exists but fails to parse
  - .assay/orchestrator/<run_id>/merge_report.json on disk for direct JSON inspection without MCP
  - orchestrate_status response includes merge_report.resolutions[] with session_name, original_contents (with markers), resolved_contents (clean), resolver_stdout, validation_passed
duration: ~45min
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T03: Integration Tests, MergeReport Persistence, and orchestrate_status Extension

**Added two integration tests proving audit trail end-to-end, atomic merge_report.json persistence, and extended orchestrate_status to return `{ status, merge_report }` wrapper.**

## What Happened

**Integration tests (Steps 1–2):** Added `test_merge_resolutions_audit_trail` and `test_merge_skip_leaves_empty_resolutions` to `orchestrate_integration.rs`. The audit trail test sets up conflicting branches (session-a and session-b both modify shared.rs), uses a scripted handler that reads original content, strips markers, writes resolution, and returns a full `ConflictResolution` audit record. Asserts `report.resolutions.len() == 1` with correct session_name, conflict markers in original_contents, clean resolved_contents, and `resolver_stdout == "scripted"`. The skip test creates three non-conflicting sessions and confirms `report.resolutions.is_empty()` and `sessions_merged == 3`.

Used `assay_core::orchestrate::ordering::CompletedSession` directly (not via `extract_completed_sessions`) to control the test data precisely. Added `chrono` datetime construction for `completed_at` fields since `CompletedSession.completed_at` is `DateTime<Utc>`.

**MergeReport persistence (Steps 3–4):** Added `persist_merge_report(run_dir, report)` free function using `tempfile::NamedTempFile` + rename pattern matching `persist_state()` in executor.rs. Added `tempfile` to `[dependencies]` in `assay-mcp/Cargo.toml` (it was only in `[dev-dependencies]`). Called from inside `spawn_blocking` in `orchestrate_run` after `merge_completed_sessions()` returns — logs `tracing::warn!` on failure but doesn't abort the operation.

**orchestrate_status extension (Step 5):** Defined local `OrchestrateStatusResponse { status, merge_report }` struct with plain `Option<MergeReport>` (no `skip_serializing_if`). After parsing `state.json`, reads `merge_report.json` from the same directory — `NotFound` → `None`, parse error → `tracing::warn!` + `None`. Serializes the wrapper instead of raw status.

**Test updates (Steps 6–7):** Updated `orchestrate_status_reads_valid_state` to parse response as `serde_json::Value` and assert `value["status"]["run_id"]`. Added `orchestrate_status_reads_merge_report_when_present` with both `state.json` and a minimal `merge_report.json`. Also updated `mcp_handlers.rs::orchestrate_status_reads_persisted_state_with_sessions` (integration test) to use `response_json["status"]["run_id"]` path.

## Verification

```
cargo test -p assay-core --features orchestrate --test orchestrate_integration
# 5 passed: three_session_dag_execute_merge_end_to_end, failure_propagation_a_fails_b_skipped_c_succeeds,
#           status_persistence_round_trip, test_merge_resolutions_audit_trail,
#           test_merge_skip_leaves_empty_resolutions

cargo test -p assay-mcp orchestrate_status
# 8 passed: all unit tests (orchestrate_status_reads_valid_state, orchestrate_status_reads_merge_report_when_present,
#           orchestrate_status_missing_run_id, orchestrate_status_tool_in_router, params tests) +
#           integration tests (orchestrate_status_reads_persisted_state_with_sessions,
#                              orchestrate_status_missing_run_id_returns_domain_error)

just ready  # fmt ✓, lint ✓, test ✓, deny ✓
```

## Diagnostics

- **On-disk inspection:** `.assay/orchestrator/<run_id>/merge_report.json` — directly inspectable JSON with `resolutions[]` array
- **MCP surface:** `orchestrate_status` returns `{ "status": {...}, "merge_report": null | { "resolutions": [...] } }`
- **Persistence failure:** `tracing::warn! run_id=... error=...` in `orchestrate_run` if fsync/rename fails; merge report still returned in-memory in the MCP response
- **Distinguish no-conflicts from persistence failure:** check on-disk file; if absent after a conflict-resolving run, persistence failed; if present and `resolutions: []`, run had no conflicts

## Deviations

- `OrchestrateStatusResponse.merge_report` uses plain `Option<>` (no `skip_serializing_if`) so `null` is always serialized. The task plan mentioned `skip_serializing_if` but the test in Step 6 requires `result.get("merge_report").is_some()` — keeping the key present (as null) satisfies both the task plan intent and the test
- `mcp_handlers.rs::orchestrate_status_reads_persisted_state_with_sessions` also needed updating for the new shape (not called out in the task plan)

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/tests/orchestrate_integration.rs` — added imports, `create_branch_modifying_file` helper, `test_merge_resolutions_audit_trail`, `test_merge_skip_leaves_empty_resolutions`
- `crates/assay-mcp/src/server.rs` — `persist_merge_report()` helper; `orchestrate_run` calls it; `orchestrate_status` returns `OrchestrateStatusResponse`; updated `orchestrate_status_reads_valid_state`; added `orchestrate_status_reads_merge_report_when_present`
- `crates/assay-mcp/Cargo.toml` — added `tempfile.workspace = true` to `[dependencies]`
- `crates/assay-mcp/tests/mcp_handlers.rs` — updated `orchestrate_status_reads_persisted_state_with_sessions` for new response shape
