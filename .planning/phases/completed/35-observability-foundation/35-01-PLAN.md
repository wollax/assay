---
phase: 35
plan: 1
wave: 1
depends_on: []
files_modified:
  - crates/assay-core/src/gate/session.rs
  - crates/assay-mcp/src/server.rs
  - crates/assay-mcp/tests/mcp_handlers.rs
autonomous: true
source_issue: .planning/issues/open/2026-03-10-history-save-failure-not-surfaced.md
must_haves:
  truths:
    - "gate_run response includes warnings array when history save fails"
    - "gate_finalize response includes warnings array when history save fails instead of returning a hard error"
    - "gate_report response includes warnings field (empty, omitted via skip_serializing_if)"
    - "gate_finalize uses a proper response struct instead of inline serde_json::json!()"
    - "finalize_session is split into build_finalized_record (pure) and save (caller responsibility)"
  artifacts:
    - path: "crates/assay-core/src/gate/session.rs"
      provides: "build_finalized_record() — pure function that builds GateRunRecord without I/O"
    - path: "crates/assay-mcp/src/server.rs"
      provides: "warnings: Vec<String> on GateRunResponse, GateReportResponse, and new GateFinalizeResponse struct"
  key_links:
    - from: "gate_finalize handler"
      to: "build_finalized_record()"
      via: "calls build_finalized_record then saves separately, collecting save errors as warnings"
    - from: "gate_run handler (command-only path)"
      to: "warnings field"
      via: "save_run failure becomes warning string instead of silent tracing::warn"
---

<objective>
Add a `warnings: Vec<String>` field to all gate MCP response structs (`GateRunResponse`, `GateReportResponse`, and a new `GateFinalizeResponse`) to surface degraded-but-succeeded operations. Refactor `finalize_session` in session.rs to split record-building from history save so the handler can collect save failures as warnings instead of hard errors. This closes the history-save-failure-not-surfaced issue.
</objective>

<context>
@crates/assay-mcp/src/server.rs (lines 260-307 for response structs, lines 646-668 for gate_run command-only save, lines 740-782 for gate_finalize handler)
@crates/assay-core/src/gate/session.rs (lines 103-232 for finalize_session)
@crates/assay-mcp/tests/mcp_handlers.rs
</context>

<tasks>

<task type="auto">
<name>Task 1: Split finalize_session and add warnings to response structs</name>
<files>crates/assay-core/src/gate/session.rs, crates/assay-mcp/src/server.rs</files>
<action>
**session.rs changes:**

1. Rename `finalize_session` to `build_finalized_record`. Remove the `history::save()` call and the `assay_dir` and `max_history` parameters. Keep `working_dir` since it's used in the record. The function should be pure — build the `GateRunRecord` and return it without I/O. New signature:
   ```rust
   pub fn build_finalized_record(
       session: &AgentSession,
       working_dir: Option<&str>,
   ) -> GateRunRecord
   ```
   Note: This function currently returns `Result<GateRunRecord>` only because `history::save` can fail. Without the save, it's infallible — change return type to plain `GateRunRecord`.

2. Keep a `finalize_session` wrapper that calls `build_finalized_record` then `history::save`, for backward compatibility with existing tests. It keeps the current signature and behavior:
   ```rust
   pub fn finalize_session(
       session: &AgentSession,
       assay_dir: &Path,
       working_dir: Option<&str>,
       max_history: Option<usize>,
   ) -> Result<GateRunRecord> {
       let record = build_finalized_record(session, working_dir);
       history::save(assay_dir, &record, max_history)?;
       Ok(record)
   }
   ```

**server.rs changes:**

3. Add `warnings` field to `GateRunResponse`:
   ```rust
   /// Warnings about degraded operations (e.g., history save failure).
   /// Omitted from JSON when empty.
   #[serde(default, skip_serializing_if = "Vec::is_empty")]
   warnings: Vec<String>,
   ```

4. Add `warnings` field to `GateReportResponse` (same pattern as above).

5. Create a `GateFinalizeResponse` struct to replace the inline `serde_json::json!()` in the gate_finalize handler:
   ```rust
   #[derive(Serialize)]
   struct GateFinalizeResponse {
       run_id: String,
       spec_name: String,
       passed: usize,
       failed: usize,
       skipped: usize,
       required_failed: usize,
       advisory_failed: usize,
       persisted: bool,
       #[serde(default, skip_serializing_if = "Vec::is_empty")]
       warnings: Vec<String>,
   }
   ```

