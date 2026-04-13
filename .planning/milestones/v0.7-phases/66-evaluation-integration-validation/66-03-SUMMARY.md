---
phase: 66-evaluation-integration-validation
plan: "03"
subsystem: validation
tags: [rust, validation, diagnostics, composability, slug, preconditions, SAFE-01, SAFE-02]

# Dependency graph
requires:
  - phase: 65-resolution-core
    provides: "compose::validate_slug, compose::resolve, compose::load_library_by_slug, compose::scan_libraries"
  - phase: 66-01
    provides: "GateEvalOutcome, CriterionResult.source, last_gate_passed()"
provides:
  - "validate_spec_with_dependencies() with composability and precondition diagnostics"
  - "SAFE-01: missing parent gate error, missing library error, cycle detection error"
  - "SAFE-02: path-traversal slug validation for extends/include"
  - "Shadow warning: own criterion overrides parent with same name"
  - "Precondition reference warnings: missing spec, self-reference, empty command"
affects:
  - 66-evaluation-integration-validation
  - assay-cli spec validate command
  - assay-mcp spec_validate tool

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "assay_dir: Option<&Path> parameter pattern for backward-compatible signature extension"
    - "Separate private validation helper functions (validate_composability, validate_extends_existence_and_cycle, validate_precondition_refs)"
    - "extract_slug_reason() helper to extract reason string from AssayError::InvalidSlug"

key-files:
  created: []
  modified:
    - crates/assay-core/src/spec/validate.rs
    - crates/assay-cli/src/commands/spec.rs
    - crates/assay-mcp/src/server.rs

key-decisions:
  - "assay_dir: Option<&Path> parameter added to validate_spec_with_dependencies() — None skips composability checks for backward compat"
  - "Composability checks only apply to SpecEntry::Directory (not Legacy); Legacy specs have no extends/include/preconditions"
  - "Cycle detection in extends uses direct mutual-extend check (not full DFS) — consistent with compose::resolve() semantics"
  - "Precondition requires missing spec is warning (not error) — spec might be created later"
  - "Shadow warning location uses criteria.shadow.<name> format for clear identification"
  - "Empty include vec produces no warning (indistinguishable from omitted after deserialization)"

patterns-established:
  - "Pattern: Optional assay_dir parameter gates I/O-dependent validation without breaking existing callers"
  - "Pattern: Private helper functions for each validation concern (extends, preconditions) keep validate_composability readable"

requirements-completed: [SAFE-01, SAFE-02]

# Metrics
duration: 13min
completed: 2026-04-12
---

# Phase 66 Plan 03: Composability and Precondition Diagnostics Summary

**validate_spec_with_dependencies() extended with SAFE-01/SAFE-02 composability diagnostics: slug validation, missing parent/library errors, cycle detection, shadow warnings, and precondition reference warnings**

## Performance

- **Duration:** 13 min
- **Started:** 2026-04-12T00:12:43Z
- **Completed:** 2026-04-12T00:26:20Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Extended `validate_spec_with_dependencies()` with optional `assay_dir: Option<&Path>` parameter enabling composability checks without breaking existing callers
- SAFE-02: Slug validation for `extends` and `include[i]` fields rejects path-traversal attempts (`../evil`) before any file I/O
- SAFE-01: Missing parent gate and missing criteria library produce error diagnostics with fuzzy suggestions
- SAFE-01: Mutual extends cycle detection (`A extends B, B extends A`) produces error diagnostic
- Shadow warning when own criterion overrides parent criterion with same name
- Precondition reference warnings for missing required spec, self-reference, and empty command strings
- All 3 callers updated (assay-cli handle_spec_validate, assay-cli validate_and_exit_code test helper, assay-mcp spec_validate)
- 10 new composability/precondition tests; all 37 validate tests pass; `just ready` 2399/2399 tests pass

## Task Commits

1. **Task 1: Composability and precondition diagnostics** - `a3edab3` (feat)
   - Includes TDD: tests written, implementation added, all 10 new tests pass, all callers updated
2. **Task 2: Update callers and verify workspace** - (no separate commit; callers updated in Task 1; `just ready` verified)

**Plan metadata:** (this SUMMARY.md commit)

## Files Created/Modified
- `crates/assay-core/src/spec/validate.rs` — Extended `validate_spec_with_dependencies()`, added `validate_composability()`, `validate_extends_existence_and_cycle()`, `validate_precondition_refs()`, `extract_slug_reason()`, 10 new tests
- `crates/assay-cli/src/commands/spec.rs` — Updated 2 callers to pass `assay_dir`
- `crates/assay-mcp/src/server.rs` — Updated MCP spec_validate handler to pass `assay_dir`

## Decisions Made
- Used `Option<&Path>` for backward compat — `None` skips composability checks entirely, safe for callers without `assay_dir`
- Composability checks only apply to `SpecEntry::Directory` (Legacy specs have no extends/include fields)
- Cycle detection uses direct mutual-extend check matching `compose::resolve()` behavior, not full DFS
- `include` is `Vec<String>` not `Option<Vec<String>>` — empty vec is fine, no "empty includes" warning
- Missing required spec in preconditions is warning (not error) — the referenced spec might be created later
- Shadow warning location: `criteria.shadow.<name>` for clear scannability

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Resolved pre-existing incomplete 66-02 gate/mod.rs state**
- **Found during:** Task 1 (RED phase — trying to commit failing tests)
- **Issue:** gate/mod.rs had partial 66-02 changes staged (evaluate_criteria signature change + tests calling non-existent functions) that prevented pre-commit hook from passing
- **Fix:** Discovered 66-02 was actually already committed (`0b568b5`, `4ffe20b`) with full implementation — the staged state was from a prior session that ran cargo fmt, which had been reverted by the linter. The functions `check_preconditions` and `evaluate_all_resolved` existed in HEAD.
- **Files modified:** None — confirmed state was already correct
- **Verification:** `cargo test -p assay-core --lib gate` passed 209 tests
- **Committed in:** Already in `0b568b5`

---

**Total deviations:** 1 auto-investigated (Rule 3 - blocking)
**Impact on plan:** Investigation confirmed correct state. No changes needed. No scope creep.

## Issues Encountered
- Cargo fmt background linter repeatedly reverted in-progress file edits between Edit tool calls and Bash commands, requiring careful sequencing (edits before any cargo invocation)
- The `include` field is `Vec<String>` not `Option<Vec<String>>`, required adjusting the composability check code

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- SAFE-01 and SAFE-02 requirements complete
- validate_spec_with_dependencies() now provides full composability diagnostics at authoring time
- Phase 66 all 3 plans complete; ready for phase completion

---
*Phase: 66-evaluation-integration-validation*
*Completed: 2026-04-12*
