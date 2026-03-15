# Phase 41: Session MCP Tools - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Expose the WorkSession persistence layer (Phase 40) as three MCP tools: `session_create`, `session_update`, and `session_list`. All tools include Phase 35 `warnings` field on responses. Session recovery (Phase 42) and gate_evaluate integration (Phase 43) are out of scope.

</domain>

<decisions>
## Implementation Decisions

### Create parameters
- Worktree path optionality, agent info requirements, spec validation, and auto-persist behavior are all Claude's discretion — pick what fits existing MCP patterns and Phase 42/43 downstream needs
- Key constraint: the underlying `create_work_session` function requires all fields, so the MCP tool must either require them or provide sensible defaults

### Update contract
- Phase transition mechanism (raw phase target vs semantic actions), gate_run ID linking approach, trigger string handling, and error verbosity are all Claude's discretion
- Key constraint: success criteria requires "invalid transitions are rejected with clear errors" — whatever approach is chosen must surface actionable feedback
- Gate run linking must work for Phase 43's `gate_evaluate` which will call session management through Rust functions (not MCP)

### List filtering
- Filter matching semantics (exact vs prefix), multi-status support, limit parameter, and detail level are all Claude's discretion
- Key constraint: success criteria requires `spec_name` and `status` filters — both must be present as optional params
- Consider token budget impact when deciding between full objects vs summaries

### Response shape
- Response payload size (full session vs minimal confirmation) is Claude's discretion for all three tools
- Whether to add a 4th `session_get` tool is Claude's discretion — evaluate if session_list with filters covers single-session lookup adequately
- Warnings field follows the established Phase 35 pattern (Vec<String>, skip_serializing_if empty)

### Claude's Discretion
- All implementation choices across all four areas — user deferred everything to Claude
- Should be guided by: consistency with existing MCP tools (gate_run, gate_history, gate_report, gate_finalize), downstream consumer needs (Phase 42/43), and MCP response token budgets
- Key existing patterns to follow: GateRunParams/GateHistoryParams for param structs, GateRunResponse/GateHistoryResponse for response structs

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. Follow existing assay MCP tool conventions.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 41-session-mcp-tools*
*Context gathered: 2026-03-15*
