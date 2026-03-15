# Phase 43 Plan 01: EvaluatorOutput Schema & Subprocess Summary

**One-liner:** EvaluatorOutput JSON schema in assay-types with 4-state CriterionOutcome, plus full core evaluator module — async subprocess spawning, lenient JSON parsing, and GateRunRecord mapping.

## Completed Tasks

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | EvaluatorOutput schema types and GatesConfig extension | 7c175f9 | `crates/assay-types/src/evaluator.rs`, `crates/assay-types/src/lib.rs` |
| 2 | Core evaluator module — subprocess, prompt, parsing, mapping | e83c05e | `crates/assay-core/src/evaluator.rs`, `crates/assay-core/src/error.rs`, `crates/assay-core/src/lib.rs` |

## What Was Built

### assay-types: EvaluatorOutput Schema
- `CriterionOutcome` enum: Pass/Fail/Skip/Warn with `#[serde(rename_all = "snake_case")]`
- `EvaluatorCriterionResult`: name, outcome, reasoning, evidence (optional)
- `EvaluatorSummary`: passed (bool), rationale (string)
- `EvaluatorOutput`: criteria vec + summary — used as `--json-schema` contract
- All types registered with `inventory::submit!` for schema generation
- `GatesConfig` extended with `evaluator_model` ("sonnet"), `evaluator_retries` (1), `evaluator_timeout` (120) — all with `serde(default)` for backward compatibility

### assay-core: Evaluator Module
- `EvaluatorConfig`: model, timeout (Duration), retries
- `EvaluatorResult`: output, duration, warnings
- `build_evaluator_prompt()`: structured prompt with spec, criteria, diff, agent context sections
- `build_system_prompt()`: concise evaluator behavior instructions
- `evaluator_schema_json()`: generates JSON Schema via `schemars::schema_for!(EvaluatorOutput)`
- `parse_evaluator_output()`: lenient two-phase parse — checks `is_error`, warns on unknown envelope fields, extracts `structured_output`
- `map_evaluator_output()`: converts 4-state outcomes to `GateRunRecord` with enforcement summary and warnings
- `run_evaluator()`: async subprocess with stdin piping, timeout via `tokio::time::timeout`, retry on crash/timeout

### Error Types
- `EvaluatorError` enum: Timeout, Crash, ParseError, NoStructuredOutput, NotInstalled
- `AssayError::Evaluator` variant wraps `EvaluatorError` with `#[source]`

## Decisions Made

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | Use `child.wait()` + separate stdout/stderr tasks instead of `wait_with_output()` | `wait_with_output` takes ownership, preventing `child.kill()` on timeout |
| 2 | Added `schemars` to assay-core dependencies | Needed for `schema_for!` macro in `evaluator_schema_json()` |
| 3 | Warn outcome maps to `passed: true` + warning string | Soft concerns should not fail gates; warnings surfaced separately |
| 4 | Default enforcement is `Required` when criterion not in map | Consistent with existing gate behavior — fail-safe default |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated existing GatesConfig struct literals**
- **Found during:** Task 1
- **Issue:** Adding fields to `GatesConfig` broke 6 existing struct literal constructions in test code across 3 crates
- **Fix:** Added the three new evaluator fields with default values to all existing struct literals
- **Files modified:** `crates/assay-mcp/src/server.rs`, `crates/assay-types/tests/schema_roundtrip.rs`, `crates/assay-core/src/config/mod.rs`

**2. [Rule 3 - Blocking] Updated insta schema snapshots**
- **Found during:** Task 1
- **Issue:** `GatesConfig` schema snapshots were outdated after adding evaluator fields
- **Fix:** Ran `INSTA_UPDATE=always` to regenerate snapshots
- **Files modified:** `crates/assay-types/tests/snapshots/schema_snapshots__config-schema.snap`, `crates/assay-types/tests/snapshots/schema_snapshots__gates-config-schema.snap`

**3. [Rule 1 - Bug] Fixed subprocess ownership pattern for kill-on-timeout**
- **Found during:** Task 2
- **Issue:** `tokio::process::Child::wait_with_output()` takes `self` by value, making it impossible to call `child.kill()` on timeout
- **Fix:** Restructured to use `child.wait()` with separate tokio tasks for reading stdout/stderr, preserving ownership of child for kill

## Verification

```
just ready → All checks passed
  fmt-check ✓
  lint ✓ (clippy -D warnings)
  test ✓ (786 passed, 3 ignored)
  deny ✓ (advisories, bans, licenses, sources)
```

## Metrics

- **Duration:** ~10 minutes
- **Completed:** 2026-03-15
- **Tests added:** 26 (6 in assay-types, 20 in assay-core)
- **Files created:** 2 (`crates/assay-types/src/evaluator.rs`, `crates/assay-core/src/evaluator.rs`)
- **Files modified:** 7 (lib.rs x2, error.rs, Cargo.toml, server.rs, schema_roundtrip.rs, config/mod.rs) + 2 snapshots

## Next Phase Readiness

Plan 02 can proceed immediately. The evaluator module provides:
- `evaluator_schema_json()` for `--json-schema` flag
- `build_evaluator_prompt()` and `build_system_prompt()` for prompt construction
- `run_evaluator()` for subprocess execution
- `parse_evaluator_output()` and `map_evaluator_output()` for result processing
- `EvaluatorConfig` for configuration
- All public APIs documented and tested
