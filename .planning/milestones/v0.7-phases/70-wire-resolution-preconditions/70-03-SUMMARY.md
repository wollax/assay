---
phase: 70-wire-resolution-preconditions
plan: 03
subsystem: mcp
tags: [mcp, gate, compose, resolve, preconditions, source-annotations]

# Dependency graph
requires:
  - phase: 70-01
    provides: GateEvalOutcome type, save_blocked_run, check_preconditions, evaluate_all_resolved, PreconditionStatus

provides:
  - MCP gate_run handler using compose::resolve for Directory specs
  - MCP gate_run handler checking preconditions before evaluation
  - CriterionSummary with source/source_detail fields from CriterionResult.source
  - PreconditionFailedResponse with outcome=precondition_failed for agent disambiguation
  - GateRunResponse with outcome=evaluated for agent disambiguation
  - Precondition-blocked runs saved to history with precondition_blocked=true

affects:
  - MCP server consumers (agents calling gate_run)
  - Any test that snapshots gate_run JSON output

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "GateEvalOutcome dispatch pattern: match PreconditionFailed first, then Evaluated"
    - "source_fields closure inside format_gate_response mapping CriterionResult.source to JSON strings"
    - "spawn_blocking closure returns assay_core::Result<GateEvalOutcome> — infallible for Legacy, fallible for Directory (resolve can fail)"

key-files:
  created: []
  modified:
    - crates/assay-mcp/src/server.rs
    - crates/assay-mcp/src/snapshots/assay_mcp__server__tests__gate_run_command_spec.snap

key-decisions:
  - "PreconditionFailedResponse returned as CallToolResult::success (not error) — agents disambiguate via outcome field string"
  - "outcome field added to GateRunResponse (always 'evaluated') to pair with precondition_failed outcome"
  - "CriterionSource::Own maps to source='own' with source_detail=None; None maps to both None (legacy path)"
  - "save_precondition_blocked_run delegates to assay_core::history::save_blocked_run — avoids duplicating run ID generation"
  - "config.specs_dir cloned before spawn_blocking closure to avoid partial move of config"
  - "Inline save_blocked_run call in gate_run (no wrapper function) — save_blocked_run is already a clean API"

patterns-established:
  - "Gate evaluation pipeline for Directory specs: resolve -> check_preconditions -> evaluate_all_resolved"
  - "Pre-existing formatting fixes committed with the triggering change (pre-commit hook runs fmt --all)"

requirements-completed: [INHR-02, INHR-04, CLIB-02, PREC-01, PREC-02, PREC-03]

# Metrics
duration: 9min
completed: 2026-04-13
---

# Phase 70 Plan 03: Wire Resolution Preconditions into MCP gate_run Summary

**MCP gate_run now uses compose::resolve + check_preconditions pipeline for Directory specs, returning source-annotated criteria and structured precondition_failed responses**

## Performance

- **Duration:** 9 min
- **Started:** 2026-04-13T16:28:05Z
- **Completed:** 2026-04-13T16:37:20Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- CriterionSummary gains source/source_detail fields populated from CriterionResult.source (parent gate slug or library slug)
- GateRunResponse gains outcome="evaluated" field for agent-side disambiguation
- gate_run handler for Directory specs now calls compose::resolve then check_preconditions then evaluate_all_resolved
- Precondition failures return PreconditionFailedResponse (outcome=precondition_failed) as a successful MCP tool result
- Blocked runs saved to history with precondition_blocked=true via assay_core::history::save_blocked_run

## Task Commits

Each task was committed atomically:

1. **Task 1: Add source field to CriterionSummary and wire format_gate_response** - `fb85114` (feat)
2. **Task 2: Wire resolve + preconditions into gate_run handler** - `e0af3ac` (feat)

**Plan metadata:** (docs commit follows)

_Note: Tests and implementation committed together per project convention — pre-commit hook runs clippy which requires implementation to compile test code._

## Files Created/Modified

