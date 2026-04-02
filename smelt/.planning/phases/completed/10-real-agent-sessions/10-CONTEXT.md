# Phase 10: Real Agent Sessions - Context

**Gathered:** 2026-03-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Add Claude Code as a real agent backend for the session controller. A real agent session launches Claude Code in a worktree with a task prompt derived from the session manifest. This slots into the proven interface that scripted sessions validated across Phases 3-9. Process lifecycle is managed correctly — graceful shutdown on orchestrator interrupt, zombie prevention via process group management. End-to-end: an orchestration plan with 2+ real agent sessions produces a merged branch with combined work.

</domain>

<decisions>
## Implementation Decisions

### Agent invocation
- Use `claude --prompt "..."` CLI flag as the single prompt delivery method for v0.1.0
- Stdin pipe and --prompt-file deferred to future iteration
- One invocation method keeps implementation simple and testable

### Completion detection
- Wait for Claude Code process to exit (exit code), then validate branch has commits (belt-and-suspenders)
- Exit code 0 + commits = Completed
- Exit code non-zero = Failed
- Exit code 0 + no commits = Claude's discretion (fits existing SessionOutcome patterns)

### Session timeout
- Configurable per-session timeout via manifest field (e.g. `timeout = "30m"`)
- Agent process killed after deadline — prevents runaway sessions
- Per-session granularity (not just global) since different tasks have different complexity

### Task prompt construction
- Claude's discretion on how much context beyond the task description to include (task + file scope hints + repo metadata as appropriate)
- Claude's discretion on whether to support an optional `prompt_template` field or use a fixed template for v0.1.0

### Worktree injection
- Inject a CLAUDE.md into the worktree with session constraints (scope guidance, commit instructions)
- Inject a custom settings.json to configure Claude Code for headless execution:
  - Allowed tools (restrict to Edit/Write/Bash etc., disable web/MCP as appropriate)
  - Auto-accept permissions (non-interactive, no permission prompts)
  - Model selection (pin model per session or use manifest-level default)
- Cleanup of injected files: Claude's discretion based on existing worktree cleanup patterns

### Output & logging
- Always capture stdout/stderr to log files at `.smelt/runs/<run_id>/logs/<session>.log`
- With `--verbose`, also stream last N lines to the orchestrator dashboard
- Log files retained until run cleanup (follows existing RunStateManager lifecycle)
- Structured JSON conversation log capture: Claude's discretion based on what Claude Code exposes
- Token/cost display in dashboard: Claude's discretion (roadmap deferred cost tracking, but trivial inclusion is fine)

### Claude's Discretion
- Binary path: whether to always use `claude` from PATH or allow config override
- Prompt context richness: how much repo metadata to include alongside task description
- Prompt template: fixed template vs optional override for power users
- Injected file cleanup: remove after session or leave for inspection
- Structured output capture: JSON conversation log if Claude Code exposes it trivially
- No-commit session handling: how to map to existing SessionOutcome
- Token/cost display: include if trivially available from Claude Code output

</decisions>

<specifics>
## Specific Ideas

- Settings.json injection mirrors how Claude Code users configure their own environments — familiar pattern
- CLAUDE.md injection is non-invasive since Claude Code reads it automatically from the worktree
- The existing ProcessGroup from Phase 3 (libc SIGTERM) should be reused for real agent lifecycle management
- Per-session timeout enables heterogeneous workloads (quick lint fix vs large refactor) in the same manifest

</specifics>

<deferred>
## Deferred Ideas

- Stdin pipe and --prompt-file as alternative prompt delivery methods
- Multiple agent backends beyond Claude Code (e.g. Cursor, Aider, custom agents)
- Cost tracking and budget limits per session/run
- Real-time streaming of agent conversation to dashboard (beyond log tailing)

</deferred>

---

*Phase: 10-real-agent-sessions*
*Context gathered: 2026-03-11*
