# Phase 33: MCP Validation - Research

**Researched:** 2026-03-10
**Confidence:** HIGH (all questions answered via source code analysis)

## Standard Stack

- **MCP SDK:** `rmcp 0.17.0` (crate in workspace)
- **Parameter deserialization:** `rmcp::handler::server::wrapper::Parameters<T>` + serde
- **Error types:** `rmcp::ErrorData` (protocol-level JSON-RPC errors) vs `CallToolResult::error()` (domain errors visible to agents)
- **Spec loading:** `assay_core::spec::load_spec_entry_with_diagnostics()` already produces `SpecNotFoundDiagnostic`

## Architecture Patterns

### MCP Parameter Validation Flow (KEY FINDING)

The `Parameters<T>` extractor in rmcp works as follows (`rmcp/src/handler/server/tool.rs:166-181`):

1. `FromContextPart::from_context_part()` is called BEFORE the handler
2. It takes `context.arguments` (a `JsonObject`), wraps it as `Value::Object`, and calls `serde_json::from_value::<T>()`
3. On deserialization failure, it returns `Err(ErrorData::invalid_params("failed to deserialize parameters: {error}", None))`
4. The macro-generated dispatch code (`impl_for!`) returns `Err(e)` immediately -- **the handler is never called**
5. This `Err(ErrorData)` is a **protocol-level JSON-RPC error** (error code `-32602`), not a `CallToolResult` with `isError: true`

### Serde Error Messages (VERIFIED)

Tested actual serde_json error output for `#[derive(Deserialize)]` structs:

| Scenario | Serde Error Message |
|---|---|
| Missing required field | `missing field 'name'` |
| String where bool expected | `invalid type: string "not-a-bool", expected a boolean` |
| Integer where string expected | `invalid type: integer '123', expected a string` |
| String where u64 expected | `invalid type: string "abc", expected u64` |

Combined with rmcp's prefix, agents see:
`"failed to deserialize parameters: missing field 'name'"`

**Assessment:** Serde errors already name the parameter clearly. The field name is always included in the missing-field message. Type mismatch messages describe what was received and what was expected. These messages meet MCP-01 and MCP-02 success criteria as-is.

### What `#[serde(default)]` Does

Fields with `#[serde(default)]` are never "missing" from serde's perspective. In the existing param structs:

| Struct | Required Fields | Defaulted Fields |
|---|---|---|
| `GateRunParams` | `name` | `include_evidence`, `timeout` |
| `GateReportParams` | `session_id`, `criterion_name`, `passed`, `evidence`, `reasoning`, `evaluator_role` | `confidence` |
| `GateFinalizeParams` | `session_id` | (none) |
| `SpecGetParams` | `name` | (none) |
| `GateHistoryParams` | `name` | `run_id`, `limit` |
| `ContextDiagnoseParams` | (none) | `session_id` |
| `EstimateTokensParams` | (none) | `session_id` |
| `WorktreeCreateParams` | `name` | `base`, `worktree_dir` |
| `WorktreeListParams` | (none) | `worktree_dir` |
| `WorktreeStatusParams` | `name` | `worktree_dir` |
| `WorktreeCleanupParams` | `name` | `force`, `worktree_dir` |

## Don't Hand-Roll

- **Parameter validation layer**: Serde + rmcp already handles this. Do NOT add custom validation code for missing/wrong-type parameters. The existing `Parameters<T>` + `#[derive(Deserialize)]` with `#[serde(default)]` annotations already produce clear, parameter-naming error messages.
- **Custom error wrapping for deserialization**: The rmcp SDK already wraps serde errors with `"failed to deserialize parameters: {error}"`. Do not intercept or re-wrap.

## Common Pitfalls

### Pitfall 1: Confusing Protocol Errors with Domain Errors

Parameter validation errors from `Parameters<T>` are `ErrorData` (JSON-RPC protocol errors, code `-32602`). They are NOT `CallToolResult::error()` (domain errors with `isError: true`). The handler never executes.

