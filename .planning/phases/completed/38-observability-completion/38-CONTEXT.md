# Phase 38: Observability Completion - Context

**Gathered:** 2026-03-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Enhance two existing MCP tools: `spec_get` gains a resolved config view showing timeout precedence and working_dir validation, and `estimate_tokens` gains growth rate metrics (avg tokens per turn, estimated turns remaining). No new MCP tools are introduced.

</domain>

<decisions>
## Implementation Decisions

### Resolved config shape (spec_get)
- Full cascade display: show all three tiers (spec, config, default) plus effective value
- Absent tiers use null values (not omitted) — cascade always has the same shape
- working_dir validation included: path, exists, accessible fields
- Example shape:
  ```json
  "resolved": {
    "timeout": {
      "effective": 300,
      "spec": 300,
      "config": null,
      "default": 300
    },
    "working_dir": {
      "path": "/some/path",
      "exists": true,
      "accessible": true
    }
  }
  ```

### Growth rate presentation (estimate_tokens)
- Growth rate metrics include avg_tokens_per_turn and estimated_turns_remaining
- When fewer than 5 assistant turns exist, growth rate metrics are absent (not zero) per success criteria — Claude decides the exact absence representation (null fields vs omitted section)
- Turn scope (assistant-only vs both directions), single estimate vs range, and 5-turn threshold configurability are Claude's discretion

### Tool response contracts
- Breaking changes are acceptable — can restructure existing fields if it improves overall shape
- Whether spec_get uses opt-in `resolve: true` parameter or always returns resolved config is Claude's discretion
- Whether growth_rate is top-level or nested is Claude's discretion
- Whether read-only tools include Phase 35 warnings field is Claude's discretion

### Claude's Discretion
- The three timeout tiers (determine from codebase what config layers currently exist)
- Whether `resolve` is opt-in parameter or always-on
- Growth rate nesting strategy (top-level vs nested object)
- Turns remaining: single integer vs range
- Turn counting scope (assistant-only vs bidirectional)
- Low-data absence strategy (null fields with reason vs omit section)
- 5-turn threshold: hardcoded vs configurable
- Warnings field on read-only tools (consistency vs convention)

</decisions>

<specifics>
## Specific Ideas

- Timeout cascade should always be the same shape regardless of which tiers have values — null for unset, never omit keys
- working_dir validation is a real filesystem check (exists + accessible), not just path resolution

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 38-observability-completion*
*Context gathered: 2026-03-13*
