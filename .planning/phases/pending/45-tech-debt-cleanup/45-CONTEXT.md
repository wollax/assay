# Phase 45: Tech Debt Cleanup - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Batch sweep of highest-value backlog issues from `.planning/issues/open/`. Prioritize issues that interact with v0.4.0 changes (phases 35-44: evaluator, sessions, context engine, MCP server, observability). Close as many as fit — minimum 10. Also triage stale/irrelevant issues as "won't fix". All resolved issues verified by `just ready`.

</domain>

<decisions>
## Implementation Decisions

### Issue selection criteria
- Primary filter: issues that touch code changed in v0.4.0 phases (35-44)
- Secondary: trivial issues from any area are fair game (missing derives, naming, etc.)
- Claude's discretion on discovery method (file-path matching vs reading issues vs hybrid)
- Claude's discretion on which issue categories to deprioritize or exclude

### Grouping strategy
- Group issues by module/crate (assay-types, assay-core, assay-mcp)
- Each plan must be independently shippable — passes `just ready` on its own
- Claude decides number of plans based on issue distribution across crates
- Closed issues moved to `.planning/issues/closed/` (not deleted)

### Resolution depth
- Surgical fixes only — fix exactly what the issue describes, minimal diff
- No surrounding code cleanup or refactoring of nearby code
- If an issue turns out more complex than expected, skip it and keep it open
- Claude decides per-issue whether to add tests (test gaps get tests, code fixes get regression tests at Claude's judgment)
- Claude decides whether API changes are acceptable (pre-1.0, breaking changes OK if warranted)

### Triage
- Review issues and close irrelevant/outdated ones as "won't fix" — reduces backlog noise
- Claude decides whether won't-fix issues go to `closed/` with a note or get deleted

### Claude's Discretion
- Discovery method for identifying v0.4.0-interacting issues
- Number of plans (driven by crate distribution)
- Whether to exclude doc-only or test-gap-only issues
- Per-issue test decisions
- API change tolerance
- Won't-fix disposition (move vs delete)

</decisions>

<specifics>
## Specific Ideas

- This is likely the final phase before v0.4.0 ships — treat it as a polish pass, but another phase could be added if needed
- Target is "as many as fit" rather than a fixed number — maximize cleanup while issues are straightforward
- The `.planning/issues/closed/` directory is available for resolved issues

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 45-tech-debt-cleanup*
*Context gathered: 2026-03-15*
