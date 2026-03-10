# Phase 32: CLI Polish - Context

**Gathered:** 2026-03-10
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix correctness issues and eliminate code duplication across the CLI surface — NO_COLOR handling, help text, enforcement blocks, color branches, StreamCounters, and magic strings. Requirements: CLI-01 through CLI-08.

</domain>

<decisions>
## Implementation Decisions

### NO_COLOR behavior
- Follow the no-color.org spec strictly — only respect `NO_COLOR`, no `FORCE_COLOR`/`CLICOLOR_FORCE` support
- Presence-based detection: `var_os("NO_COLOR").is_some()` disables color (any value including empty)
- Auto-disable color when stdout is not a TTY (standard CLI convention)

### Help text deduplication
- Use clap's built-in `#[command(about = ...)]` / `#[arg(help = ...)]` attributes — no custom help templates
- Running `assay gate` with no subcommand shows help automatically (not an error)

### Magic string extraction
- Extract only the `[srs]` magic string (CLI-05 scope) — no broader audit
- Constant lives in `assay-types` (protocol-level marker shared across crates)

### Claude's Discretion
- NO_COLOR: Whether to resolve color mode once at startup vs per-call-site (leaning startup, but flexible)
- Help text: Structure of top-level vs subcommand detail; whether to include usage examples
- StreamCounters: All API design decisions — tally() return type, gate_blocked() signature, enforcement check placement, module location (cli vs core)
- Magic string: Constant naming and whether to include brackets in the value

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 32-cli-polish*
*Context gathered: 2026-03-10*