**Implication for testing:** You cannot test parameter validation errors by calling `server.gate_run(Parameters(...))` directly in Rust -- that bypasses the deserialization layer (you're constructing `Parameters<T>` yourself). To test the actual MCP parameter validation, you would need to go through the full MCP JSON-RPC transport, or test serde deserialization separately.

### Pitfall 2: Spec-Not-Found Already Works

`load_spec_entry_mcp` calls `load_spec_entry_with_diagnostics` which produces `SpecNotFoundDiagnostic` with:
- List of available spec names
- List of invalid (unparseable) spec names
- Optional fuzzy-match suggestion (via Levenshtein distance)

The `domain_error()` function calls `err.to_string()` which triggers the `#[error(...)]` Display impl, which calls `format_spec_not_found()` -- this produces messages like:
`"spec 'xyz' not found. Available specs: alpha, beta"`

**This is already complete.** MCP-03 is verification-only.

### Pitfall 3: Testing Validation at Wrong Layer

Since parameter validation happens at the rmcp deserialization layer (before the handler), existing integration tests that construct `Parameters(GateRunParams { ... })` directly do NOT test parameter validation. Writing "missing parameter" tests requires either:

a) **Unit-testing serde deserialization** of the param structs directly (recommended -- simple, fast)
b) Going through the full MCP JSON-RPC protocol (complex, unnecessary)

Option (a) is the right approach: `serde_json::from_value::<GateRunParams>(json)` and assert the error message.

## Code Examples

### Testing Parameter Validation (Serde Layer)

```rust
#[test]
fn gate_run_params_missing_name() {
    let json = serde_json::json!({});
    let err = serde_json::from_value::<GateRunParams>(json).unwrap_err();
    assert!(
        err.to_string().contains("missing field"),
        "should mention missing field: {err}"
    );
    assert!(
        err.to_string().contains("name"),
        "should name the parameter: {err}"
    );
}

#[test]
fn gate_run_params_wrong_type() {
    let json = serde_json::json!({"name": 123});
    let err = serde_json::from_value::<GateRunParams>(json).unwrap_err();
    assert!(
        err.to_string().contains("invalid type"),
        "should mention invalid type: {err}"
    );
}
```

### Stdout Fallback for Failure Reason (MCP-04)

Current code (`server.rs:1224`):
```rust
let reason = first_nonempty_line(&gate_result.stderr)
    .unwrap_or("unknown")
    .to_string();
```

Change to:
```rust
let reason = first_nonempty_line(&gate_result.stderr)
    .or_else(|| first_nonempty_line(&gate_result.stdout))
    .unwrap_or("unknown")
    .to_string();
```

### Clone Removal in gate_run (MCP-05)

#### Clone 1: `summary.clone()` at line 635

```rust
// BEFORE (line 633-643):
if let Err(e) = assay_core::history::save_run(
    &assay_dir,
    summary.clone(),  // clone for save_run (takes ownership)
    Some(working_dir.to_string_lossy().to_string()),
    max_history,
) {
    tracing::warn!(
        spec_name = %summary.spec_name,  // only use after save_run
        "failed to save command-only gate run history: {e}"
    );
}

// AFTER: extract spec_name before moving summary
let spec_name = summary.spec_name.clone(); // cheap String clone
if let Err(e) = assay_core::history::save_run(
    &assay_dir,
    summary,  // moved, not cloned
    Some(working_dir.to_string_lossy().to_string()),
    max_history,
) {
    tracing::warn!(
        spec_name = %spec_name,
        "failed to save command-only gate run history: {e}"
    );
}
```

**Savings:** Avoids cloning entire `GateRunSummary` (which contains `Vec<CriterionResult>` with stdout/stderr strings).

#### Clone 2: `summary.results.clone()` at line 578

```rust
// BEFORE (line 574-578):
let session = assay_core::gate::session::create_session(
    &summary.spec_name,
    info.agent_criteria_names,
    info.spec_enforcement,
    summary.results.clone(),  // clone Vec<CriterionResult>
);
```

After `format_gate_response(&summary, include_evidence)` on line 570, `summary` is only used for:
- `&summary.spec_name` (line 575) -- borrow
- `summary.results.clone()` (line 578) -- this clone

Since `summary` is not used after line 578 in the `if let Some(info)` branch, we can extract `spec_name` first, then destructure or move results:

```rust
// AFTER: avoid cloning results by extracting what we need
let spec_name_ref = summary.spec_name.as_str(); // can't borrow after move
// Actually need to clone spec_name since create_session takes &str and summary is consumed
let spec_name = summary.spec_name.clone();
let results = summary.results; // move, not clone
let session = assay_core::gate::session::create_session(
    &spec_name,
    info.agent_criteria_names,
    info.spec_enforcement,
    results,
);
```

