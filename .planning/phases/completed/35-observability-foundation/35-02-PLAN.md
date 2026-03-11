---
phase: 35
plan: 2
wave: 2
depends_on: [1]
files_modified:
  - crates/assay-mcp/src/server.rs
  - crates/assay-mcp/tests/mcp_handlers.rs
autonomous: true
source_issue: null
must_haves:
  truths:
    - "gate_history accepts outcome parameter with values passed, failed, or any"
    - "gate_history with outcome=failed returns only runs where required_failed > 0"
    - "gate_history with outcome=passed returns only runs where required_failed == 0"
    - "gate_history limit is capped at 50 regardless of requested value"
    - "gate_history limit defaults to 10 when omitted"
  artifacts:
    - path: "crates/assay-mcp/src/server.rs"
      provides: "outcome parameter on GateHistoryParams, limit capped at 50, outcome-filtered list mode"
  key_links:
    - from: "GateHistoryParams.outcome"
      to: "gate_history list mode loop"
      via: "filter records by required_failed > 0 during newest-first iteration"
---

<objective>
Add `outcome` filter parameter to `gate_history` (passed/failed/any, default any) and cap `limit` at 50. The outcome filter loads records newest-first and checks `required_failed > 0` to determine failed vs passed, collecting up to `limit` matching entries.
</objective>

<context>
@crates/assay-mcp/src/server.rs (lines 121-141 for GateHistoryParams, lines 788-853 for gate_history handler, lines 309-339 for GateHistoryListResponse and GateHistoryEntry)
@crates/assay-mcp/tests/mcp_handlers.rs
</context>

<tasks>

<task type="auto">
<name>Task 1: Add outcome filter and limit cap to gate_history</name>
<files>crates/assay-mcp/src/server.rs</files>
<action>
1. Add `outcome` parameter to `GateHistoryParams`:
   ```rust
   /// Filter by outcome: "passed", "failed", or "any" (default: "any").
   /// A run is "failed" when any required criterion failed (required_failed > 0).
   #[schemars(
       description = "Filter runs by outcome: 'passed' (no required failures), 'failed' (has required failures), or 'any' (default: 'any')"
   )]
   #[serde(default)]
   pub outcome: Option<String>,
   ```

2. Cap `limit` at 50 on line 818:
   Change `let limit = params.0.limit.unwrap_or(10);` to `let limit = params.0.limit.unwrap_or(10).min(50);`

3. Refactor the list mode loop (lines 820-842) to support outcome filtering. Replace the current approach (take last N IDs then load) with a load-and-filter loop that iterates newest-first:
   ```rust
   let limit = params.0.limit.unwrap_or(10).min(50);
   let outcome_filter = params.0.outcome.as_deref().unwrap_or("any");

   let mut runs = Vec::with_capacity(limit);
   for id in all_ids.iter().rev() {
       if runs.len() >= limit {
           break;
       }
       match assay_core::history::load(&assay_dir, &params.0.name, id) {
           Ok(record) => {
               let is_failed = record.summary.enforcement.required_failed > 0;
               let matches = match outcome_filter {
                   "passed" => !is_failed,
                   "failed" => is_failed,
                   _ => true, // "any" or unrecognized
               };
               if matches {
                   runs.push(GateHistoryEntry {
                       run_id: record.run_id,
                       timestamp: record.timestamp.to_rfc3339(),
                       passed: record.summary.passed,
                       failed: record.summary.failed,
                       skipped: record.summary.skipped,
                       required_failed: record.summary.enforcement.required_failed,
                       advisory_failed: record.summary.enforcement.advisory_failed,
                       blocked: is_failed,
                   });
               }
           }
           Err(e) => {
               tracing::warn!(run_id = %id, "skipping unreadable history entry: {e}");
           }
       }
   }
   ```

4. `total_runs` stays as `all_ids.len()` — it reflects total on-disk records, not filtered count.

5. Update the `gate_history` tool description to mention the `outcome` filter parameter.
</action>
<verify>
`cargo check -p assay-mcp`
</verify>
<done>
- `GateHistoryParams` has `outcome: Option<String>` field
- `limit` is capped at 50 via `.min(50)`
- List mode iterates newest-first, loading and filtering by outcome
- Unrecognized outcome values treated as "any"
</done>
</task>

<task type="auto">
<name>Task 2: Add integration tests for outcome filtering and limit cap</name>
<files>crates/assay-mcp/tests/mcp_handlers.rs</files>
<action>
Add tests to `mcp_handlers.rs`:

1. **gate_history outcome=failed filters correctly** — Create a project with a spec that has a command criterion. Run gate_run multiple times with a mix of passing and failing commands (e.g., `echo pass` and `false`). Query gate_history with `outcome: Some("failed".into())`. Assert only the failed runs are returned.

2. **gate_history outcome=passed filters correctly** — Same setup as above. Query with `outcome: Some("passed".into())`. Assert only the passed runs are returned.

3. **gate_history outcome=any returns all** — Query with `outcome: Some("any".into())`. Assert all runs are returned (up to limit).

4. **gate_history limit capped at 50** — Query with `limit: Some(100)`. Parse the response. Assert the returned `runs` array length is at most 50. (With fewer than 50 actual runs, it returns all of them — the point is the limit parameter was accepted without error and the code path that caps at 50 is exercised.)

5. **gate_history default limit is 10** — Create 15 runs. Query with no limit specified. Assert exactly 10 are returned.

Follow existing test patterns. Use `create_project()`, `create_spec()`, `extract_text()`, etc. For creating multiple history entries, call `gate_run` multiple times with different commands.
</action>
<verify>
`cargo test -p assay-mcp --test mcp_handlers`
</verify>
<done>
- Tests verify outcome=failed returns only failed runs
- Tests verify outcome=passed returns only passed runs
- Tests verify limit is capped at 50
- Tests verify default limit is 10
- All existing tests still pass
</done>
</task>

</tasks>

<verification>
- `just ready` passes
- gate_history with outcome=failed returns only runs with required_failed > 0
- gate_history with outcome=passed returns only runs with required_failed == 0
- gate_history with limit > 50 returns at most 50 entries
</verification>

<success_criteria>
1. GateHistoryParams has outcome field with schemars description
2. limit is capped at 50 via .min(50)
3. Outcome filtering loads and filters newest-first
4. Integration tests cover passed, failed, any, limit cap, and default limit
</success_criteria>
