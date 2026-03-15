# Phase 43 Verification Report

**Phase**: 43 — gate_evaluate Schema & Subprocess
**Status**: PASSED
**Verified**: 2026-03-15
**Build**: `just test` → 786 passed, 3 ignored (all clean)

---

## ORCH-01: gate_evaluate evaluates agent criteria in single call

**Status: VERIFIED**

`crates/assay-mcp/src/server.rs` exposes `gate_evaluate` as an MCP tool (line 1211). The handler:
1. Loads config and spec (Step 1)
2. Resolves working directory (Step 2)
3. Computes `git diff HEAD` (Step 3)
4. Builds evaluator prompt (Step 4)
5. Spawns headless Claude Code subprocess via `run_evaluator()` (Step 7)
6. Maps output to `GateRunRecord` (Step 8)
7. Persists via `history::save` (Step 9)
8. Optionally links session (Step 10)

All criteria evaluated in one subprocess call, result persisted before returning.

---

## ORCH-02: Subprocess model — parent parses JSON, evaluator never calls MCP

**Status: VERIFIED**

`crates/assay-core/src/evaluator.rs` spawns subprocess with:
```
--tools ""
--max-turns 1
--no-session-persistence
--output-format json
--json-schema <schema>
```

All parsing (`parse_evaluator_output`), mapping (`map_evaluator_output`), and persistence (`history::save`) happen in the parent process. The evaluator subprocess has no MCP tool access via `--tools ""`.

---

## ORCH-03: EvaluatorOutput JSON schema with lenient parsing

**Status: VERIFIED**

`crates/assay-types/src/evaluator.rs` defines:
- `CriterionOutcome` (Pass/Fail/Skip/Warn) — derives Serialize, Deserialize, JsonSchema, snake_case
- `EvaluatorCriterionResult` — with name, outcome, reasoning, optional evidence
- `EvaluatorSummary` — with passed bool and rationale
- `EvaluatorOutput` — with criteria vec and summary

All registered in the schema registry via `inventory::submit!`.

`parse_evaluator_output()` uses two-phase lenient parse:
1. Outer envelope parsed as `serde_json::Value` — unknown fields produce warnings, not errors
2. `structured_output` field extracted and deserialized into typed `EvaluatorOutput`
3. `is_error` flag checked before extraction

---

## Plan 01 Must-Haves

| Requirement | Status | Evidence |
|---|---|---|
| EvaluatorOutput schema in assay-types | PASS | `crates/assay-types/src/evaluator.rs` |
| CriterionOutcome (Pass/Fail/Skip/Warn) | PASS | Lines 13-24 |
| EvaluatorCriterionResult struct | PASS | Lines 34-45 |
| EvaluatorSummary struct | PASS | Lines 55-61 |
| EvaluatorOutput struct | PASS | Lines 74-80 |
| All derive Serialize, Deserialize, JsonSchema | PASS | Confirmed on each struct |
| GatesConfig.evaluator_model (default "sonnet") | PASS | `crates/assay-types/src/lib.rs` lines 198-199, 299 |
| GatesConfig.evaluator_retries (default 1) | PASS | Lines 202-203, 303 |
| GatesConfig.evaluator_timeout (default 120) | PASS | Lines 205-206, 307 |
| async run_evaluator() in assay-core | PASS | `crates/assay-core/src/evaluator.rs` line 356 |
| Subprocess args: --json-schema, --tools "", --max-turns 1, stdin prompt | PASS | Lines 415-434 |
| Lenient parse: serde_json::Value intermediate, warn on unknown fields | PASS | Lines 155-202 |
| is_error flag check → EvaluatorError::Crash | PASS | Lines 163-177 |
| build_evaluator_prompt from spec/criteria/diff/agent_prompt | PASS | Lines 80-132 |
| map_evaluator_output: CriterionOutcome → GateRunRecord | PASS | Lines 215-301 |
| EvaluatorError: Timeout, Crash, ParseError, NoStructuredOutput, NotInstalled | PASS | `crates/assay-core/src/error.rs` lines 10-46 |
| Unit tests for parse and mapping logic | PASS | 516-950 in evaluator.rs |

---

## Plan 02 Must-Haves

| Requirement | Status | Evidence |
|---|---|---|
| gate_evaluate accepts name, optional session_id, optional timeout, optional model | PASS | `GateEvaluateParams` struct (lines 353-388 in server.rs) |
| Loads spec, resolves working_dir, computes git diff | PASS | Steps 1-3 in handler (lines 1219-1301) |
| Spawns evaluator subprocess | PASS | Step 7 (lines 1348-1409) |
| Parses output, persists GateRunRecord | PASS | Steps 8-9 (lines 1414-1477) |
| session_id: loads WorkSession, uses worktree_path for diff | PASS | Lines 1253-1269 |
| session_id: transitions session to GateEvaluated via record_gate_result | PASS | Lines 1480-1506 |
| Subprocess invoked with --tools "" and --max-turns 1 | PASS | `spawn_and_collect()` lines 424-428 |
| Response includes per-criterion results | PASS | `EvaluateCriterionResult` (lines 663-677), populated lines 1437-1459 |
| Response includes warnings field | PASS | `GateEvaluateResponse.warnings` (lines 658-661) |
| Tool count in server module doc updated to "eighteen" | PASS | server.rs line 3: "exposes eighteen tools over MCP" |

---

## Test Coverage

All unit tests run in-process (no subprocess spawning required):
- `evaluator.rs` tests: schema generation, prompt construction, parse_evaluator_output (6 cases), map_evaluator_output (7 cases), EvaluatorError display
- `assay-types/evaluator.rs` tests: serde roundtrip, snake_case serialization, evidence omission

Build: 786 tests passed, 3 ignored, 0 failures.
