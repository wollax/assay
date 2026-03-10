# Phase 34: MCP Truncation Visibility — Research

## Summary

Add `truncated` and `original_bytes` fields to the MCP `CriterionSummary` struct so agents can programmatically detect when command output was truncated, rather than relying on the in-band `[truncated: N bytes omitted]` text marker.

## Standard Stack

- **No new dependencies.** This is a pure struct + field-population change within `crates/assay-mcp/src/server.rs`.
- Serialization: `serde` with `skip_serializing_if` (already used for all optional fields on `CriterionSummary`).
- Source of truth for truncation data: `GateResult.truncated: bool` and `GateResult.original_bytes: Option<u64>` in `crates/assay-types/src/gate.rs`.

## Architecture Patterns

### CriterionSummary struct (the target)

**File:** `crates/assay-mcp/src/server.rs`, line 343

Current fields (all use `#[serde(skip_serializing_if = "Option::is_none")]` for optionals):
- `name: String`
- `status: String` — `"passed"`, `"failed"`, or `"skipped"`
- `enforcement: String`
- `kind_label: Option<String>`
- `exit_code: Option<i32>`
- `duration_ms: Option<u64>`
- `reason: Option<String>`
- `stdout: Option<String>`
- `stderr: Option<String>`

**Pattern to follow:** Add two new optional fields at the end, before or after `stderr`:
```rust
/// Whether stdout or stderr was truncated. Absent for skipped criteria.
#[serde(skip_serializing_if = "Option::is_none")]
truncated: Option<bool>,
/// Original combined byte count before truncation. Absent when not truncated.
#[serde(skip_serializing_if = "Option::is_none")]
original_bytes: Option<u64>,
```

Use `Option<bool>` (not bare `bool`) to match the existing pattern where skipped criteria omit fields entirely. The phase description says `Option<bool>` and `Option<u64>`.

### GateResult (the source)

**File:** `crates/assay-types/src/gate.rs`, line 58

Relevant fields:
- `pub truncated: bool` (line 89) — `true` when output was truncated
- `pub original_bytes: Option<u64>` (line 94) — byte count before truncation, `None` when not truncated

### format_gate_response (the mapping function)

**File:** `crates/assay-mcp/src/server.rs`, line 1184

Constructs `CriterionSummary` in three match arms:
1. **Skipped** (line 1199): `cr.result` is `None` — set both new fields to `None`
2. **Passed** (line 1210): `gate_result.passed` is `true` — populate from `gate_result`
3. **Failed** (line 1229): everything else — populate from `gate_result`

For arms 2 and 3, the mapping is:
```rust
truncated: Some(gate_result.truncated),
original_bytes: gate_result.original_bytes,
```

Note: `truncated` wraps the bare `bool` in `Some()` so skipped criteria can use `None`. `original_bytes` is already `Option<u64>` and passes through directly.

### CLI reference pattern

**File:** `crates/assay-cli/src/commands/gate.rs`, line 820

The CLI uses `result.truncated` (bare bool) and prints `[output truncated]` when true. The MCP approach is analogous but exposes the structured data instead of a text marker.

## Don't Hand-Roll

- **Do not add a `schemars::JsonSchema` derive to `CriterionSummary`.** It currently only derives `Serialize`. It is an MCP-internal struct, not part of the public schema registry. Leave it as-is.
- **Do not change `GateResult` or any types crate code.** The source fields already exist and are correct.
- **Do not change the text-based truncation marker** (`[truncated: N bytes omitted]`) that appears inline in stdout/stderr. The new fields are supplementary structured metadata.

## Common Pitfalls

1. **Using bare `bool` instead of `Option<bool>` for `truncated`.** Skipped criteria have no `GateResult` so there is no truncation status. Using bare `bool` would emit `"truncated": false` for skipped criteria, which is misleading. Must be `Option<bool>`.

2. **Forgetting to update the skipped arm.** The skipped arm (line 1199) must explicitly set `truncated: None, original_bytes: None`. Rust will catch this as a compile error, but the test should also verify it.

3. **Forgetting to update all test helper construction sites.** Every place that constructs `CriterionSummary` in tests must include the new fields. There are several locations (lines 1979, 1990, 2001, 2104). These will be compile errors.

