# Phase 29: Gate Output Truncation -- Verification

**Status:** passed
**Score:** 5/5 must-haves verified
**Date:** 2026-03-09

## Criteria Verification

### 1. Gate command output exceeding the byte budget is truncated with head and tail sections preserved

**Verified.** The `truncate_head_tail` function at `crates/assay-core/src/gate/mod.rs:653` implements head+tail truncation. When input exceeds the budget, it splits into head (~33% of budget) and tail (~67% of budget) sections. The `STREAM_BUDGET` constant is set to 32,768 bytes (32 KiB) at line 37.

Tests: `truncate_head_tail_over_budget`, `truncate_head_tail_preserves_head_and_tail`.

### 2. Truncated output contains a `[truncated: X bytes omitted]` marker between head and tail

**Verified.** Line 679: `format!("{head}\n[truncated: {omitted} bytes omitted]\n{tail}")`. The marker is inserted between newline-separated head and tail sections, with the exact omitted byte count.

Tests: `truncate_head_tail_marker_format`.

### 3. Truncation never splits a multi-byte UTF-8 sequence

**Verified.** The function uses `floor_char_boundary` (line 667) for the head end position and `ceil_char_boundary` (line 668) for the tail start position. These standard library methods ensure cuts land on valid UTF-8 character boundaries. An overlap guard at line 670-673 handles edge cases where head and tail would overlap by falling back to tail-only with a safe start boundary.

Tests: `truncate_head_tail_utf8_multibyte` (3-byte chars: CJK), `truncate_head_tail_utf8_4byte` (4-byte chars: emoji). Both tests verify every character in the output is either the expected multi-byte char or an ASCII marker character.

### 4. stdout and stderr have independent byte budgets

**Verified.** In `evaluate_command` (lines 546-547):
```rust
let stdout_result = truncate_head_tail(&stdout_raw, STREAM_BUDGET);
let stderr_result = truncate_head_tail(&stderr_raw, STREAM_BUDGET);
```
Each stream is truncated independently against its own `STREAM_BUDGET`.

Test: `evaluate_command_independent_stream_truncation` generates large stdout (> STREAM_BUDGET) with small stderr, then asserts stdout contains the truncation marker while stderr does not.

### 5. `GateResult.truncated` is `true` and `GateResult.original_bytes` reflects the pre-truncation size

**Verified.** In `evaluate_command` (lines 549-554):
```rust
let truncated = stdout_result.truncated || stderr_result.truncated;
let original_bytes = if truncated {
    Some((stdout_result.original_bytes + stderr_result.original_bytes) as u64)
} else {
    None
};
```
The `truncated` field is set to `true` if either stream was truncated. `original_bytes` is the combined pre-truncation size of both streams (set only when truncation occurred).

The `GateResult` type in `crates/assay-types/src/gate.rs` defines both fields with appropriate serde attributes (`truncated` skipped when false, `original_bytes` skipped when `None`).

Tests: `evaluate_command_independent_stream_truncation` asserts `result.truncated` is true and `result.original_bytes.is_some()`. Type-level tests in `assay-types` verify serialization behavior.

## Additional Checks

- **Old code removed:** No references to `MAX_OUTPUT_BYTES` or `truncate_output` exist anywhere in the codebase (confirmed via search).
- **Tests pass:** All 356 `assay-core` tests pass. One unrelated pre-existing failure exists in `assay-mcp` (`gate_history_no_history_returns_empty`) which is not related to Phase 29.
- **Edge case coverage:** Tests cover empty input, exact budget, tiny budget (1 byte), overlap guard, and multi-byte UTF-8 (3-byte and 4-byte characters).

## Gaps

None.
