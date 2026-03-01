# Phase 2: MCP Spike - Context

**Gathered:** 2026-02-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Validate that rmcp 0.17 + stdio transport + Claude Code's MCP client exchange protocol successfully. This is a GO/NO-GO gate for the entire v0.1 architecture. The spike proves the integration path works — real MCP tools are built in Phase 8.

</domain>

<decisions>
## Implementation Decisions

### Spike tool behavior
- Tool runs a fixed, hardcoded command (e.g., `echo hello`) and returns the output
- No user-provided command input — zero security surface, pure protocol validation
- Tool named with an obviously temporary name (e.g., `spike_echo`) to signal throwaway code
- Tool will be replaced entirely in Phase 8 when real tools land

### Verification workflow
- Claude's Discretion: verification approach (manual Claude Code test, scripted JSON-RPC, or both)
- Claude's Discretion: plugin config location during spike (project-level `.mcp.json` vs global)
- Claude's Discretion: whether to include verification scripts as artifacts or keep spike code-only
- Claude's Discretion: whether spike code stays in place until Phase 8 or gets stripped to skeleton after GO

### GO/NO-GO criteria
- **GO requires all three:** protocol roundtrip works, Claude Code discovers and calls the tool end-to-end, no non-JSON-RPC bytes leak to stdout
- **NO-GO threshold:** only fundamental blockers trigger NO-GO — minor issues with workarounds are acceptable
- Pivot direction decided at the time if NO-GO occurs — spike findings will inform what to try next
- Claude's Discretion: spike result documentation level (formal report vs STATE.md update)

### Logging and diagnostics
- Default tracing level: `warn` — quiet operation, `RUST_LOG` overrides for debugging
- tracing-subscriber initialized to stderr (stdout reserved for JSON-RPC)
- Claude's Discretion: plain text vs structured JSON output format for stderr
- Claude's Discretion: whether to log JSON-RPC messages at debug level or trust rmcp internals
- Claude's Discretion: tracing-subscriber setup complexity (minimal fmt vs EnvFilter)

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-mcp-spike*
*Context gathered: 2026-02-28*
