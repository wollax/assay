# Phase 60: Process Safety - Context

**Gathered:** 2026-04-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix 5 process lifecycle and output safety bugs (SAFE-01 through SAFE-05). All requirements come from post-M024 review findings with precise success criteria.

</domain>

<decisions>
## Implementation Decisions

### SAFE-01: Process group termination
- `kill_agent_subprocess` in `pipeline_checkpoint.rs:191` must use `killpg` instead of single-process `child.kill()`
- Reference implementation already exists in `gate/mod.rs:794` — same pattern (set pgid via `pre_exec`, `killpg` with SIGKILL)

### SAFE-02: Auto-promote TOCTOU race
- `pipeline.rs:1130-1191` has check-then-act on spec status — race between status check and promotion
- Must make the status transition atomic (CAS, lock, or filesystem-level atomic rename)

### SAFE-03: Pipeline crash stderr capture
- When pipeline subprocess crashes, error messages must include stderr content
- Currently stderr is captured but not included in crash error paths

### SAFE-04: Relay thread panic logging
- `relay.join().expect("thread panicked")` at multiple call sites (`pipeline.rs:1497`, etc.) propagates panic to caller
- Must catch panics and log them instead of crashing the host

### SAFE-05: TUI ANSI stripping
- `app.rs:342` uses `TextDelta` text raw — no control character filtering
- Must strip ANSI escape sequences and control characters before rendering to prevent terminal injection

### Claude's Discretion
- ANSI stripping approach (regex, byte scan, or `strip-ansi-escapes` crate)
- Specific atomic mechanism for TOCTOU fix (filesystem rename vs in-memory lock)
- Whether to add `pre_exec` for pgid in `kill_agent_subprocess` or restructure to share gate's existing pattern

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `gate/mod.rs:792-794`: Working `killpg` pattern with `pre_exec` for pgid — direct reference for SAFE-01
- `pr.rs:111-121`: stderr capture pattern already used in PR module — reference for SAFE-03
- `review/mod.rs:643`: TOCTOU-aware comment about atomic directory creation — shows awareness of the pattern

### Established Patterns
- Process spawning uses `Command::new()` with explicit `stdout`/`stderr` piping
- Relay threads return exit codes via `JoinHandle<i32>` — panic propagation via `.expect()`
- TUI event handling in `app.rs:342` dispatches on `AgentEvent` variants

### Integration Points
- `kill_agent_subprocess` called from `pipeline_checkpoint.rs:571,607`
- Auto-promote in `pipeline.rs:1130-1191` (single call site)
- Relay thread joins at `pipeline.rs:1497,1524,1533,1552` and `pipeline_streaming.rs`
- TUI TextDelta handling at `app.rs:342`

</code_context>

<specifics>
## Specific Ideas

No specific requirements — all 5 fixes are precisely defined by the SAFE-01 through SAFE-05 success criteria.

</specifics>

<deferred>
## Deferred Ideas

None — discussion skipped (all requirements are clear-cut bug fixes with no ambiguity).

</deferred>

---

*Phase: 60-process-safety*
*Context gathered: 2026-04-08*