- `/Users/wollax/Git/personal/assay/crates/assay-mcp/src/server.rs` - CriterionSummary source fields, GateRunResponse outcome field, PreconditionFailedResponse struct, gate_run resolve+preconditions pipeline, 5 source annotation tests, 5 gate_run integration tests
- `/Users/wollax/Git/personal/assay/crates/assay-mcp/src/snapshots/assay_mcp__server__tests__gate_run_command_spec.snap` - Updated snapshot to include outcome field

## Decisions Made

- PreconditionFailedResponse returned as `CallToolResult::success` (not error) — agents must inspect `outcome` field to distinguish from normal evaluation
- `outcome` added to `GateRunResponse` (always `"evaluated"`) so agents can distinguish without field-presence checks
- `CriterionSource::None` maps to both `source=None` and `source_detail=None` (legacy specs and non-resolved paths produce no source)
- Inline `assay_core::history::save_blocked_run` call rather than a wrapper function — the existing API is already clean
- `config.specs_dir` cloned before `spawn_blocking` closure to avoid partial move of the `config` struct

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Updated gate_run_command_spec insta snapshot**
- **Found during:** Task 2 (gate_run handler changes)
- **Issue:** Adding `outcome` field to GateRunResponse caused existing snapshot test to fail
- **Fix:** Updated snapshot to include `"outcome": "evaluated"` in expected JSON
- **Files modified:** crates/assay-mcp/src/snapshots/assay_mcp__server__tests__gate_run_command_spec.snap
- **Verification:** `cargo test -p assay-mcp -- gate_run` passes
- **Committed in:** e0af3ac (Task 2 commit)

**2. [Rule 1 - Bug] Updated existing test struct initializations**
- **Found during:** Task 1 (adding source/source_detail to CriterionSummary)
- **Issue:** Several existing test-only struct literals missing newly-required source/source_detail/outcome fields
- **Fix:** Added `source: None, source_detail: None` to all CriterionSummary literals; added `outcome: "evaluated".to_string()` to all GateRunResponse test literals
- **Files modified:** crates/assay-mcp/src/server.rs
- **Verification:** `cargo test -p assay-mcp` passes (220 tests)
- **Committed in:** fb85114 (Task 1 commit)

**3. [Rule 3 - Blocking] Removed anyhow dependency usage**
- **Found during:** Task 2 (save_precondition_blocked_run helper)
- **Issue:** Used `anyhow::Error` in return type but anyhow is not in assay-mcp/Cargo.toml
- **Fix:** Switched closure return type to `assay_core::Result<GateEvalOutcome>`; inline save_blocked_run call avoids needing a helper function with a foreign error type
- **Files modified:** crates/assay-mcp/src/server.rs
- **Verification:** `cargo check -p assay-mcp` passes
- **Committed in:** e0af3ac (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (2 Rule 1 bugs, 1 Rule 3 blocking)
**Impact on plan:** All fixes necessary for correctness and compilation. No scope creep.

## Issues Encountered

- `save_precondition_blocked_run` helper function was partially eaten by the file linter during a failed compile. The linter strips the function body when there's a syntax error in the file. Resolved by checking git status and re-writing the approach inline.
- Pre-commit hook runs `cargo fmt --all -- --check` which caught formatting issues in pre-existing unrelated files (assay-cli/src/commands/gate.rs). Resolved by running `cargo fmt --all` to normalize the entire workspace.

## Next Phase Readiness

- MCP gate_run fully supports extends/include/preconditions with correct response shapes
- All 6 requirements (INHR-02, INHR-04, CLIB-02, PREC-01, PREC-02, PREC-03) are complete
- Phase 70 implementation complete — STATE.md, ROADMAP.md updates follow

## Self-Check: PASSED

- FOUND: crates/assay-mcp/src/server.rs
- FOUND: crates/assay-mcp/src/snapshots/assay_mcp__server__tests__gate_run_command_spec.snap
- FOUND: .planning/phases/70-wire-resolution-preconditions/70-03-SUMMARY.md
- FOUND commit: fb85114 (Task 1)
- FOUND commit: e0af3ac (Task 2)
