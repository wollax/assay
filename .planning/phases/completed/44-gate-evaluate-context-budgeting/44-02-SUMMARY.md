# 44-02 Summary: budget_context Integration into gate_evaluate

**Phase:** 44 — gate_evaluate Context Budgeting
**Plan:** 02 of 02 — budget_context integration
**Status:** COMPLETE
**Completed:** 2026-03-15

## What Was Done

Replaced the byte-budget diff truncation in `gate_evaluate` with token-aware budgeting via `budget_context`. The handler now computes how much of the diff fits within the model's actual context window — after subtracting system prompt, spec body, and criteria text — rather than applying a fixed 32 KiB cap.

### Changes to `crates/assay-mcp/src/server.rs`

**Restructured gate_evaluate steps 3-8 (formerly 3-6):**

- **Step 3 (new):** Resolve model and gates config (moved before diff capture so model is available for `context_window_for_model`)
- **Step 4 (new):** Capture raw git diff without truncation
- **Step 5 (new):** Build system prompt, schema, and criteria text (moved before budgeting)
- **Step 6 (new):** Call `budget_context` with system prompt, spec description, criteria text, raw diff, and model window to get token-budgeted content
- **Step 7 (renamed):** Build evaluator prompt with (possibly truncated) diff
- **Step 8 (renamed):** Construct `EvaluatorConfig` using already-resolved model

**Truncation handling in Step 6:**

- Compares `budget_context` output length against raw diff length to detect truncation
- When truncated: extracts `extract_diff_files` on both raw and budgeted diffs to compute `included_files` / `omitted_files`
- Emits a structured warning: `"Diff truncated from X to Y bytes (N files kept, M files omitted) to fit token budget"`
- Builds `assay_types::DiffTruncation` metadata
- Graceful fallback: if `budget_context` errors, passes full diff through with a warning (no crash)

**GateEvaluateResponse extended:**

- Added `diff_truncation: Option<assay_types::DiffTruncation>` with `#[serde(skip_serializing_if = "Option::is_none")]`
- Field is absent from JSON when no truncation occurred

**GateRunRecord mutation:**

- After `map_evaluator_output`, sets `record.diff_truncation = diff_truncation.clone()` to persist metadata with the history record

## Task Commits

| Task | Commit | Description |
|------|--------|-------------|
| Task 1 | 18ace32 | feat(44-02): wire budget_context into gate_evaluate diff truncation |

## Verification

- `cargo check -p assay-mcp`: passes
- `cargo clippy -p assay-mcp -- -D warnings`: passes
- `just ready` (fmt-check + lint + test + deny): passes

## Success Criteria

| # | Criteria | Status |
|---|----------|--------|
| 1 | `gate_evaluate` uses `budget_context` instead of `truncate_diff` | DONE |
| 2 | Diff budget = model window - system prompt - spec body - criteria text | DONE |
| 3 | `DiffTruncation` populated on `GateRunRecord` when truncation occurs | DONE |
| 4 | Truncation triggers a warning in MCP response | DONE |
| 5 | `GateEvaluateResponse` includes `diff_truncation` only when truncated | DONE |
| 6 | No truncation metadata when diff fits within budget | DONE |
| 7 | `budget_context` failure falls back gracefully with warning | DONE |
| 8 | `just ready` passes with no regressions | DONE |

## Key Decisions

- `model` resolution moved before diff capture (was in EvaluatorConfig step) so `context_window_for_model` has model available
- Criteria text built locally in Step 5 (cheap double computation vs. refactoring `build_evaluator_prompt` API)
- Truncation detection uses byte length comparison — reliable since `budget_context` passthrough returns identical strings when no truncation needed
- `DIFF_BUDGET_BYTES` constant retained (still used by `gate_run` handler)

## ORCH Requirements Satisfied

- **ORCH-04:** gate_evaluate computes diff token budget via context engine (model window - overhead). Complete.
- **ORCH-05:** Diff is truncated with head-first + tail fallback via cupel pipeline in `budget_context`. Complete.
