# Phase 43: gate_evaluate Schema & Subprocess — UAT

**Date:** 2026-03-15
**Status:** PASSED (10/10)

## Tests

| # | Test | Expected | Result |
|---|------|----------|--------|
| 1 | EvaluatorOutput JSON schema generates valid JSON | `schemars::schema_for!(EvaluatorOutput)` produces parseable JSON with criteria/summary fields | PASS |
| 2 | CriterionOutcome serializes as snake_case | Pass→"pass", Fail→"fail", Skip→"skip", Warn→"warn" | PASS |
| 3 | GatesConfig backward compatibility | Existing assay.toml without evaluator fields parses with defaults (sonnet/1/120) | PASS (51 config tests) |
| 4 | gate_evaluate appears as 18th tool in MCP server | Tool list includes gate_evaluate with correct param schema | PASS |
| 5 | Evaluator prompt includes all sections | build_evaluator_prompt output contains spec name, criteria, diff, and evaluation guidance | PASS (4 prompt tests) |
| 6 | Lenient parse warns on unknown envelope fields | parse_evaluator_output returns warnings for unexpected top-level fields | PASS |
| 7 | Lenient parse extracts structured_output correctly | Valid Claude Code JSON envelope → parsed EvaluatorOutput | PASS |
| 8 | is_error flag produces Crash error | Envelope with is_error:true → EvaluatorError::Crash (with and without result) | PASS (2 tests) |
| 9 | map_evaluator_output produces correct pass/fail/skip/warn counts | 4-outcome fixture maps to 2 passed, 1 failed, 1 skipped | PASS |
| 10 | Warn outcome maps to passed=true + warning | Warn does not fail gate, warning captured separately | PASS (6 map tests) |

## Summary

All 10 tests pass. Full test suite: 788 passed, 3 ignored across 12 suites.