6. In the `gate_run` handler (command-only path, around line 646-663), collect save failure as a warning instead of only logging:
   ```rust
   let mut warnings = Vec::new();
   if let Err(e) = assay_core::history::save_run(...) {
       let msg = format!("history save failed: {e}");
       tracing::warn!(spec_name = %spec_name_for_log, "{msg}");
       warnings.push(msg);
   }
   response.warnings = warnings;
   ```
   Move the `response.warnings = warnings;` before serialization but after the save attempt.

7. In the `gate_finalize` handler (around line 757-781), switch to `build_finalized_record` + explicit save:
   ```rust
   let record = assay_core::gate::session::build_finalized_record(
       &session,
       Some(&working_dir.to_string_lossy()),
   );

   let mut warnings = Vec::new();
   match assay_core::history::save(&assay_dir, &record, max_history) {
       Ok(_) => {}
       Err(e) => {
           let msg = format!("history save failed: {e}");
           tracing::warn!(session_id = %record.run_id, "{msg}");
           warnings.push(msg);
       }
   }

   let response = GateFinalizeResponse {
       run_id: record.run_id,
       spec_name: record.summary.spec_name,
       passed: record.summary.passed,
       failed: record.summary.failed,
       skipped: record.summary.skipped,
       required_failed: record.summary.enforcement.required_failed,
       advisory_failed: record.summary.enforcement.advisory_failed,
       persisted: warnings.is_empty(),
       warnings,
   };
   ```
   The `persisted` field is `true` only when save succeeded (no warnings about save failure).

8. Initialize `response.warnings` to empty vec in the `gate_run` handler where GateRunResponse is constructed (the existing construction site). Add `warnings: Vec::new()` to the struct literal.
</action>
<verify>
`cargo test -p assay-core --lib gate::session` and `cargo check -p assay-mcp`
</verify>
<done>
- `build_finalized_record` exists in session.rs as a pure function returning `GateRunRecord`
- `finalize_session` still exists as a convenience wrapper
- All three gate response structs have `warnings: Vec<String>` with `skip_serializing_if`
- `gate_finalize` handler uses `GateFinalizeResponse` struct, not inline JSON
- `gate_run` command-only path collects save failures as warnings
- `gate_finalize` handler collects save failures as warnings instead of hard errors
</done>
</task>

<task type="auto">
<name>Task 2: Add integration tests for warnings field</name>
<files>crates/assay-mcp/tests/mcp_handlers.rs</files>
<action>
Add tests to `mcp_handlers.rs` that verify:

1. **gate_run command-only success has no warnings field in JSON** — Run gate_run on a command-only spec (e.g., `echo hello`). Parse the response JSON. Assert no `warnings` key is present (skip_serializing_if should omit it). Verify the existing fields (`passed`, `failed`, etc.) are still correct.

2. **gate_finalize success has no warnings field** — Run the full session lifecycle (gate_run → gate_report → gate_finalize). Parse the finalize response JSON. Assert `persisted` is `true` and no `warnings` key is present.

3. **gate_finalize response has correct structure** — Verify the gate_finalize response includes all expected fields: `run_id`, `spec_name`, `passed`, `failed`, `skipped`, `required_failed`, `advisory_failed`, `persisted`. This validates the migration from inline `json!()` to `GateFinalizeResponse` struct.

Follow the existing test patterns in the file — use `create_project()`, `create_spec()`, `extract_text()`, `Parameters()` wrapper, and `serde_json::from_str::<serde_json::Value>()` for response parsing.
</action>
<verify>
`cargo test -p assay-mcp --test mcp_handlers`
</verify>
<done>
- Tests verify warnings field is absent when no warnings occur
- Tests verify gate_finalize response structure matches the new struct
- All existing tests still pass
</done>
</task>

</tasks>

<verification>
- `just ready` passes (fmt, lint, test, deny)
- gate_run response JSON omits `warnings` when empty
- gate_finalize returns structured response with `persisted` field
- gate_finalize save failure produces warning instead of hard error
</verification>

<success_criteria>
1. `build_finalized_record` is a pure function in session.rs
2. All three gate response structs have `warnings: Vec<String>` with skip_serializing_if
3. `gate_finalize` uses a dedicated response struct
4. Save failures in gate_run and gate_finalize are collected as warnings
5. Integration tests pass validating response structure
</success_criteria>
