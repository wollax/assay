---
plan: 45-06
wave: 2
status: complete
issues_resolved: 13
commits:
  - fix(45-06): path validation, error context, and recovery improvements
  - test(45-06): session/recovery test coverage — 8 new test groups
  - docs(45-06): complete session/recovery sweep plan
---

# Plan 45-06 Summary: Session/Recovery Sweep

## Objective

Harden session management and recovery code in `assay-core`: path validation,
error handling correctness, and test coverage expansion.

## Changes Applied

### Task 1: Code Fixes (5 issues)

**`load_session` path traversal guard** (`load-session-validate-path`)
- Added `validate_path_component(session_id, "session ID")` before constructing
  the file path, matching the guard already present in `save_session`.

**`save_session` error context** (`save-session-json-error-context`)
- Moved `final_path` declaration before serialization so all error paths
  (JSON serialization, write, sync) reference the actual file path, not the
  sessions directory.
- JSON error operation string now includes session ID:
  `"serializing work session {id}"`.

**Checkpoint timestamp write** (`checkpoint-timestamp-silent-write`)
- Replaced `let _ = std::fs::write(...)` with a `tracing::warn!` on failure,
  making timestamp write errors visible in logs.

**`previous_phase` closure capture** (`previous-phase-capture-fragile`)
- Refactored `session_update` in `assay-mcp` to load the session explicitly
  before calling `with_session`, capturing `previous_phase` as a plain `let`
  binding rather than via a closure side effect.

**`RecoverySummary.truncated` field** (`recovery-summary-truncated-field`)
- Added `pub truncated: bool` to `RecoverySummary`.
- Introduced `RECOVERY_CAP: usize = 100` constant.
- Set `summary.truncated = ids.len() > RECOVERY_CAP` before the scan loop.

### Task 2: Test Additions (8 issues, 17+ new tests)

**`load_session` path validation** — two tests: `..` traversal and `/` in ID.

**`list_sessions` non-JSON filter** — writes `.txt`, no-extension, and `.bak`
files; asserts only `.json` sessions appear in results.

**`full_lifecycle_transition_fields`** — per-transition field assertions
(from, to, trigger, notes, timestamps) plus monotonically non-decreasing
timestamp check and round-trip persistence verification.

**Convenience function error paths** — `record_gate_result` on a completed
session and `complete_session` from `AgentRunning` both return
`WorkSessionTransition` errors without mutating on-disk state.

**`recover_skips_non_agent_running_all_assertions`** — tightened existing test
with `skipped == 0`, `errors == 0`, and `truncated == false` assertions.

**Recovery scan cap** — two tests: 101 sessions → `recovered == 100` and
`truncated == true`; 3 sessions → `truncated == false`.

**Stale threshold behavior** — 30-minute-old session (fresh) vs. 2-hour-old
session (stale) with 1h threshold; verifies only the stale session is recovered.

**`SessionPhase` deserialization** (in `assay-types`) — deserialize-from-string
for all five variants, full round-trip, and unknown variant error test.

**`PhaseTransition` notes `Some`** (in `assay-types`) — serialization includes
the `notes` key and value; round-trip preserves the notes field.

## Test Count

- `assay-core`: 539 → 556 tests (+17)
- `assay-types`: 70 → 76 tests (+6)

## Issues Resolved

13 issues moved to `.planning/issues/closed/`:
1. `load-session-validate-path`
2. `save-session-json-error-context`
3. `checkpoint-timestamp-silent-write`
4. `previous-phase-capture-fragile`
5. `recovery-summary-truncated-field`
6. `convenience-fn-error-paths-untested`
7. `list-sessions-non-json-filter-test`
8. `full-lifecycle-transition-fields`
9. `session-phase-deserialization-tests`
10. `phase-transition-notes-some-test`
11. `recover-skips-non-running-skipped-assert`
12. `recovery-scan-cap-untested`
13. `load-recovery-threshold-untested`

## Verification

`just ready` passes: fmt-check, lint, all tests (assay-core 556, assay-types 76),
cargo-deny.
