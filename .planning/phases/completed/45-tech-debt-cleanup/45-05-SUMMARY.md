# Plan 45-05 Summary: Evaluator Sweep

**Wave:** 2
**Depends on:** 02
**Status:** Complete

## Issues Resolved (13)

| Issue | Change |
|-------|--------|
| `evaluator-error-missing-io-variant` | Added `EvaluatorError::Io(#[from] std::io::Error)` variant |
| `run-evaluator-last-error-unreachable` | Removed `last_error` dead code; replaced fallback with `unreachable!()` |
| `evaluator-schema-lazy-lock-caching` | `evaluator_schema_json()` now returns `&'static str` backed by `LazyLock<String>` |
| `budget-priority-magic-numbers` | Replaced `80`/`50` with `PRIORITY_SPEC: i64 = 80` / `PRIORITY_DIFF: i64 = 50` |
| `map-evaluator-output-duration-param` | Signature changed from `duration_ms: u64` to `duration: Duration` |
| `map-evaluator-output-imperative-counters` | Refactored to functional `fold` with a local `Acc` accumulator struct |
| `build-evaluator-prompt-empty-diff-untested` | Test: `Some("")` and `None` produce identical prompts |
| `map-evaluator-output-empty-criteria-no-count-assertions` | Test: empty criteria yields zero counts across all fields |
| `map-evaluator-output-warn-required-untested` | Test: Warn on Required criterion counts as passed, not failed |
| `map-pass-outcome-kind-role-test-incomplete` | Test: Fail outcome also has AgentReport kind and Independent role |
| `schema-generation-test-key-structure-not-asserted` | Test: schema JSON references `criteria`/`summary` and has recognizable schema keys |
| `budget-test-empty-system-prompt` | Test: empty system prompt is excluded from budget_context output |
| `extract-diff-files-rename-test` | Test: rename diff (`a/old b/new`) returns destination (`b/`) path |

## Files Changed

- `crates/assay-core/src/error.rs` — added `Io` variant to `EvaluatorError`
- `crates/assay-core/src/evaluator.rs` — schema caching, unreachable fallback, Duration param, fold refactor, 5 new tests
- `crates/assay-core/src/context/budgeting.rs` — named priority constants, 1 new test
- `crates/assay-core/src/gate/mod.rs` — 1 new rename diff test
- `crates/assay-mcp/src/server.rs` — updated `map_evaluator_output` call site (Duration), fixed needless borrow

## Verification

`just ready` passes: fmt-check, lint (clippy -D warnings), 546 tests (539 → +7), cargo-deny.
