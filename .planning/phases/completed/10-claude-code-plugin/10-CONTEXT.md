# Phase 10: Claude Code Plugin - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

A working Claude Code plugin that installs Assay as an MCP server and provides skills and hooks for spec-driven development workflows. Delivers: `.mcp.json` config, `/gate-check` skill, `/spec-show` skill, CLAUDE.md workflow snippet, PostToolUse hook (reminder-based), Stop hook (configurable enforcement).

</domain>

<decisions>
## Implementation Decisions

### Skill output design
- `/gate-check` uses adaptive output: concise summary on pass ("3/3 criteria passed"), full evidence (stdout/stderr/exit code/duration) on failure
- `/gate-check` spec argument is optional: if omitted, run all specs; if provided, run just that one
- `/spec-show` detail level and output format (markdown vs plain text): Claude's discretion

### Hook enforcement strategy
- PostToolUse hook is **reminder-only**, not auto-execution — after Write/Edit, the hook reminds the agent to run `/gate-check` when ready; it does NOT auto-run gates
- Stop hook defaults to **hard block** (agent cannot complete while gates fail), but is configurable — user can soften to warn-and-allow
- When the agent runs `/gate-check` (on-demand), it sees the full adaptive output (same as the skill)
- Hook scope for spec selection: Claude's discretion

### CLAUDE.md workflow snippet
- **Prescriptive** tone — step-by-step workflow: read spec, implement criteria, run gates, iterate until pass
- **Mandatory spec-first** — "Always read the relevant spec before writing code"
- **Both abstract + concrete** — abstract guidance ("read specs and validate before completing") with a command reference section listing `/spec-show` and `/gate-check`
- **Both static + generated** — plugin ships a static default snippet; `assay init` can enhance it with project-specific details (spec names, project context)

### Installation and discovery
- Binary path is **configurable** — default to PATH lookup ("assay"), but allow user to set explicit path in plugin config
- `.mcp.json` lives in **both** places — plugin ships a default; `assay init` can optionally create a project-level override
- Missing binary produces a **clear error message** with install instructions ("assay not found. Install with: cargo install assay-cli")
- **Graceful degradation** — full functionality when `assay init` has been run, but basic commands still work without `.assay/` directory

### Claude's Discretion
- `/spec-show` detail level and format
- Hook scope (current spec vs all specs)
- Exact markdown formatting in skill output
- PostToolUse reminder phrasing
- Plugin config file format and location

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

*Phase: 10-claude-code-plugin*
*Context gathered: 2026-03-02*
