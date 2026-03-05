# Phase 17 Research: MCP Hardening & Agent History

## Standard Stack

| Need | Use | Why |
|------|-----|-----|
| Timeout wrapping | `tokio::time::timeout` | Already in `tokio.workspace = true`; wraps the `spawn_blocking` future |
| Path validation | `std::path::Path::is_dir()` / `exists()` | No external crate needed |
| History querying | `assay_core::history::{list, load}` | Already implemented with full API |
| Response types | `serde + serde_json` + `schemars::JsonSchema` | Matches all existing MCP response patterns |
| Error handling | `CallToolResult::error(vec![Content::text(...)])` | Established domain error pattern |

## Architecture Patterns

**Confidence: HIGH** -- These are directly observed in the codebase.

### 1. MCP Tool Pattern

Every tool in `server.rs` follows this exact structure:
```rust
#[tool(description = "...")]
async fn tool_name(&self, params: Parameters<ParamStruct>) -> Result<CallToolResult, McpError> {
    let cwd = resolve_cwd()?;
    let config = match load_config(&cwd) {
        Ok(c) => c,
        Err(err_result) => return Ok(err_result),
    };
    // ... domain logic ...
    let json = serde_json::to_string(&response)
        .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}
```

Key conventions:
- Parameter struct: `#[derive(Deserialize, JsonSchema)]` with `#[schemars(description = "...")]` on each field
- Response struct: `#[derive(Serialize)]` with `#[serde(skip_serializing_if = "...")]` on optional fields
- Domain errors -> `CallToolResult::error` (agent sees them), protocol errors -> `McpError` (infrastructure)
- All response structs are module-private (not in `assay-types`)

### 2. Blocking Evaluation Pattern

Gate evaluation is **synchronous** and wrapped with `spawn_blocking`:
```rust
let summary = tokio::task::spawn_blocking(move || {
    assay_core::gate::evaluate_all(&spec, &working_dir, None, config_timeout)
}).await
.map_err(|e| McpError::internal_error(format!("gate evaluation panicked: {e}"), None))?;
```

For timeout (MCP-01), wrap **this entire future** with `tokio::time::timeout`:
```rust
let timeout_duration = Duration::from_secs(params.timeout.unwrap_or(300));
let result = tokio::time::timeout(timeout_duration, tokio::task::spawn_blocking(move || {
    // evaluate_all...
})).await;
// match: Ok(Ok(summary)) / Ok(Err(panic)) / Err(elapsed)
```

### 3. History Module API

`assay_core::history` exposes three functions (all synchronous):
- `list(assay_dir, spec_name) -> Result<Vec<String>>` -- returns run IDs sorted chronologically
- `load(assay_dir, spec_name, run_id) -> Result<GateRunRecord>` -- loads a single record
- `save(assay_dir, record, max_history) -> Result<SaveResult>` -- saves with optional pruning

Records live at `.assay/results/<spec-name>/<run-id>.json`.

### 4. Scan + Error Handling

`assay_core::spec::scan()` returns `ScanResult { entries, specs, errors }`. The `errors` field already collects parse/validation failures. **But `spec_list` in server.rs currently ignores `scan_result.errors` entirely** -- it only maps `scan_result.entries`.

## Existing Code Inventory

### What exists (no changes needed for basic function):

| Component | File | Status |
|-----------|------|--------|
| `history::list()` | `crates/assay-core/src/history/mod.rs` | Complete, tested |
| `history::load()` | Same | Complete, tested |
| `GateRunRecord` type | `crates/assay-types/src/gate_run.rs` | Complete, with `deny_unknown_fields` |
| `EnforcementSummary` | `crates/assay-types/src/enforcement.rs` | Complete |
| `scan()` with errors | `crates/assay-core/src/spec/mod.rs` | Errors collected, not surfaced |
| `resolve_working_dir()` | `crates/assay-mcp/src/server.rs:561-573` | Exists, no validation |

### What needs to change:

