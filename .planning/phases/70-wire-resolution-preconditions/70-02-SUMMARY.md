---
phase: 70-wire-resolution-preconditions
plan: 02
subsystem: cli
tags: [rust, cli, gate, resolve, preconditions, composition, history]

requires:
  - phase: 70-01
    provides: PreconditionStatus::all_passed(), GateRunRecord::precondition_blocked Option<bool>
  - phase: 65-resolution-core
    provides: compose::resolve(), ResolvedCriterion, CriterionSource, ResolvedGate
  - phase: 66-evaluation-integration-validation
    provides: check_preconditions(), evaluate_all_resolved()
provides:
  - CLI gate run with full resolve+precondition+evaluate pipeline for Directory specs
  - source_tag display in streaming output ([Parent: slug], [Library: slug])
  - Exit code 2 for precondition-blocked runs
  - save_blocked_run() in assay-core history
  - Precondition-blocked history records with precondition_blocked: Some(true)
affects: [cli-consumers, mcp-gate-run, tui-gate-run]

tech-stack:
  added: []
  patterns:
    - "CLI gate run: resolve -> check_preconditions -> evaluate_all_resolved pipeline for Directory specs"
    - "Exit codes: 0=pass, 1=gate failure, 2=precondition blocked"
    - "CWD_LOCK mutex for serializing CWD-mutating integration tests"

key-files:
  created: []
  modified:
    - crates/assay-cli/src/commands/gate.rs
    - crates/assay-core/src/history/mod.rs

key-decisions:
  - "CWD_LOCK static Mutex used to serialize integration tests that set_current_dir — avoids parallel test data races"
  - "save_blocked_run() added to assay-core/history as a sibling of save_run() — cleaner than a local CLI helper with different signature"
  - "server.rs dead function save_precondition_blocked_run() removed (was referencing nonexistent anyhow dep) — Plan 70-03 commit already used assay_core::history::save_blocked_run directly"
  - "Streaming mode Directory path: uses resolved.criteria with Some(&rc.source) for source tags; Legacy path unchanged with None"

patterns-established:
  - "source_tag() helper: empty string for Own/None, ' [Parent: slug]' for Parent, ' [Library: slug]' for Library"
  - "save_precondition_blocked_record() CLI helper: calls history::save_blocked_run, non-fatal (warns on failure)"

requirements-completed: [INHR-02, INHR-04, CLIB-02, PREC-01, PREC-02, PREC-03]

duration: 45min
completed: 2026-04-13
---

# Phase 70 Plan 02: Wire Resolution Preconditions CLI Summary

**CLI gate run fully wired with compose::resolve + check_preconditions + evaluate_all_resolved for Directory specs, with source tags, exit code 2, and blocked history records**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-04-13T16:30:00Z
- **Completed:** 2026-04-13T17:15:00Z
- **Tasks:** 1 (TDD)
- **Files modified:** 2

## Accomplishments

- Wired `compose::resolve()` into both JSON and streaming paths of `handle_gate_run()` for Directory specs
- Wired `check_preconditions()` before evaluation in both `handle_gate_run()` and `handle_gate_run_all()`
- Wired `evaluate_all_resolved()` replacing `evaluate_all_gates()` for Directory specs
- Added `source_tag()` helper and updated `stream_criterion()` to display `[Parent: slug]` / `[Library: slug]` annotations
- Added `save_blocked_run()` to `assay-core/history` for atomic precondition-blocked history records
- Exit code 2 returned for precondition failures; `handle_gate_run_all` tracks `blocked_count` separately from gate failures
- Legacy spec path unchanged throughout; all existing `stream_criterion` callers pass `None` for source
- 9 new tests covering source_tag, save_precondition_blocked_record, and handle_gate_run integration scenarios

## Task Commits

1. **Task 1: Wire resolve + preconditions into handle_gate_run and handle_gate_run_all** - `436a1d9` (feat)

**Plan metadata:** (pending docs commit)

## Files Created/Modified

- `crates/assay-cli/src/commands/gate.rs` - Full pipeline for Directory specs, source_tag, stream_criterion updated, save_precondition_blocked_record helper, 9 integration tests
- `crates/assay-core/src/history/mod.rs` - Added save_blocked_run() public function

## Decisions Made

- CWD_LOCK static Mutex used to serialize integration tests that call `set_current_dir` — prevents data races when tests run in parallel
- `save_blocked_run()` added to assay-core/history as a sibling of `save_run()` — cleaner than a CLI-local helper with different signature
- A dead `save_precondition_blocked_run` function in server.rs (referencing unavailable `anyhow` crate) was removed as part of fixing a pre-existing Rule 3 blocker — the Plan 70-03 commit had already used `assay_core::history::save_blocked_run` directly in the call site

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed missing save_precondition_blocked_run in assay-mcp/server.rs**
- **Found during:** Task 1 (initial build)
- **Issue:** server.rs referenced `save_precondition_blocked_run()` (local function) which didn't exist and used `anyhow::Error` not in assay-mcp dependencies
- **Fix:** (a) Added `save_blocked_run()` to assay-core history (the function the plan asked for), (b) removed the dead stub in server.rs that was blocking compilation
- **Files modified:** crates/assay-core/src/history/mod.rs, crates/assay-mcp/src/server.rs
- **Verification:** `cargo build --workspace` passes; all tests pass
- **Committed in:** `436a1d9` (part of task commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Fix was required to unblock workspace compilation. No scope creep.

## Issues Encountered

- Test helper `setup_assay_project()` initially wrote config to `assay.toml` at root; fixed to `.assay/config.toml` with field `project_name` (as required by `assay_core::config::load`)
- CWD-mutating integration tests were racing in parallel — resolved with `CWD_LOCK` static mutex

## Next Phase Readiness

- All CLI gate run paths now use the full composition + precondition + evaluation pipeline
- Requirements INHR-02, INHR-04, CLIB-02, PREC-01, PREC-02, PREC-03 complete
- Phase 70 complete — v0.7.0 gap-closure phases fully wired

---
*Phase: 70-wire-resolution-preconditions*
*Completed: 2026-04-13*
