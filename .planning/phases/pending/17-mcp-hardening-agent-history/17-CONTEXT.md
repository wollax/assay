# Phase 17: MCP Hardening & Agent History - Context

**Gathered:** 2026-03-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Harden the MCP tool surface with timeout support, path validation, error handling, and documentation. Expose gate history to agents via a new `gate_history` tool. Add enforcement-level awareness to `gate_run` responses. No new gate types, no new evaluation logic — this phase improves the existing MCP surface.

</domain>

<decisions>
## Implementation Decisions

### Timeout behavior
- Claude's Discretion: agent-controlled vs server-enforced timeout model
- Claude's Discretion: handling of partial results when timeout occurs (return partial vs fail whole run)
- Claude's Discretion: per-criterion vs per-gate-run timeout scope
- Claude's Discretion: whether timed-out runs are persisted to history

### Error responses
- Claude's Discretion: spec_list error presentation (inline errors vs separate array)
- Claude's Discretion: whether working_dir validation errors include the attempted path
- Claude's Discretion: structured vs free-form error shape consistency
- Claude's Discretion: documentation style — fill gaps and fix inaccuracies, matching existing patterns or comprehensive rewrite

### History query design
- Claude's Discretion: response depth (summaries vs full records vs configurable)
- Claude's Discretion: filtering surface (limit-only vs limit+status vs rich query)
- Claude's Discretion: whether spec name is required or optional
- Claude's Discretion: optimize for both progress tracking and regression detection use cases

### Enforcement in responses
- Claude's Discretion: flat counts vs nested enforcement_summary object
- Claude's Discretion: explicit blocked field vs agent-computed
- Claude's Discretion: whether to include human-readable summary line
- Claude's Discretion: enforcement info presence in gate_history responses

### Claude's Discretion
All implementation decisions for Phase 17 are at Claude's discretion. The user trusts the builder to make pragmatic choices informed by the existing codebase patterns (Phase 11-16 conventions), the requirements in ROADMAP.md, and the principle of keeping the MCP surface simple and agent-friendly.

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. Follow patterns established in Phases 14-16.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 17-mcp-hardening-agent-history*
*Context gathered: 2026-03-05*
