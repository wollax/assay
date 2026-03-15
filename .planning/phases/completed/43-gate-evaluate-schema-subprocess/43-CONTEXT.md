# Phase 43: gate_evaluate Schema & Subprocess - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Single-call `gate_evaluate` MCP tool that computes diff, spawns a headless Claude Code evaluator subprocess (`--print --output-format json`), parses structured per-criterion JSON results, and persists a `GateRunRecord`. The evaluator subprocess never calls MCP tools — the parent process owns all parsing and persistence. Context budgeting is Phase 44 (separate).

</domain>

<decisions>
## Implementation Decisions

### EvaluatorOutput schema shape
- Four-state outcomes per criterion: **pass / fail / skip / warn** — skip for criteria the evaluator couldn't assess, warn for soft concerns that shouldn't fail the gate
- Per-criterion results + an **overall summary** with aggregate pass/fail and brief rationale
- Warn on unrecognized extra fields in evaluator output — log warnings for unexpected fields but still accept the result (aids debugging without breaking)

### Subprocess failure modes
- Timeout and crash: **gate-level error, no per-criterion results** — record that evaluation was attempted but failed, distinct from criterion-level judgments (Claude leans this way, has discretion on exact implementation)
- **Configurable retries** in `assay.toml` — retry count for transient subprocess failures (crash, OOM), with sensible default

### Prompt construction
- Include **full context**: diff + criteria + spec description + agent_prompt — evaluator has maximum information to make judgments
- Session integration: if `session_id` provided, **infer worktree_path from the session** to compute the diff (ties into Phase 40-42 work)

### Evaluator invocation details
- Model is **configurable in `assay.toml`** — default model set in config, overridable per-spec for cost/quality trade-off

### Claude's Discretion
- Reasoning capture format per criterion (free-text vs structured reasoning + evidence)
- Single prompt for all criteria vs one subprocess per criterion (trade-off: speed/cost vs independence)
- Output mode: system prompt instructs JSON vs Claude Code `--output-format json` — pick based on what the flag actually provides
- Schema hint in prompt: verbatim schema, example, both, or other approach — pick best prompt engineering strategy
- Session auto-linking: whether `gate_evaluate` automatically transitions session phase and attaches gate run ID when `session_id` is provided
- Evaluator timeout: reuse existing gate timeout cascade or introduce a separate config key
- Partial results handling: use what's parseable (mark missing as skip) vs reject entirely
- Quality detection: whether to warn on suspicious output (empty reasoning, identical results)

</decisions>

<specifics>
## Specific Ideas

- The roadmap specifies `--print --output-format json` for the Claude Code subprocess — this is the invocation pattern
- Lenient parsing via `serde_json::Value` intermediate parse is a locked decision from the v0.4.0 brainstorm
- `gate_evaluate` should use direct Rust function calls for session management (not MCP round-trips) — locked decision from Phase 42
- Diff source should integrate with the session/worktree system from Phases 40-42

</specifics>

<deferred>
## Deferred Ideas

- Context budgeting for diff truncation — Phase 44 (ORCH-04, ORCH-05)
- Prompt engineering optimization and iteration — post-Phase 43 tuning

</deferred>

---

*Phase: 43-gate-evaluate-schema-subprocess*
*Context gathered: 2026-03-15*
