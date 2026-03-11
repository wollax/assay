# Phase 37: Spec Validation - Context

**Gathered:** 2026-03-11
**Status:** Ready for planning

<domain>
## Phase Boundary

A `spec_validate` MCP tool that checks spec files for correctness without running them. Validates TOML parsing, structural completeness, criterion uniqueness, AgentReport prompt presence, optional command existence on PATH, and cross-spec dependency cycles. Does not execute specs or evaluate gates.

</domain>

<decisions>
## Implementation Decisions

### Diagnostic structure
- Three severity levels: error, warning, info
- Errors block validity; warnings are advisory (but see strict mode); info provides suggestions
- Claude's discretion on whether diagnostics are flat or grouped, and whether to include summary counts
- Claude's discretion on location references (line numbers vs path-based vs message-only)

### Validation strictness
- Configurable strict mode: default allows warnings without blocking; `strict: true` parameter promotes warnings to errors
- Missing optional fields (description, metadata) are info-level hints — never blocking
- Unrecognized/extra fields in spec TOML are warnings — catches typos, blocks in strict mode
- Claude's discretion on empty criteria list severity

### Dependency resolution
- Full transitive cycle detection — A -> B -> C -> A is caught, not just direct cycles
- Cycle error messages show the full cycle path (e.g., "A -> B -> C -> A") so the user knows where to break it
- Claude's discretion on missing dependency severity (error vs warning)
- Claude's discretion on single-spec vs all-specs validation scope

### Command checking
- `check_commands` parameter is opt-in (off by default), as specified in roadmap
- Verify both existence AND execute permissions on the binary
- Cross-platform: handle platform-specific resolution (Unix `which`, Windows `where` equivalents)
- Claude's discretion on argument parsing (first token only vs intelligent extraction)
- Claude's discretion on command-not-found severity level

### Claude's Discretion
- Diagnostic organization (flat list vs grouped by category)
- Summary counts in top-level response
- Location reference approach (line numbers, structural paths, or message context)
- Empty criteria list severity level
- Missing dependency severity (error vs warning)
- Single-spec vs all-specs validation scope
- Command argument parsing strategy
- Command-not-found severity when check_commands is enabled

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. Decisions give Claude wide latitude on implementation details while locking the user-facing behaviors: three severity levels, configurable strict mode, full transitive cycle detection with path display, and cross-platform command checking with execute permission verification.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 37-spec-validation*
*Context gathered: 2026-03-11*
