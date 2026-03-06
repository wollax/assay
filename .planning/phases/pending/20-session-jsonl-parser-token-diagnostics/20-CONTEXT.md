# Phase 20: Session JSONL Parser & Token Diagnostics - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Parse Claude Code session JSONL files to provide exact token-aware diagnostics. Delivers a core parser library, CLI commands (`assay context diagnose`, `assay context list`), and MCP tools (`context_diagnose`, `estimate_tokens`). This is the foundation for all context management features in phases 21-23.

</domain>

<decisions>
## Implementation Decisions

### Session file discovery
- Check `.assay/config.toml` for a configured session directory first, fall back to auto-detection from `~/.claude` if not configured
- Default scope is current project only; `--all` flag scans all projects
- Use Claude Code's internal project index (e.g., `projects.jsonl`) to map project paths to session directories
- `context list` shows last 20 sessions by default; `--limit` or `--all` to see more

### Bloat categorization
- Fixed set of 6 categories: progress ticks, thinking blocks, stale reads, tool output, metadata, system reminders
- No extensibility or user-defined categories
- Display both absolute token counts and percentages (e.g., "thinking blocks: 12,450 tokens (23.4%)")
- Token counting uses API usage fields where present, with estimation (tiktoken/character heuristics) for entries without usage data

### CLI output design
- `assay context diagnose` uses dashboard-style sections: Overview, Bloat Breakdown, Top Offenders, Recommendations
- Default target is most recent session; optional session ID argument to target a specific one
- Colored output with Unicode symbols by default (consistent with existing Assay CLI); `--plain` flag for pipe-friendly output

### MCP tool contracts
- `context_diagnose` returns summary-level data (less detail than CLI) -- agents need quick signal, not full breakdown
- `estimate_tokens` performance target is Claude's discretion (100ms goal, correctness over speed)
- Both MCP tools accept optional `session_id` parameter; default to most recent session
- `estimate_tokens` health indicator (healthy/warning/critical) is Claude's discretion

### Claude's Discretion
- Stale read detection heuristic (same path re-read vs content-changed)
- `context list` column selection (based on available data in session files)
- `estimate_tokens` performance trade-offs and health indicator inclusion
- Exact dashboard section content and formatting

</decisions>

<specifics>
## Specific Ideas

- Inspired by [Cozempic](https://github.com/Ruya-AI/cozempic) -- full feature parity with Cozempic's diagnostics, native Rust performance
- Session parser is the foundation for phases 21-23 (team checkpointing, pruning, guard daemon)

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope

</deferred>

---

*Phase: 20-session-jsonl-parser-token-diagnostics*
*Context gathered: 2026-03-06*