| Requirement | File(s) | Change Type |
|-------------|---------|-------------|
| MCP-01: timeout param | `server.rs` | Add `timeout` field to `GateRunParams`, wrap `spawn_blocking` with `tokio::time::timeout` |
| MCP-02: path validation | `server.rs` | Add `Path::is_dir()` check in `resolve_working_dir` or at call site in `gate_run` |
| MCP-03: spec_list errors | `server.rs` | Surface `scan_result.errors` in `spec_list` response alongside entries |
| MCP-04: docs | `server.rs` | Add doc comments to response structs, review tool descriptions |
| AGNT-05: gate_history | `server.rs` | New tool, new param/response structs, delegates to `history::list` + `history::load` |
| ENFC-04: enforcement in response | `server.rs` | `GateRunResponse` already has `required_failed`/`advisory_failed` -- verify it's correct, possibly add `required_passed`/`advisory_passed` and `blocked: bool` |

## Don't Hand-Roll

| Problem | Use Instead |
|---------|-------------|
| Async timeout | `tokio::time::timeout` -- do NOT poll or manually track elapsed time |
| Run ID generation | `history::generate_run_id` -- already collision-resistant |
| History file layout | `history::list` + `history::load` -- already handles path validation, sorting, JSON deser |
| Response JSON serialization | `serde_json::to_string` -- matches all existing tools |
| Working dir resolution | Extend existing `resolve_working_dir()` -- do NOT create a parallel path |

## Common Pitfalls

### 1. Timeout kills the wrong thing (HIGH confidence)
`tokio::time::timeout` only cancels the **future**, not the spawned blocking task. The `spawn_blocking` closure will continue running. For command-based gates, each command already has its own per-criterion timeout via `resolve_timeout()` (poll-based `try_wait` with process kill). The MCP-level timeout is an **outer ceiling** that catches the case where many criteria run within individual limits but the total exceeds what the agent wants.

**Mitigation:** Accept that `tokio::time::timeout` drops the `JoinHandle` (the blocking task keeps running until its internal per-criterion timeouts fire). Document that this is a best-effort outer limit, not a hard kill. This is pragmatically fine because commands already have their own kill logic.

### 2. spec_list error format needs thought (MEDIUM confidence)
Current `spec_list` returns `Vec<SpecListEntry>`. Adding errors means changing the shape. Two options:
- **Wrap in envelope:** `{ specs: [...], errors: [...] }` -- breaking change for agents already parsing the array
- **Add error_count field + separate list:** pragmatic but two shapes

**Decision needed:** Given this is v0.2.0 and MCP tools are not yet stable, the envelope approach is cleaner. Use `{ specs: [...], errors: [...] }`.

### 3. gate_history tool -- keep it simple (HIGH confidence)
Agents don't need full `GateRunRecord` dumps for every run. The tool should:
- Accept spec name (required) and optional `limit` (default: 10)
- Return lightweight summaries (run_id, timestamp, passed, failed, skipped, enforcement summary, blocked)
- Provide a way to get full detail for a specific run (could be a `run_id` param on the same tool, or `include_details` flag)

**Recommendation:** Single tool with two modes: list mode (spec name only) and detail mode (spec name + run_id).

### 4. Working dir validation timing (HIGH confidence)
`resolve_working_dir` is called in `gate_run` **before** `spawn_blocking`. The validation check (`Path::is_dir()`) should happen right after resolution, before any gate evaluation begins. Return a domain error immediately.

### 5. GateRunResponse already has enforcement fields (HIGH confidence)
`GateRunResponse` already includes `required_failed` and `advisory_failed` (lines 128-129 of server.rs). ENFC-04 asks to "distinguish required vs advisory results" -- the response already does this for failures. Consider adding `required_passed` and `advisory_passed` for completeness, plus a top-level `blocked: bool` field (true when `required_failed > 0`).

### 6. `gate_run` working_dir is from config, not from agent (HIGH confidence)
Currently `gate_run` does NOT accept a `working_dir` parameter from the agent. The working dir comes from `config.gates.working_dir` or defaults to CWD. MCP-02 says "resolve_working_dir validates that the path exists" -- this means validating the **resolved** path (config + CWD), not adding a new parameter.

If agents should be able to override working_dir, that's a separate design decision. The requirement as stated is just validation.

## Code Examples

### MCP-01: Timeout parameter on gate_run