But `format_gate_response` borrows `&summary` and returns `response` which does NOT hold a reference to summary (all fields are owned). So after `format_gate_response`, `summary` can be consumed.

**Better approach:** restructure to move `summary` fields after `format_gate_response`:

```rust
let mut response = format_gate_response(&summary, include_evidence);

if let Some(info) = agent_info {
    let spec_name = summary.spec_name;  // move
    let results = summary.results;      // move (avoids clone!)
    let session = assay_core::gate::session::create_session(
        &spec_name,
        info.agent_criteria_names,
        info.spec_enforcement,
        results,
    );
    // ... rest unchanged
} else {
    // For the else branch, summary is consumed by save_run
    // Need to restructure slightly
}
```

The challenge: `summary` is consumed in EITHER the `if` or `else` branch but the compiler needs to prove it's consumed in exactly one. Solution: destructure summary into individual fields before the branch, or accept partial moves.

**Recommended approach:** Move both branches to consume `summary` by value. Extract `spec_name` for tracing before consuming. The `format_gate_response` call already happens before the branch, so `summary` can be consumed after it.

#### Clone 3: `session_id` clones (lines 581, 584, 591)

Three clones of `session.session_id`:
- Line 581: `let session_id = session.session_id.clone()` -- extract before session is moved into HashMap
- Line 584: `response.session_id = Some(session_id.clone())` -- into response
- Line 591: `.insert(session_id.clone(), session)` -- as HashMap key

One clone can be saved by reordering:
```rust
response.session_id = Some(session.session_id.clone()); // 1 clone
let session_id = session.session_id.clone();             // 2nd clone (for HashMap key + spawn)
self.sessions.lock().await.insert(session_id.clone(), session); // HashMap key needs owned String
// session_id moves into spawned task
```

Actually this is the same count. The minimum is 2 clones from `session.session_id` (one for response, one for the spawn task -- the HashMap key can share with one of these but needs its own). Minor optimization at best.

**Verdict:** Focus on clones 1 and 2 (significant savings). Clone 3 is minor -- three String clones of a UUID is negligible.

## Research Findings Summary

| Requirement | Finding | Work Needed |
|---|---|---|
| MCP-01 (missing param errors) | Serde already names the field: `"missing field 'name'"`. rmcp wraps it: `"failed to deserialize parameters: missing field 'name'"` | Verification tests only |
| MCP-02 (invalid type errors) | Serde already describes the mismatch: `"invalid type: string \"abc\", expected u64"` | Verification tests only |
| MCP-03 (spec-not-found with available names) | Already implemented via `load_spec_entry_with_diagnostics` -> `SpecNotFoundDiagnostic` -> `format_spec_not_found()` | Verification test only |
| MCP-04 (stdout fallback for failure reason) | One-line change in `format_gate_response` at line 1224 | Small implementation + test |
| MCP-05 (clone removal in gate_run) | Two significant clones removable: `summary.clone()` (line 635) and `summary.results.clone()` (line 578). Session ID clones (lines 581/584/591) are minor. | Refactor + existing tests verify |

### Critical Insight for MCP-01/MCP-02

The validation happens at the **protocol layer** (JSON-RPC error code -32602), not the tool layer. The handler never executes. This means:

1. **No custom validation code is needed** -- serde + rmcp already provide specific error messages
2. **Tests should verify serde deserialization directly** (unit tests on param structs), not through the MCP handler
3. The CONTEXT.md decision about "return via `domain_error()` -> `CallToolResult::error()`" is moot -- errors flow through `ErrorData::invalid_params`, not `CallToolResult`

### Test Strategy

- **MCP-01/02:** Unit tests deserializing param structs with `serde_json::from_value` and asserting error messages contain field names and type descriptions
- **MCP-03:** Integration test calling `server.spec_get()` (or `gate_run()`) with nonexistent spec name and asserting the response text contains "Available specs:"
- **MCP-04:** Unit test for `format_gate_response` with empty stderr + non-empty stdout, asserting reason comes from stdout
- **MCP-05:** No new tests needed -- existing `gate_lifecycle_run_report_finalize` and `gate_run_command_only_persists_history` cover the gate_run flow. Run `just test` to verify refactor is sound.

---

*Phase: 33-mcp-validation*
*Research completed: 2026-03-10*
