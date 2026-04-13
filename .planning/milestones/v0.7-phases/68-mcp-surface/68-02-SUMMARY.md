---
phase: 68-mcp-surface
plan: 02
subsystem: mcp
tags: [rust, rmcp, mcp, criteria-list, criteria-get, spec-resolve, spawn-blocking, shadow-detection]

# Dependency graph
requires:
  - phase: 68-01
    provides: gate_wizard and criteria_create MCP tool patterns in server.rs
  - phase: 65-resolution-core
    provides: scan_libraries, load_library_by_slug, resolve() in assay_core::spec::compose

provides:
  - criteria_list MCP tool handler with CriteriaListResponse struct in server.rs
  - criteria_get MCP tool handler with CriteriaGetResponse struct in server.rs
  - spec_resolve MCP tool handler with SpecResolveResponse struct, shadow detection logic in server.rs
  - 9 integration tests covering tool registration, success paths, not-found errors, and shadow warnings

affects:
  - any future MCP surface plans that build on the same handler pattern
  - agents consuming the MCP surface (all three tools now callable)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "spec_resolve pre-loads inherited criterion names before calling resolve() — shadow detection without modifying compose::resolve signature"
    - "Legacy spec format returns CallToolResult::error directly (no AssayError variant needed) for non-domain format rejection"
    - "criteria_list uses no-params handler pattern (same as spec_list)"

key-files:
  created: []
  modified:
    - crates/assay-mcp/src/server.rs

key-decisions:
  - "Tests and implementation committed together (single feat commit) — pre-commit hook runs clippy which requires the implementation to exist for test code to compile"
  - "Legacy spec format rejection uses CallToolResult::error directly (no AssayError::Other variant) — avoids adding a catch-all variant to the error enum"
  - "Shadow detection pre-loads parent/library criterion names via separate calls before resolve(), then post-checks Own criteria for name collisions — consistent with CONTEXT.md design"
  - "GatesSpec TOML in tests requires name field (required, no default) — test TOML must include name = \"slug\" to pass validation"

patterns-established:
  - "criteria_list + criteria_get follow the exact same spawn_blocking handler shape as gate_wizard and criteria_create"
  - "spec_resolve is the most complex tool: entry-format check -> shadow pre-load -> resolve() -> shadow post-check -> response"

requirements-completed:
  - WIZM-02
  - WIZM-03

# Metrics
duration: 7min
completed: 2026-04-13
---

# Phase 68 Plan 02: MCP Surface Summary

**criteria_list, criteria_get, and spec_resolve MCP tools with CriterionSource annotations, shadow warnings, and 9 integration tests**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-04-13T02:16:14Z
- **Completed:** 2026-04-13T02:23:18Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- criteria_list MCP handler: scans `.assay/criteria/` via `compose::scan_libraries()`, returns slug + criterion_count + description per library; handles empty state gracefully
- criteria_get MCP handler: loads single library by slug via `compose::load_library_by_slug()`, returns full CriteriaLibrary; domain error with fuzzy suggestion on not-found
- spec_resolve MCP handler: resolves directory-format spec via `compose::resolve()` with per-criterion CriterionSource annotations; shadows own criteria that override inherited names (parent or library); domain error for legacy-format specs
- Module doc comment updated with all three new tool entries
- 210 total tests pass (up from 201), no clippy warnings, `just ready` passes

## Task Commits

Tests and implementation were committed together due to pre-commit hook requiring compilation:

1. **Tasks 1+2: criteria_list + criteria_get + spec_resolve handlers + tests** - `d47c3a3` (feat)

## Files Created/Modified

- `crates/assay-mcp/src/server.rs` - Added CriteriaGetParams + SpecResolveParams param structs; CriteriaListEntry + CriteriaListResponse + CriteriaGetResponse + SpecResolveResponse response structs; criteria_list + criteria_get + spec_resolve handlers; 9 integration tests; updated module doc comment

## Decisions Made

- Tests and implementation committed in a single feat commit: the pre-commit hook runs `cargo clippy --workspace -D warnings`, which requires the implementation to exist for the test code to compile. (Same pattern established in Phase 68 Plan 01.)
- Legacy spec format rejection uses `CallToolResult::error` directly rather than constructing an `AssayError` variant, avoiding a catch-all `Other(String)` variant in the error enum.
- Shadow detection pre-loads inherited criterion names before calling `compose::resolve()`, then post-checks Own-sourced criteria — this avoids modifying the `resolve()` signature (consistent with CONTEXT.md design decision).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] GatesSpec TOML in tests missing required `name` field**
- **Found during:** Task 2 (spec_resolve_returns_resolved_gate and spec_resolve_shadow_warnings tests)
- **Issue:** Test TOML strings for gates.toml omitted the required `name` field; `validate_gates_spec()` requires it; tests failed with "parsing gates spec" errors
- **Fix:** Added `name = "slug-value"` to all test gates.toml TOML strings
- **Files modified:** crates/assay-mcp/src/server.rs
- **Committed in:** d47c3a3 (Task 1+2 commit)

**2. [Rule 1 - Bug] Nested `if` in shadow detection raised clippy error**
- **Found during:** Post-task clippy run
- **Issue:** `if matches!(...) { if ... { ... } }` triggered `clippy::collapsible_if`
- **Fix:** Collapsed into single `if matches!(...) && ...` condition
- **Files modified:** crates/assay-mcp/src/server.rs
- **Committed in:** d47c3a3 (Task 1+2 commit)

**3. [Rule 1 - Bug] Formatting issues in test path construction**
- **Found during:** `just ready` fmt-check
- **Issue:** Multi-line `.join()` chains in test code required collapsing to single line by rustfmt
- **Fix:** `cargo fmt --all` applied automatically
- **Files modified:** crates/assay-mcp/src/server.rs
- **Committed in:** d47c3a3 (Task 1+2 commit)

---

**Total deviations:** 3 auto-fixed (all Rule 1 - bug)
**Impact on plan:** All auto-fixes were necessary for correctness (test format), code quality (clippy), and style consistency (rustfmt). No scope creep.

## Issues Encountered

None beyond the auto-fixed deviations above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All five Phase 68 MCP tools are now callable: gate_wizard, criteria_create, criteria_list, criteria_get, spec_resolve
- Phase 68 is complete — all planned MCP surface tools implemented
- Ready to proceed to Phase 69 or milestone wrap-up

---
*Phase: 68-mcp-surface*
*Completed: 2026-04-13*
