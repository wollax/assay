# Phase 8: MCP Server Tools - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace the Phase 2 spike server with real MCP tools that expose spec and gate operations. Three tools (`spec_get`, `spec_list`, `gate_run`) served via `assay mcp serve` over stdio. The spike tool is removed entirely. Plugin configuration and skills are Phase 10.

</domain>

<decisions>
## Implementation Decisions

### Tool response shapes
- `gate_run` returns summary by default: pass/fail, exit code, duration per criterion, plus aggregate "N/M passed"
- `gate_run` accepts an optional `include_evidence` parameter — when true, includes full stdout/stderr per criterion
- `spec_get` response shape: Claude's discretion based on what agents need from the parsed spec
- `spec_list` response shape: Claude's discretion on whether to include metadata beyond names

### Error reporting to agents
- Error surface strategy (MCP error vs success-with-error-field): Claude's discretion based on MCP conventions and agent ergonomics
- Timeout behavior (fail-fast vs continue): Claude's discretion based on what's most useful for agent decision-making
- Error detail level (problem-only vs problem+hint): Claude's discretion for v0.1
- Init validation boundary (startup vs per-tool): Claude's discretion on where to guard for uninitialized projects

### Project discovery
- CWD only — server expects `.assay/` in the current working directory, no tree-walking
- Resolution timing (startup vs per-call): Claude's discretion based on MCP server lifecycle patterns
- Gate working directory: Claude's discretion, consistent with existing CLI gate evaluation behavior

### Tool naming and descriptions
- Tool names use underscores: `spec_get`, `spec_list`, `gate_run`
- Tool name prefix (assay_ or not): Claude's discretion based on MCP client presentation
- Description verbosity: Claude's discretion — MCP-08 requirement says "clear enough for agent discovery without additional prompting"
- Parameter descriptions: Claude's discretion — should satisfy MCP-08 self-documenting requirement

### Spike cleanup
- Remove the Phase 2 spike tool entirely — real tools replace it, clean break

### Claude's Discretion
- `spec_get` response detail level
- `spec_list` response detail level
- Error surface strategy and detail level
- Timeout continuation behavior
- Init validation boundary
- CWD resolution timing
- Gate execution working directory
- Tool name prefix convention
- Description and parameter documentation verbosity

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. The existing spike code (Phase 2) serves as a working reference for rmcp patterns (`#[tool_router]`, `#[tool_handler]`, tracing-to-stderr).

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 08-mcp-server-tools*
*Context gathered: 2026-03-02*
