# Phase 35: Observability Foundation - Research

**Researched:** 2026-03-10
**Confidence:** HIGH (all findings from direct codebase investigation)

## Standard Stack

No new external dependencies required. This phase uses only existing crate capabilities:
- `serde` with `skip_serializing_if` for conditional field presence
- `schemars::JsonSchema` for schema generation
- Existing `assay_types` pattern for shared types
- Existing `assay_core::history` module for load/list operations

## Architecture Patterns

### Current MCP Response Pattern

All MCP tool handlers in `crates/assay-mcp/src/server.rs` follow the same pattern:
1. Each handler has a dedicated response struct (e.g., `GateRunResponse`, `GateReportResponse`, `GateHistoryListResponse`)
2. Response structs derive `Serialize` only (not `Deserialize` -- they're output-only)
3. Handlers serialize response to JSON string, wrap in `Content::text()`, return `CallToolResult::success(vec![Content::text(json)])`
4. Domain errors use `CallToolResult::error()` via the `domain_error()` helper
5. Response structs are **private** to `server.rs` -- not shared types

### Response Structs That Need `warnings` Field

Mutating MCP tools (tools that cause side effects):
- **`gate_run`** -- Evaluates criteria AND saves history. Uses `GateRunResponse` struct (line 261).
- **`gate_report`** -- Records an agent evaluation to an in-memory session. Uses `GateReportResponse` struct (line 294).
- **`gate_finalize`** -- Finalizes session AND saves history. Uses inline `serde_json::json!()` (line 767).
- **`worktree_create`** -- Creates git worktree. Serializes `WorktreeInfo` directly from assay-core.
- **`worktree_cleanup`** -- Removes git worktree. Uses inline `serde_json::json!()` (line 1048).

Non-mutating tools (read-only, do NOT need warnings):
- `spec_list`, `spec_get`, `gate_history`, `context_diagnose`, `estimate_tokens`, `worktree_list`, `worktree_status`

### `gate_run` History Save -- The Primary Warning Site

In `gate_run` (line 646-663), for command-only specs:
```rust
if let Err(e) = assay_core::history::save_run(
    &assay_dir, summary,
    Some(working_dir.to_string_lossy().to_string()),
    max_history,
) {
    tracing::warn!(
        spec_name = %spec_name_for_log,
        "failed to save command-only gate run history: {e}"
    );
}
```
**Key finding:** The error is logged via `tracing::warn!` but never surfaces to the MCP caller. The response is already formatted from the summary before the save attempt. This is the exact issue described in DEBT-02.

For session-based specs (line 615-644), the timeout auto-finalize task also silently logs history save failures via `tracing::error!`.

### `gate_finalize` History Save

In `gate_finalize` (line 757-765), the handler calls `assay_core::gate::session::finalize_session()` which internally calls `history::save()`. If save fails, the error propagates as `Err(e)` and is returned as a domain error. **This path already surfaces the error** -- but as a hard error, not a warning. The gate evaluation succeeded but the save failed, so it should arguably be a warning (degraded success) rather than an error.

### Current `gate_history` Implementation

**Handler:** `gate_history` in `server.rs` (line 788-853)

**Parameters (current):**
- `name: String` -- spec name (required)
- `run_id: Option<String>` -- specific run ID for detail mode
- `limit: Option<usize>` -- max runs in list mode (default: 10, NO max cap currently)

**Two modes:**
1. **Detail mode** (run_id provided): Loads and returns full `GateRunRecord` via `assay_core::history::load()`
2. **List mode** (no run_id): Calls `assay_core::history::list()` which returns ALL run IDs sorted oldest-first, then takes last N (reversed for newest-first), loads each as `GateHistoryEntry`

**Current list mode logic:**
```rust
let all_ids = assay_core::history::list(&assay_dir, &params.0.name)?;
let total_runs = all_ids.len();
let limit = params.0.limit.unwrap_or(10);
let selected_ids: Vec<&String> = all_ids.iter().rev().take(limit).collect();
```

**Missing features:**
- No `outcome` filter parameter
- No max cap on `limit`
- `limit` already defaults to 10 (matching spec requirement)

**`GateHistoryEntry` struct (line 321-339):**
- `run_id`, `timestamp`, `passed`, `failed`, `skipped`, `required_failed`, `advisory_failed`, `blocked`
- `blocked` is derived: `record.summary.enforcement.required_failed > 0`

### `GateRunRecord` Structure

Defined in `crates/assay-types/src/gate_run.rs`:
- `run_id: String`
- `assay_version: String`
- `timestamp: DateTime<Utc>`
- `working_dir: Option<String>`
- `summary: GateRunSummary` which contains:
  - `spec_name`, `results: Vec<CriterionResult>`, `passed`, `failed`, `skipped`, `total_duration_ms`
  - `enforcement: EnforcementSummary` with `required_passed`, `required_failed`, `advisory_passed`, `advisory_failed`

**Outcome classification:** A run is "failed" when `summary.enforcement.required_failed > 0`. This matches the `blocked` field derivation in `GateHistoryEntry`.

### History Module API

`crates/assay-core/src/history/mod.rs` provides:
- `save_run(assay_dir, summary, working_dir, max_history) -> Result<SaveResult>`
- `save(assay_dir, record, max_history) -> Result<SaveResult>`
- `load(assay_dir, spec_name, run_id) -> Result<GateRunRecord>`
- `list(assay_dir, spec_name) -> Result<Vec<String>>` -- returns sorted run IDs (oldest first)

Outcome filtering will need to happen at the handler level (load records, check `required_failed > 0`, filter). The `list()` function returns only IDs, so records must be loaded to check outcome.

## Don't Hand-Roll

- **Serde conditional serialization** -- Use `#[serde(default, skip_serializing_if = "Vec::is_empty")]` for the warnings field. This is the established pattern throughout the codebase.
- **JSON Schema generation** -- Response structs in `server.rs` do NOT derive `JsonSchema`. They derive only `Serialize`. Don't add `JsonSchema` derives to these structs -- they're not in the schema registry.
- **Run ID generation** -- Use existing `history::generate_run_id()`.

## Common Pitfalls

### 1. Warnings field on `gate_finalize` response
`gate_finalize` currently uses `serde_json::json!()` macro for its response, not a struct. The warnings field needs to be added to this inline JSON, or the response should be refactored to use a struct (preferred for consistency).

### 2. `gate_finalize` error vs warning for save failure
Currently, `finalize_session()` in `session.rs` calls `history::save()` and propagates errors. If save fails, the finalization result (which succeeded) is lost. The save should be attempted, and failure should become a warning on a successful response, not prevent returning the gate results.

### 3. Outcome filtering loads all records
`history::list()` returns only IDs. To filter by outcome, you must load each record and check `required_failed`. For large histories, this is O(n) disk reads. Since `limit` caps at 50 and `max_history` config can cap total files, this is acceptable. But the implementation should load-and-filter incrementally (newest-first) rather than loading ALL records then filtering.

### 4. `limit` parameter already exists
`GateHistoryParams` already has `limit: Option<usize>`. The only changes needed are: (a) cap at 50, (b) add `outcome` parameter. Don't accidentally create a second `limit` field.

### 5. Session timeout auto-finalize warnings are unrecoverable
The spawned timeout task (line 615-644) runs asynchronously. Warnings from this path cannot be surfaced to any MCP response because there's no active request. This is acceptable -- `tracing::error!` is the right approach for async background failures. Do NOT try to collect warnings from this path.

### 6. `worktree_create` and `worktree_cleanup` response shapes
`worktree_create` serializes a `WorktreeInfo` struct from `assay_types`. Adding a `warnings` field requires either wrapping it in a response envelope in `server.rs`, or not adding warnings to worktree tools in this phase. The phase success criteria only mention "mutating MCP tool responses" -- consider scoping to gate tools initially since those are the tools with known failure-silencing issues.

## Code Examples

### Warnings field on response structs
```rust
/// Aggregate gate run response returned by the `gate_run` tool.
#[derive(Serialize)]
struct GateRunResponse {
    // ... existing fields ...
    /// Warnings about degraded operations (e.g., history save failure).
    /// Omitted from JSON when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}
```

### Collecting warnings in `gate_run` handler
```rust
let mut warnings = Vec::new();

// After gate evaluation succeeds, save history
if let Err(e) = assay_core::history::save_run(&assay_dir, summary, ...) {
    warnings.push(format!("history save failed: {e}"));
}

response.warnings = warnings;
```

### Outcome filter parameter
```rust
#[derive(Deserialize, JsonSchema)]
pub struct GateHistoryParams {
    pub name: String,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    /// Filter by outcome: "passed", "failed", or "any" (default: "any").
    #[serde(default)]
    pub outcome: Option<String>,
}
```

### Outcome filtering logic
```rust
let limit = params.0.limit.unwrap_or(10).min(50);
let outcome_filter = params.0.outcome.as_deref().unwrap_or("any");

let mut runs = Vec::with_capacity(limit);
// Iterate newest-first, load and filter until we have `limit` matches
for id in all_ids.iter().rev() {
    if runs.len() >= limit { break; }
    match assay_core::history::load(&assay_dir, &params.0.name, id) {
        Ok(record) => {
            let is_failed = record.summary.enforcement.required_failed > 0;
            let matches = match outcome_filter {
                "passed" => !is_failed,
                "failed" => is_failed,
                _ => true, // "any" or unrecognized
            };
            if matches {
                runs.push(make_history_entry(&record));
            }
        }
        Err(e) => {
            tracing::warn!(run_id = %id, "skipping unreadable history entry: {e}");
        }
    }
}
```

### Refactoring `gate_finalize` to use warnings
```rust
// In gate_finalize handler, separate save from finalization:
let (record, save_warning) = {
    let record = assay_core::gate::session::build_finalized_record(&session, ...);
    let warning = match assay_core::history::save(&assay_dir, &record, max_history) {
        Ok(_) => None,
        Err(e) => Some(format!("history save failed: {e}")),
    };
    (record, warning)
};

let mut warnings = Vec::new();
if let Some(w) = save_warning {
    warnings.push(w);
}
// Include warnings in response JSON
```

## Key Decisions (Researcher Recommendations)

1. **Plain strings for warnings** -- Structured warning objects add complexity without value at this stage. Plain strings are sufficient for surfacing degraded operations and can be upgraded later.

2. **`skip_serializing_if = "Vec::is_empty"`** -- Absent when no warnings (consistent with codebase convention for optional arrays).

3. **Scope to gate tools initially** -- Add `warnings` to `gate_run`, `gate_report`, `gate_finalize`. Worktree tools don't have known silent-failure patterns, so defer their warnings to a future phase.

4. **`outcome` as string enum** -- Use `"passed"`, `"failed"`, `"any"` as string values. Default `"any"`. Unrecognized values should be treated as `"any"` (lenient parsing for forward compatibility).

5. **`limit` capped at 50** -- `params.0.limit.unwrap_or(10).min(50)`.

6. **`total_runs` stays as total, not filtered count** -- The response should show `total_runs` (total records on disk) AND the number actually returned after filtering. This lets callers know "there are 100 runs total, 5 of which are failed, here are the 5 you asked for."

7. **Refactor `finalize_session` in session.rs** -- Split into `build_finalized_record()` (pure, no I/O) and let the handler do the save. This makes warnings collection clean and testable.

## File Inventory

| File | Role | Changes Needed |
|------|------|----------------|
| `crates/assay-mcp/src/server.rs` | MCP handler implementations | Add `warnings` to response structs, refactor `gate_finalize` response to struct, add `outcome` param to `GateHistoryParams`, cap `limit` at 50, collect warnings from save failures |
| `crates/assay-core/src/gate/session.rs` | Session finalization logic | Split `finalize_session` into record-building (pure) and save (I/O) so handler can collect warnings |
| `crates/assay-core/src/history/mod.rs` | History persistence | No changes needed -- existing API is sufficient |
| `crates/assay-types/src/gate_run.rs` | GateRunRecord type | No changes needed |
| `crates/assay-mcp/tests/mcp_handlers.rs` | Integration tests | Add tests for warnings field, outcome filtering, limit capping |

---

*Phase: 35-observability-foundation*
*Research completed: 2026-03-10*
