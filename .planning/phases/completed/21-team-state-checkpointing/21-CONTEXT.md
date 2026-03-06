# Phase 21: Team State Checkpointing - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Extract and persist agent team state from Claude Code session JSONL files and `~/.claude/teams/*/config.json`. Provide on-demand CLI snapshots (`assay checkpoint`) and automatic hook-driven checkpoints on PostToolUse[Task|TaskCreate|TaskUpdate], PreCompact, and Stop events. Pruning strategies and guard daemon are separate phases (22, 23).

</domain>

<decisions>
## Implementation Decisions

### Checkpoint content
- **Agent detail level:** Moderate — each agent entry includes name, model, status (active/idle/done), current task, and working directory
- **Task state:** Structured entries with task name, status (pending/in-progress/done), assigned agent, and last update time
- **Coordination summary:** Claude's discretion — determine what coordination content (metadata vs key messages) best serves recovery context
- **Format:** Claude's discretion — choose between pure markdown or dual-purpose (markdown + structured frontmatter) based on what downstream phases (pruning, guard daemon) need

### File location & lifecycle
- **Location:** Claude's discretion — pick the most natural subdirectory under `.assay/` given existing layout
- **Cardinality:** Both — a rolling "latest" checkpoint file that's always current, plus timestamped snapshot archive for history comparison
- **Retention:** Claude's discretion — determine appropriate retention policy based on downstream phase needs
- **Git tracking:** Claude's discretion — decide whether checkpoints are operational (gitignored) or archival (committed) based on their nature

### Hook trigger behavior
- **Debouncing:** Claude's discretion — pick the right trade-off between accuracy and I/O overhead for rapid-fire events
- **Failure handling:** Claude's discretion — hooks should never break agent workflow; pick appropriate error handling
- **Hook tiers:** Claude's discretion — determine if PreCompact/Stop should behave differently from PostToolUse hooks
- **Sync model:** Claude's discretion — pick based on Claude Code plugin hook constraints

### Config.json discovery
- **Discovery strategy:** Claude's discretion — pick the most reliable approach to finding team config files
- **Authority on conflict:** Merge with config.json priority — use config as the authoritative base, enrich with session runtime data (timestamps, status, current task)
- **Solo agent (no team):** Claude's discretion — determine if solo checkpointing adds value for session recovery
- **Freshness:** Claude's discretion — pick based on typical team config change frequency

### Claude's Discretion
- Checkpoint file location within `.assay/`
- Markdown format (pure vs dual-purpose with frontmatter)
- Coordination summary depth (metadata only vs key messages)
- Hook debouncing strategy
- Hook failure handling approach
- Hook tier differentiation (PreCompact/Stop vs PostToolUse)
- Hook sync model (blocking vs fire-and-forget)
- Config.json discovery strategy
- Solo agent checkpoint behavior
- Config freshness strategy (on-demand vs cached)
- Checkpoint retention policy
- Git tracking decision

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

*Phase: 21-team-state-checkpointing*
*Context gathered: 2026-03-06*
