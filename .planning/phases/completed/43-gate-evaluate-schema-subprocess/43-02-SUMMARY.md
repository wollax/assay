# Phase 43 Plan 02: gate_evaluate MCP Tool Handler Summary

**One-liner:** Wired the gate_evaluate MCP tool in assay-mcp, orchestrating the full 10-step flow from spec loading through subprocess evaluation to history persistence and session auto-linking.

## Completed Tasks

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | gate_evaluate parameter struct and MCP handler | 8aebc64 | `crates/assay-mcp/src/server.rs` |
| 2 | Integration verification and final checks | 6bd653d | `crates/assay-mcp/src/server.rs` (fmt) |

## What Was Built

### gate_evaluate MCP Tool Handler

Full 10-step orchestration in a single async handler:

1. **Load config and spec** — reuses `load_config` and `load_spec_entry_mcp` helpers
2. **Resolve working directory** — from session's `worktree_path` (if session_id) or config fallback
3. **Compute git diff** — `git diff HEAD` with `truncate_diff(DIFF_BUDGET_BYTES)` budget
4. **Build evaluator prompt** — via `assay_core::evaluator::build_evaluator_prompt`
5. **Build system prompt and schema** — via core module functions
6. **Construct EvaluatorConfig** — parameter > config > default precedence for model/timeout/retries
7. **Spawn evaluator subprocess** — async `run_evaluator()` with full error variant handling
8. **Map to GateRunRecord** — `map_evaluator_output` with enforcement map from spec criteria
9. **Persist via history::save** — with warning on failure (Phase 35 pattern)
10. **Session auto-linking** — `record_gate_result` transitions to GateEvaluated, adds warning on failure

### Parameter Struct (GateEvaluateParams)
- `name` (required): spec name
- `session_id` (optional): work session for worktree resolution and auto-linking
- `timeout` (optional): evaluator subprocess timeout override
- `model` (optional): evaluator model override

### Response Structure (GateEvaluateResponse)
- `run_id`, `spec_name`, `overall_passed`, `evaluator_model`, `duration_ms`
- `summary`: passed/failed/skipped/required_failed/advisory_failed/blocked
- `results`: per-criterion with outcome/reasoning/evidence/enforcement
- `warnings`: accumulated from parse, mapping, save, and session operations
- `session_id`: echoed back when provided

### Error Handling
All `EvaluatorError` variants mapped to `CallToolResult` with `isError: true`:
- `NotInstalled` → install hint
- `Timeout` → suggests increasing timeout or reducing criteria
- `Crash` → exit code + stderr excerpt (500 chars)
- `ParseError` → error details + raw output excerpt
- `NoStructuredOutput` → raw output excerpt
- Catch-all for non-exhaustive enum safety

### Module Updates
- Doc comment updated to "eighteen tools"
- `gate_evaluate` added to tool list
- Server instructions updated to recommend gate_evaluate for automated evaluation

## Decisions Made

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | Separate match arms for ParseError and NoStructuredOutput | Combined arm would bind `error` field that NoStructuredOutput lacks |
| 2 | Catch-all `Err(e)` arm for non-exhaustive EvaluatorError | Required by `#[non_exhaustive]` attribute on the enum |
| 3 | agent_prompt built from concatenation of all criteria prompts | Provides evaluator with full guidance context without per-criterion subprocess calls |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Non-exhaustive enum match**
- **Found during:** Task 1 build
- **Issue:** `EvaluatorError` is `#[non_exhaustive]`, requiring a wildcard arm in external crate matches
- **Fix:** Added `Err(e) => ...` catch-all with generic error message

**2. [Rule 1 - Bug] Clippy needless-borrows-for-generic-args**
- **Found during:** Task 2 lint
- **Issue:** `&cr.outcome` in `serde_json::to_value` call — `CriterionOutcome` implements `Copy`
- **Fix:** Removed unnecessary borrow

**3. [Rule 3 - Blocking] rustfmt formatting**
- **Found during:** Task 2 fmt-check
- **Issue:** Several schemars attributes and multiline expressions not formatted per project style
- **Fix:** Ran `cargo fmt --all`

## Verification

```
just ready → All checks passed
  fmt-check: pass
  lint: pass (clippy -D warnings)
  test: pass (786 passed, 3 ignored)
  deny: pass (advisories, bans, licenses, sources)
```

ORCH-01: gate_evaluate tool orchestrates full evaluation flow (10 steps).
ORCH-02: Evaluator subprocess uses `--tools ""` and `--max-turns 1` (verified in core module).
ORCH-03: Lenient `serde_json::Value` intermediate parse with `structured_output` extraction (verified in core module).

## Metrics

- **Duration:** ~7 minutes
- **Completed:** 2026-03-15
- **Tests added:** 0 (handler relies on core module tests; full integration requires live subprocess)
- **Files modified:** 1 (`crates/assay-mcp/src/server.rs`)
- **Lines added:** ~438 (handler + param struct + response structs + doc updates)
