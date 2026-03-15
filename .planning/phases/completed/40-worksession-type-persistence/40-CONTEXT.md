# Phase 40: WorkSession Type & Persistence - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Define and persist the `WorkSession` type as JSON under `.assay/sessions/`. A session links a worktree path, spec name, agent invocation details, and gate run references. Sessions track phase transitions (`created → agent_running → gate_evaluated → completed | abandoned`) with timestamps and an audit trail. Sessions must round-trip through JSON without data loss.

Phase 41 (Session MCP Tools) and Phase 42 (Session Recovery) build on top of this type — this phase delivers the data model and persistence layer only.

</domain>

<decisions>
## Implementation Decisions

### Session Identity
- Session IDs use **ULID** — sortable, unique, lexicographic ordering matches creation order
- ULID dependency: Claude's discretion on whether to use `ulid` crate or alternative
- Multiple sessions per spec are **allowed without restriction** — agents may work on the same spec in parallel (e.g. different worktrees)
- File naming/directory structure: Claude's discretion

### State Transitions
- Valid states: `created`, `agent_running`, `gate_evaluated`, `completed`, `abandoned`
- Transitions follow the linear sequence: `created → agent_running → gate_evaluated → completed`
- **Any state can transition to `abandoned`** — this is the escape hatch
- Completed and abandoned are terminal — re-opening behavior is Claude's discretion
- Invalid transitions return an error — error design is Claude's discretion

### Transition Audit Trail
- Each transition records a **full audit entry**: timestamp, trigger (what caused it), and optional notes/context
- Trigger examples: gate_run ID, MCP tool name, "recovery scan", "user abandoned"
- The session stores a history of all transitions, not just the current state

### Data Relationships
- Gate run references: Claude's discretion on whether to store bare IDs or embedded summaries
- Worktree path storage format: Claude's discretion (absolute vs relative)
- Agent invocation captures: **spec name + invocation command + model info**
- Spec reference approach (slug only vs slug + content hash): Claude's discretion

### Session File Lifecycle
- **Completed sessions**: retained forever (lightweight JSON, user deletes manually if needed)
- **Abandoned sessions**: TTL-based cleanup (auto-delete after configurable duration)
- Git tracking (.assay/sessions/ gitignored or tracked): Claude's discretion based on existing .assay/ conventions
- Corrupt file handling: Claude's discretion
- Cleanup trigger (manual vs automatic): Claude's discretion — consider what Phase 42 (startup recovery) needs

### Claude's Discretion
- ULID crate selection
- File naming and directory structure for sessions
- Gate run reference style (bare IDs vs embedded summaries)
- Worktree path storage format
- Spec reference approach
- Invalid transition error design
- Re-opening completed sessions
- Git tracking of sessions directory
- Corrupt file handling strategy
- Cleanup trigger mechanism

</decisions>

<specifics>
## Specific Ideas

- Transition audit trail should feel like a lightweight event log — each entry is a record of "what happened and when"
- The `WorkSession` type will be consumed by Phase 41 (MCP tools) and Phase 42 (recovery) — design for those downstream uses

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 40-worksession-type-persistence*
*Context gathered: 2026-03-15*