```rust
/// Parameters for the `gate_run` tool.
#[derive(Deserialize, JsonSchema)]
struct GateRunParams {
    #[schemars(description = "Spec name to evaluate gates for")]
    name: String,
    #[schemars(description = "Include full stdout/stderr evidence per criterion (default: false)")]
    #[serde(default)]
    include_evidence: bool,
    #[schemars(description = "Maximum seconds for the entire gate run (default: 300). Individual criteria may have shorter timeouts.")]
    #[serde(default)]
    timeout: Option<u64>,
}

// In gate_run handler:
let gate_timeout = Duration::from_secs(params.0.timeout.unwrap_or(300));
let eval_future = tokio::task::spawn_blocking(move || { /* evaluate_all */ });

match tokio::time::timeout(gate_timeout, eval_future).await {
    Ok(Ok(summary)) => { /* normal path */ }
    Ok(Err(e)) => { /* spawn_blocking panicked */ }
    Err(_elapsed) => {
        return Ok(CallToolResult::error(vec![Content::text(
            format!("gate run timed out after {}s", gate_timeout.as_secs())
        )]));
    }
}
```

### MCP-02: Working dir validation

```rust
// In gate_run, after resolving working_dir:
let working_dir = resolve_working_dir(&cwd, &config);
if !working_dir.is_dir() {
    return Ok(CallToolResult::error(vec![Content::text(
        format!("working directory does not exist: {}", working_dir.display())
    )]));
}
```

### MCP-03: spec_list with errors

```rust
#[derive(Serialize)]
struct SpecListResponse {
    specs: Vec<SpecListEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<SpecListError>,
}

#[derive(Serialize)]
struct SpecListError {
    file: String,
    message: String,
}
```

### AGNT-05: gate_history tool

```rust
#[derive(Deserialize, JsonSchema)]
struct GateHistoryParams {
    #[schemars(description = "Spec name to query history for")]
    name: String,
    #[schemars(description = "Specific run ID to get full details for. When omitted, returns a summary list.")]
    #[serde(default)]
    run_id: Option<String>,
    #[schemars(description = "Maximum number of recent runs to return (default: 10, max: 50)")]
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Serialize)]
struct GateHistoryEntry {
    run_id: String,
    timestamp: String,
    passed: usize,
    failed: usize,
    skipped: usize,
    required_failed: usize,
    advisory_failed: usize,
    blocked: bool,
}
```

### ENFC-04: Enhanced GateRunResponse

```rust
struct GateRunResponse {
    // ... existing fields ...
    required_passed: usize,   // NEW
    required_failed: usize,   // existing
    advisory_passed: usize,   // NEW
    advisory_failed: usize,   // existing
    blocked: bool,            // NEW: true when required_failed > 0
}
```

## Decisions for Planner

1. **spec_list response shape change**: Wrap in `{ specs: [...], errors: [...] }` envelope. This is a breaking change but the MCP surface is pre-1.0.

2. **gate_history single vs two tools**: Use a single `gate_history` tool with optional `run_id` for detail mode. Simpler tool surface for agents.

3. **gate_history runs in blocking context**: `history::list` and `history::load` are synchronous. Wrap with `spawn_blocking` (matches gate_run pattern). For list mode, load each record to extract summary fields -- or just return run IDs and let agent call with `run_id` for details. **Recommendation:** Load summaries for list mode (agents need to see pass/fail without extra calls), but cap at `limit` to bound I/O.

4. **MCP-04 doc scope**: Add `/// field doc` comments to all response struct fields. Update tool `description` strings to reflect new params (timeout, errors). This is mechanical but should be a distinct task.

5. **Timeout floor**: Match existing `MIN_TIMEOUT_SECS = 1` convention. Cap at something reasonable (e.g., 3600s = 1 hour) to prevent unbounded waits.

## Test Strategy

| Requirement | Test Approach |
|-------------|---------------|
| MCP-01 timeout | Unit test: `tokio::time::timeout` with a short timeout around a `sleep` simulating long eval. Verify error response. |
| MCP-02 path validation | Unit test: `resolve_working_dir` with non-existent path, verify `CallToolResult::error`. Integration: `gate_run` with bad `working_dir` in config. |
| MCP-03 errors | Unit test: `SpecListResponse` serialization with errors. Integration: scan dir with a malformed `.toml` file, verify errors in response. |
| MCP-04 docs | Manual review / snapshot test of tool descriptions. |
| AGNT-05 history | Unit test: create temp dir, save records, call history handler logic, verify response shape. Test empty history, single record, multiple records, detail mode. |
| ENFC-04 enforcement | Unit test: `format_gate_response` already tested -- extend with assertions on `required_passed`, `advisory_passed`, `blocked`. |
