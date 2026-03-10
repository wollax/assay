# Phase 33: MCP Validation - Context

**Gathered:** 2026-03-10
**Status:** Ready for planning

<domain>
## Phase Boundary

Harden MCP tool parameter validation with specific error messages, improve spec-not-found diagnostics, check stdout for failure reasons, and remove unnecessary clones. Dependencies: Phase 31 (error message patterns established). Requirements: MCP-01 through MCP-05.

</domain>

<decisions>
## Implementation Decisions

### Failure reason extraction (MCP-04)
- Stderr-first, stdout-fallback — if `first_nonempty_line(&stderr)` returns content, use it as the reason
- If stderr is empty, fall back to `first_nonempty_line(&stdout)`
- Never combine both streams into the reason string — agents need a single concise line
- Matches unix convention: stderr is the diagnostic channel

### Validation error message format (MCP-01, MCP-02)
- Terse, parameter-naming format: `"missing required parameter: <name>"` / `"invalid parameter '<name>': expected <type>"`
- Return via existing `domain_error()` → `CallToolResult::error()` path
- No schema dumps, no recovery hints beyond naming what's wrong
- Agents self-correct from parameter name alone

### Validation strategy (MCP-01, MCP-02)
- Verify-first approach: check what the MCP SDK's `Parameters<T>` deserialization actually produces for missing/wrong-type fields
- If Serde errors already name the parameter clearly, criteria may be satisfied without custom code
- If Serde errors are cryptic (e.g., `"missing field at line 1 column 42"`), add a thin validation layer
- This is a research question — researcher should test actual error output before designing a solution

### Spec-not-found completeness (MCP-03)
- `load_spec_entry_mcp` already flows through `load_spec_entry_with_diagnostics` which produces `SpecNotFoundDiagnostic` with available specs, invalid specs, and fuzzy suggestions
- Researcher should verify the wire-up is complete and the enriched error string reaches agents
- If already working, this criterion becomes a verification checkbox with a test, not implementation work

### Clone removal (MCP-05)
- Audit `gate_run` handler for unnecessary `.clone()` calls on intermediaries
- Remove clones where ownership can be transferred or references suffice

### Claude's Discretion
- Exact validation layer implementation (if needed after Serde verification)
- Whether to wrap Serde errors or intercept at deserialization
- Test structure and assertion patterns
- Clone removal specifics (borrow vs move decisions)

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. Follow existing patterns established in Phase 31 error messages.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 33-mcp-validation*
*Context gathered: 2026-03-10*
