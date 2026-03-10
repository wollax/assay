# Phase 29: Gate Output Truncation - Context

**Gathered:** 2026-03-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement head+tail output capture with byte budgets so gate command output is bounded, UTF-8 safe, and truncation is visible in results. Covers: truncation engine, GateResult field additions (`truncated`, `original_bytes`), independent stdout/stderr budgets, and UTF-8 safety at truncation boundaries.

</domain>

<decisions>
## Implementation Decisions

### Byte budget defaults
- Default byte budget: **32 KiB (32,768 bytes) per stream**
- stdout and stderr have **independent budgets** — one can truncate while the other doesn't
- Total worst-case stored output per gate: 64 KiB (32 KiB stdout + 32 KiB stderr)

### Head/tail ratio
- Claude's Discretion — pick the right default split ratio

### Truncation marker format
- Claude's Discretion — pick the right marker format, detail level, stream labeling, and visual treatment

### Claude's Discretion
The user delegated significant implementation freedom on these areas:
- **Per-gate configurability** of byte budget — whether to support it and how
- **Head/tail ratio** — default split and whether it's configurable
- **Budget target** — whether budget counts raw captured bytes or final UTF-8 string
- **Truncation unit** — byte-based with UTF-8 alignment vs line-based within budget
- **Marker format details** — bytes-only vs bytes+lines, inline vs separate field, stream-labeled vs uniform, plain text vs styled in CLI
- **Empty output representation** — how GateResult fields behave with no output
- **Binary/non-UTF-8 handling** — lossy conversion vs hex escaping
- **Truncation timing** — post-capture vs streaming ring buffer
- **Backward compatibility** — whether old results are retroactively truncated on read

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. The success criteria from ROADMAP.md are precise and testable:

1. Output exceeding byte budget → head+tail with marker
2. Marker format: `[truncated: X bytes omitted]`
3. Never split multi-byte UTF-8 sequences
4. Independent stdout/stderr budgets
5. `GateResult.truncated` = true + `GateResult.original_bytes` set when truncation occurs

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 29-gate-output-truncation*
*Context gathered: 2026-03-09*
