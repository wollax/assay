# Phase 35: Observability Foundation - Context

**Gathered:** 2026-03-10
**Status:** Ready for planning

<domain>
## Phase Boundary

Add a `warnings` field to mutating MCP tool responses to surface degraded-but-succeeded operations (e.g., history save failures, diff capture failures, cleanup issues). Add outcome filtering and limit parameter to `gate_history`. Close the history-save-failure-not-surfaced issue via the warnings mechanism.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

All three areas were discussed and the user delegated implementation decisions to Claude. The following design space is open:

**Warning semantics:**
- Whether warnings cover degraded-only or also advisory notices — pick what feels natural
- Whether warnings carry optional recovery hints or are informational-only
- Whether to cap warnings per response or return all
- Which mutating tools get the warnings field — gate tools only or all mutating tools proactively

**Response shape:**
- How warnings integrate into existing MCP response JSON (top-level field vs wrapper envelope)
- Whether individual warnings are plain strings or structured objects
- Whether the field is always present (empty array) or absent when no warnings
- Whether the field appears in JSON Schema for tool discovery

**History filter defaults:**
- Default outcome filter when none specified
- Sort order (newest-first vs oldest-first)
- Empty result behavior (bare empty array vs informational message)
- Whether limit=0 is valid or minimum is 1

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. The success criteria in the roadmap define the hard constraints:
- Mutating MCP tool responses include `warnings` field
- `gate_history` accepts `outcome` parameter (passed/failed/any)
- `gate_history` accepts `limit` parameter (default 10, max 50)
- History save failure issue is closed via warnings field

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 35-observability-foundation*
*Context gathered: 2026-03-10*
