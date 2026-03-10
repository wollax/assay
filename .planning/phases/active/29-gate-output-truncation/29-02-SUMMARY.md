# Phase 29 Plan 02: Integrate Head+Tail Truncation Summary

Wired `truncate_head_tail` into `evaluate_command`, replacing the old tail-only truncation with independent 32 KiB per-stream budgets.

## Changes

- Replaced `MAX_OUTPUT_BYTES` (64 KiB) constant with `STREAM_BUDGET` (32 KiB)
- Updated `evaluate_command` to call `truncate_head_tail` instead of `truncate_output`
- Updated both match arms (normal exit and timeout) to use `TruncationResult.output`
- Removed `#[allow(dead_code)]` annotations from `TruncationResult` and `truncate_head_tail`
- Deleted `truncate_output` function entirely
- Replaced old `truncate_output_*` tests with `evaluate_command_independent_stream_truncation` integration test (GATE-04)

## Deviations

None.

## Commits

| Hash | Message |
|------|---------|
| `b5321c8` | `feat(29-02): integrate head+tail truncation into gate evaluation` |

## Verification

- `just fmt-check` — pass
- `just lint` — pass
- `just test` — pass (356 tests)
- `just deny` — pass
- `grep truncate_output\|MAX_OUTPUT_BYTES` — zero matches (dead code fully removed)
- `check-plugin-version` — pre-existing failure (unrelated plugin version mismatch)

## Requirements Satisfied

- **GATE-04**: stdout and stderr truncated independently with separate 32 KiB budgets
- **GATE-05**: `GateResult.truncated` true when either stream truncated; `original_bytes` reflects combined pre-truncation size