4. **Not testing the `include_evidence=false` case.** The new fields should be present regardless of `include_evidence` — truncation metadata is not evidence, it's metadata about the response. Verify that `truncated` and `original_bytes` appear in both summary and evidence modes.

5. **JSON serialization: `Option<bool>` with `skip_serializing_if`.** `Some(false)` serializes as `"truncated": false`. This is correct — for non-truncated, non-skipped criteria, the agent sees `false`. For skipped criteria, the field is omitted entirely. This matches the existing `exit_code`/`duration_ms` pattern.

## Code Examples

### Struct addition
```rust
// In CriterionSummary (server.rs:343), add after stderr field:

/// Whether stdout or stderr was truncated due to size limits.
/// Absent for skipped criteria.
#[serde(skip_serializing_if = "Option::is_none")]
truncated: Option<bool>,
/// Original combined byte count before truncation.
/// Absent when output was not truncated or criterion was skipped.
#[serde(skip_serializing_if = "Option::is_none")]
original_bytes: Option<u64>,
```

### Mapping in format_gate_response
```rust
// Skipped arm (line 1199):
truncated: None,
original_bytes: None,

// Passed arm (line 1210):
truncated: Some(gate_result.truncated),
original_bytes: gate_result.original_bytes,

// Failed arm (line 1234):
truncated: Some(gate_result.truncated),
original_bytes: gate_result.original_bytes,
```

### Test: truncated criterion in evidence mode
```rust
#[test]
fn test_format_gate_response_truncated_fields() {
    let mut summary = sample_summary();
    // Make the first criterion truncated
    if let Some(ref mut result) = summary.results[0].result {
        result.truncated = true;
        result.original_bytes = Some(524_288);
    }

    // Summary mode: truncated fields present even without evidence
    let response = format_gate_response(&summary, false);
    assert_eq!(response.criteria[0].truncated, Some(true));
    assert_eq!(response.criteria[0].original_bytes, Some(524_288));

    // Non-truncated criterion
    assert_eq!(response.criteria[1].truncated, Some(false));
    assert_eq!(response.criteria[1].original_bytes, None);

    // Skipped criterion: both None
    assert_eq!(response.criteria[2].truncated, None);
    assert_eq!(response.criteria[2].original_bytes, None);

    // Verify JSON serialization: skipped omits truncated field
    let json = serde_json::to_value(&response).unwrap();
    assert!(json["criteria"][2].get("truncated").is_none());
    assert!(json["criteria"][2].get("original_bytes").is_none());

    // Truncated criterion has both fields in JSON
    assert_eq!(json["criteria"][0]["truncated"], true);
    assert_eq!(json["criteria"][0]["original_bytes"], 524_288);

    // Non-truncated criterion has truncated=false, no original_bytes
    assert_eq!(json["criteria"][1]["truncated"], false);
    assert!(json["criteria"][1].get("original_bytes").is_none());
}
```

## Affected Files

| File | Change |
|------|--------|
| `crates/assay-mcp/src/server.rs:343` | Add `truncated` and `original_bytes` fields to `CriterionSummary` |
| `crates/assay-mcp/src/server.rs:1184` | Populate new fields in all 3 arms of `format_gate_response()` |
| `crates/assay-mcp/src/server.rs` (tests) | Update all `CriterionSummary` construction sites; add dedicated truncation test |

No changes needed in `assay-types`, `assay-core`, or `assay-cli`.

## Confidence Levels

| Finding | Confidence |
|---------|-----------|
| `CriterionSummary` location and structure | HIGH — read directly from source |
| `GateResult.truncated` / `original_bytes` field names and types | HIGH — read directly from source |
| `format_gate_response` mapping logic and 3-arm structure | HIGH — read directly from source |
| `Option<bool>` for truncated (not bare bool) | HIGH — matches existing skip-on-None pattern for skipped criteria |
| New fields should be independent of `include_evidence` flag | HIGH — truncation metadata describes the response shape, not evidence content |
| No schemars derive needed on CriterionSummary | HIGH — verified struct only derives `Serialize` |
| Compile errors will guide all required test updates | HIGH — Rust exhaustive struct construction |
