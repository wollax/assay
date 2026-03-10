# Phase 30: Core Tech Debt - Context

**Gathered:** 2026-03-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Eliminate validation duplication, extract shared evaluation logic, harden history and daemon persistence, tighten visibility on internal APIs, and surface spec parse errors. Seven requirements (CORE-02, CORE-03, CORE-04, CORE-06, CORE-07, CORE-08, CORE-09) — all internal refactoring within `assay-core`. No new capabilities, no public API additions.

</domain>

<decisions>
## Implementation Decisions

### Warning Surface (CORE-04, CORE-09)
- Claude's discretion on warning mechanism (eprintln!, tracing, or whatever fits existing patterns)
- Claude's discretion on whether warnings are suppressible or always visible
- Claude's discretion on whether to list each skipped entry or use a summary count (CORE-04)
- Claude's discretion on partial failure behavior for spec parsing (CORE-09) — continue with valid specs or abort

### Dedup Pattern (CORE-02, CORE-03)
- Claude's discretion on extraction pattern (shared helper, delegation, or consolidation)
- Claude's discretion on whether to preserve both public API names or consolidate if one is redundant
- Claude's discretion on standalone function vs method for evaluation extraction (CORE-03)
- Claude's discretion on whether to add equivalence tests or rely on existing coverage

### Visibility (CORE-06)
- No external consumers of assay-core — free to tighten any public API without semver concern
- Beyond CORE-06: if Claude finds other trivially-tightenable `pub` → `pub(crate)` items while working, fix them too
- Claude's discretion on daemon test approach for CORE-07/CORE-08 fixes

### Claude's Discretion
All three areas were delegated to Claude's judgment. Key latitude:
- Warning mechanism and verbosity approach
- Deduplication pattern and API consolidation strategy
- Test coverage depth for daemon fixes
- Opportunistic visibility tightening beyond CORE-06

</decisions>

<specifics>
## Specific Ideas

- No external consumers of assay-core exist — only assay-cli, assay-tui, and assay-mcp within the workspace
- Opportunistic `pub` → `pub(crate)` tightening is welcome if trivial (don't expand scope significantly)

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 30-core-tech-debt*
*Context gathered: 2026-03-09*
