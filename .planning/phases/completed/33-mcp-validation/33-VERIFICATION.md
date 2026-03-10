---
phase: 33-mcp-validation
status: passed
score: 5/5
verified_by: kata-verifier
date: 2026-03-10
---

# Phase 33: MCP Validation — Verification

## Status: passed
## Score: 5/5 must_haves verified

---

## Criterion 1 (MCP-01): Missing required parameter returns specific error naming the parameter

**Status: PASS**

Four unit tests in `crates/assay-mcp/src/server.rs` (lines 3079–3130) verify serde deserialization error messages for missing required parameters:

- `test_gate_run_params_missing_name` — asserts error contains "missing field" and "name"
- `test_gate_report_params_missing_fields` — asserts error contains "missing field"
- `test_spec_get_params_missing_name` — asserts error contains "missing field" and "name"
- `test_gate_finalize_params_missing_session_id` — asserts error contains "missing field" and "session_id"

serde's `#[derive(Deserialize)]` produces messages like `missing field 'name'` for each missing required field, and rmcp's `Parameters<T>` wrapper propagates these directly as MCP `invalid_params` errors.

---

## Criterion 2 (MCP-02): Invalid parameter type returns specific error naming the parameter and expected type

**Status: PASS**

Three unit tests in `crates/assay-mcp/src/server.rs` (lines 3134–3182) verify serde type-mismatch messages:

- `test_gate_run_params_invalid_timeout_type` — string passed for u64 field, asserts "invalid type" or "invalid value"
- `test_gate_report_params_invalid_passed_type` — string passed for bool field, asserts "invalid type" and "bool"
- `test_gate_run_params_invalid_include_evidence_type` — integer passed for bool field, asserts "invalid type" and "bool"

serde produces messages that name the expected type (e.g., `invalid type: string "abc", expected u64`). rmcp propagates these to the MCP client.

---

## Criterion 3 (MCP-03): Spec-not-found MCP error includes list of available spec names

**Status: PASS**

Evidence found in two layers:

**assay-core layer** (`crates/assay-core/src/spec/mod.rs`, lines 2344–2401, tagged `// ── MCP-03`):
- `spec_not_found_display_includes_available_specs` — creates two specs ("alpha", "beta"), looks up a nonexistent spec, and verifies `err.to_string()` contains "not found", "alpha", and "beta"
- `spec_not_found_display_empty_dir` — verifies "No specs found" appears when the directory is empty

**MCP layer** (`crates/assay-mcp/src/server.rs`):
- `load_spec_entry_mcp()` (line 1079) calls `assay_core::spec::load_spec_entry_with_diagnostics()` which produces `SpecNotFoundDiagnostic` with available spec names populated
- `domain_error()` (line 1105) converts any `AssayError` via `err.to_string()` (Display), which for `SpecNotFoundDiagnostic` includes the available spec list
- `test_load_spec_entry_not_found` (line 1797) verifies the MCP error contains a diagnostic message

The full chain from spec lookup through Display through MCP error propagation is tested.

---

## Criterion 4 (MCP-04): Gate failure reason checks stdout in addition to stderr

**Status: PASS**

Implementation in `format_gate_response()` at line 1228:
```rust
let reason = first_nonempty_line(&gate_result.stderr)
    .or_else(|| first_nonempty_line(&gate_result.stdout))
    .unwrap_or("unknown")
    .to_string();
```

Three tests verify the behavior (`crates/assay-mcp/src/server.rs`, lines 1596–1731):
- `test_failure_reason_prefers_stderr` — both populated, reason comes from stderr
- `test_failure_reason_falls_back_to_stdout` — empty stderr, reason comes from stdout ("error from stdout")
- `test_failure_reason_both_empty_shows_unknown` — both empty, reason is "unknown"

---

## Criterion 5 (MCP-05): gate_run handler has no unnecessary clone intermediaries

**Status: PASS**

Two target clones eliminated in the `gate_run` handler (`crates/assay-mcp/src/server.rs`):

1. `summary.results.clone()` (was ~line 578): Eliminated. `summary.results` is now moved directly via `let deterministic_results = summary.results;` (line 575), then passed by value to `create_session()`.

2. `summary.clone()` (was ~line 635): Eliminated. `summary.spec_name` is extracted first (`let spec_name_for_log = summary.spec_name.clone();`, line 634), then `summary` is moved directly into `save_run()` (line 639) without cloning the whole struct.

Remaining `session_id.clone()` calls (lines 583, 586, 593) are String clones required because `session_id` is used in three distinct places (response, HashMap insert, async task). These are explicitly acceptable per plan decision "Minor String clones (session_id) are acceptable — keep those as-is per research findings."

---

## just ready Result

All checks pass: fmt-check, lint, test (414 assay-core tests + 60 assay-mcp tests), deny.

No warnings or errors. Only pre-existing `license-not-encountered` advisory warnings in cargo-deny (unrelated to this phase).
