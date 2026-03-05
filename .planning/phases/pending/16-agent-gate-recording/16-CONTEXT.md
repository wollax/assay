# Phase 16: Agent Gate Recording - Context

**Gathered:** 2026-03-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Agents can submit gate evaluations via the MCP `gate_report` tool with structured reasoning, creating the second track of Assay's dual-track quality gates. Agent-reported results are persisted to run history with evaluator role metadata.

Requirements: AGNT-01, AGNT-02, AGNT-03, AGNT-04
Depends on: Phase 13 (Enforcement), Phase 14 (Run History Core)

</domain>

<decisions>
## Implementation Decisions

### Submission Model
- One criterion per `gate_report` call, referenced by criterion `name` field
- Results accumulate into a session; explicit `gate_finalize` call produces the `GateRunRecord`
- Partial results accepted — gate cannot pass until all required criteria are evaluated
- `gate_run` auto-creates a session when the spec contains `AgentReport` criteria
- `gate_run` executes command criteria immediately, shows agent criteria as "pending/awaiting evaluation", returns without blocking
- Sessions are serializable for crash recovery
- Stale sessions auto-finalize on timeout with partial results — missing required criteria cause gate failure, forcing remediation

### Structured Reasoning Fields
- `evidence` — what the agent observed (concrete facts)
- `reasoning` — why those facts lead to pass/fail
- `confidence` — optional, enum: `high` / `medium` / `low`

### Evaluator Roles
- `self` — same agent that wrote the code evaluates its own work
- `independent` — different agent (or same agent in separate context) evaluates without having written the code
- `human` — human submitted the evaluation (defined now, usable in future phases)
- Agent self-declares its role in the `gate_report` call
- Multiple evaluations per criterion allowed from different roles
- Priority for effective result: human > independent > self (all kept, highest-priority is authoritative)

### Enforcement and Trust
- Agent-reported gates default to `advisory` enforcement unless overridden in spec
- Spec author is the trust ceiling — agent cannot escalate enforcement above spec-defined level
- Agent can submit with enforcement <= spec-defined level
- `GateRunSummary.passed` is only set at finalization, not live-updated during session

### Type Design
- `GateKind::AgentReport` variant — carries the static definition (this criterion is agent-evaluated)
- Agent evaluation data (`evidence`, `reasoning`, `confidence`, `evaluator_role`) lives on `GateResult` as optional fields (mirrors how Command has `cmd` on GateKind but `stdout`/`exit_code` on GateResult)
- New `prompt` field on `Criterion` — optional, provides instruction to the agent for evaluation
- Explicit `kind = "AgentReport"` field on `Criterion` to declare agent-evaluable criteria
- `kind = "AgentReport"` and `cmd`/`path` are mutually exclusive (validation error if both set)
- Session state type (`AgentSession`) in `assay-types`, serializable for crash recovery

### Criterion Validation
- `gate_report` with a criterion name not in the spec is a hard error (rejected immediately)
- Spec dispatch: explicit `kind` field takes precedence over inferred `cmd`/`path` logic

### Visual Distinction
- Label-based approach: `[agent]` / `[cmd]` / `[file]` prefixes in all surfaces
- Applies to: CLI `gate run` output, `history` detail view, MCP responses

### Claude's Discretion
- Session timeout duration
- Exact serialization format for `AgentSession`
- Label styling/colors in CLI output
- `gate_finalize` MCP tool response shape
- Error messages for validation failures

</decisions>

<specifics>
## Specific Ideas

- `Criterion` already has a comment on line 24 anticipating a `prompt` field for agent-based evaluation — this phase delivers it
- The split mirrors existing patterns: `GateKind` = definition, `GateResult` = execution output
- Session model is similar to a transaction — accumulate, then commit

</specifics>

<deferred>
## Deferred Ideas

- Human evaluation submission path (CLI/UI) — future phase, `human` role variant defined now
- `gate_history` MCP tool for querying past results — Phase 17
- MCP timeout parameter — Phase 17
- Independent evaluator orchestration (spawning a separate agent) — v0.3.0

</deferred>

---

*Phase: 16-agent-gate-recording*
*Context gathered: 2026-03-05*
