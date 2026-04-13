# Phase 70: Wire Resolution + Preconditions into Gate Pipeline - Context

**Gathered:** 2026-04-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Wire `compose::resolve()` before criteria evaluation and `check_preconditions()` before evaluation in every gate run path (CLI, MCP, TUI), so gates with `extends`, `include`, and `[preconditions]` work correctly at runtime. All building blocks exist from Phases 65-66 — this phase connects them to callers and updates output/history to reflect composition.

Out of scope: new core logic, new MCP tools, wizard changes, validation changes (all done in earlier phases).

</domain>

<decisions>
## Implementation Decisions

### Gate run output format
- Inline source tag after criterion name in CLI streaming output: e.g. `✔ lint-check [Parent: base-gate]`
- Tags appear for inherited/library criteria — Claude's discretion on whether Own criteria also get a tag (signal-to-noise tradeoff)
- JSON mode: per-criterion `source` field already exists from Phase 66 — Claude's discretion on whether to add a top-level composition summary object
- CLI summary line: Claude's discretion on whether to include composition counts

### Precondition failure UX
- CLI exit code 2 for precondition failures — distinct from 0 (pass) and 1 (gate failed). Scripts can distinguish all three states.
- CLI display: Claude's discretion on exact format — should clearly communicate "blocked, not failed" (gate never ran)
- MCP response: Claude's discretion on whether PreconditionFailed is a successful response with distinct outcome field or an MCP error — follow existing patterns
- `gate run all` behavior: Claude's discretion — existing behavior continues on individual failures, blocked specs should follow the same pattern

### Legacy spec handling
- Claude's discretion on all legacy decisions — follow existing patterns
- Key principle: legacy specs (SpecEntry::Legacy) cannot participate in composition, resolution only applies to SpecEntry::Directory
- Gate history is format-agnostic (keyed by spec name) — this should inform precondition requires behavior

### History recording
- PreconditionFailed runs saved to history with distinct outcome type — full audit trail
- Same retention/pruning rules as normal evaluation runs (PreconditionFailed entries count toward max_history)
- Claude's discretion on whether `last_gate_passed()` looks through PreconditionFailed entries to find the last actual evaluation, or treats blocked as not-passed
- Claude's discretion on gate_history MCP tool detail level for precondition entries

### Claude's Discretion
- Source annotation visibility for Own criteria (show tag or omit)
- JSON composition summary metadata shape
- CLI summary line composition counts
- Precondition failure display format (CLI)
- MCP PreconditionFailed response shape (error vs successful with outcome)
- Legacy spec resolution behavior (silent skip vs debug log)
- Whether to always resolve directory specs or check fields first
- `last_gate_passed()` semantics for PreconditionFailed entries
- gate_history detail level for blocked entries
- TUI gate run display adaptations for source annotations and precondition failures

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `compose::resolve()` (`assay-core/src/spec/compose.rs:243`): Closure-based resolution — ready to call from CLI/MCP/TUI
- `check_preconditions()` (`assay-core/src/gate/mod.rs:300`): Evaluates requires + commands, returns PreconditionStatus
- `evaluate_all_resolved()` (`assay-core/src/gate/mod.rs:371`): Accepts ResolvedCriterion slice, preserves source annotations
- `GateEvalOutcome` (`assay-types/src/gate_run.rs`): Evaluated | PreconditionFailed enum — wraps results
- `stream_criterion()` (`assay-cli/src/commands/gate.rs`): CLI streaming evaluator — needs source tag addition
- `format_gate_response()` (`assay-mcp/src/server.rs`): MCP response formatter — needs composition awareness
- `save_run_record()` (`assay-cli/src/commands/gate.rs`): History persistence — needs GateEvalOutcome support
- `last_gate_passed()` (`assay-core/src/history/mod.rs`): History query — may need to handle PreconditionFailed entries
- `load_spec_entry_with_diagnostics()` (`assay-core/src/spec/mod.rs`): Loads spec entry — compose::resolve() needs GatesSpec from directory entry

### Established Patterns
- CLI `handle_gate_run()` dispatches on SpecEntry::Legacy vs SpecEntry::Directory — extend with resolution for directory
- MCP `gate_run()` uses `spawn_blocking` for evaluation — resolution fits in same blocking closure
- `GateRunSummary` is the current return type — `GateEvalOutcome` wraps it
- History JSON files store `GateRunSummary` — needs to accommodate `GateEvalOutcome`
- Exit code pattern: 0 = success, 1 = failure — adding 2 = precondition blocked

### Integration Points
- `crates/assay-cli/src/commands/gate.rs`: `handle_gate_run()` and `handle_gate_run_all()` — add resolution + precondition pipeline
- `crates/assay-mcp/src/server.rs`: `gate_run()` handler — add resolution + precondition pipeline
- `crates/assay-tui/src/app.rs`: TUI gate run path — add resolution + precondition pipeline
- `crates/assay-core/src/history/mod.rs`: History save/load — support GateEvalOutcome
- `crates/assay-core/src/pipeline.rs`: Pipeline evaluation path — may need GateEvalOutcome support

</code_context>

<specifics>
## Specific Ideas

- The resolve → check preconditions → evaluate pipeline should be a clear, sequential flow in each caller. No helper function wrapping all three — each surface may have surface-specific behavior between steps (e.g., CLI streams, MCP creates sessions, TUI updates state).
- Existing callers that use `evaluate_all()` / `evaluate_all_gates()` for directory specs should switch to the resolved path. Legacy specs continue using `evaluate_all()` unchanged.
- The `GateEvalOutcome` type from Phase 66 is the key integration point — every surface returns or handles this enum.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 70-wire-resolution-preconditions*
*Context gathered: 2026-04-13*
