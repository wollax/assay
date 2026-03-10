# Requirements: Assay v0.4.0 Headless Orchestration

## Orchestration

- [ ] **ORCH-01**: `gate_evaluate` MCP tool evaluates agent criteria in a single call — computes diff, spawns headless Claude Code evaluator (`--print --output-format json`), parses structured per-criterion results, persists GateRunRecord
- [ ] **ORCH-02**: `gate_evaluate` uses subprocess model — parent parses JSON output, evaluator never calls MCP tools directly
- [ ] **ORCH-03**: `gate_evaluate` defines `EvaluatorOutput` JSON schema before prompt engineering — lenient `serde_json::Value` intermediate parse
- [ ] **ORCH-04**: `gate_evaluate` computes diff token budget via context engine integration (model window − spec criteria − system prompt)
- [ ] **ORCH-05**: `gate_evaluate` truncates diff to budget with head-first + tail fallback strategy

## Session Persistence

- [ ] **SESS-01**: `WorkSession` type persisted as JSON under `.assay/sessions/<session-id>.json` — links worktree path, spec name, agent invocation, gate run references
- [ ] **SESS-02**: `WorkSession` tracks phase transitions with timestamps: `created → agent_running → gate_evaluated → completed | abandoned`
- [ ] **SESS-03**: `session_create` MCP tool creates and persists new session
- [ ] **SESS-04**: `session_update` MCP tool transitions phase and links gate run IDs
- [ ] **SESS-05**: `session_list` MCP tool enumerates sessions with optional spec_name and status filters
- [ ] **SESS-06**: Startup recovery scans `.assay/sessions/` for stale `agent_running` sessions — marks abandoned with recovery note
- [ ] **SESS-07**: `gate_evaluate` calls session management as direct Rust functions, not MCP round-trips

## Spec Validation

- [ ] **SPEC-01**: `spec_validate` MCP tool statically validates specs without running them — returns structured `ValidationResult` with per-criterion diagnostics
- [ ] **SPEC-02**: Validates TOML parse errors, criterion name uniqueness, AgentReport prompt field presence, spec structure completeness
- [ ] **SPEC-03**: Optional `check_commands` parameter validates command existence on PATH (opt-in, off by default)
- [ ] **SPEC-04**: Cross-spec dependency validation with cycle detection when specs declare `depends = [...]`

## Context Engine Integration

- [ ] **CTX-01**: Depend on external context-engine crate for token-budgeted context windowing
- [ ] **CTX-02**: Define integration surface — context engine provides budget allocation, assay provides content sources (diff, spec, criteria)
- [ ] **CTX-03**: Fallback behavior when context engine is not available or budget exceeds content — pass through without truncation

## Quick Wins — Observability

- [ ] **OBS-01**: `warnings` field (`Vec<String>`, skip_serializing_if empty) on all mutating MCP tool responses — surfaces history save failures, diff capture failures, cleanup warnings
- [ ] **OBS-02**: Outcome-filtered `gate_history` — `outcome` parameter (passed/failed/any), `limit` parameter (default 10, max 50). Failed = `required_failed > 0`
- [ ] **OBS-03**: `spec_get` optional `resolve` parameter returns effective timeouts (3-tier precedence) and working_dir validation
- [ ] **OBS-04**: Growth rate metrics in `estimate_tokens` — avg tokens per turn, estimated turns remaining (requires 5+ assistant turns)

## Quick Wins — Correctness & Robustness

- [ ] **FIX-01**: Worktree status computes ahead/behind relative to base branch tip (not upstream) — fixes false `0/0` for assay-managed branches
- [ ] **FIX-02**: Better `gate_report`/`gate_finalize` error messages — distinguishes session timeout vs session not found, includes recovery hints
- [ ] **FIX-03**: Diff context attached to gate sessions — `git diff HEAD` (32 KiB cap, head-biased truncation) stored on AgentSession with `diff_truncated` flag

## Tech Debt Cleanup

- [ ] **DEBT-01**: Batch sweep of highest-value backlog issues — prioritize items that interact with v0.4.0 changes (worktree, MCP, types, history)
- [ ] **DEBT-02**: Close `history-save-failure-not-surfaced` issue (subsumed by OBS-01 warnings field)

## Traceability

_Filled by roadmapper._

---

## Future Requirements (deferred)

- [ ] Gate DAG with criteria dependency chains and `skip_reason` on CriterionResult — v0.4.1 opportunistic
- [ ] Criterion-level retry with `max_attempts` — v0.4.1
- [ ] Real-time gate output streaming via SSE — v0.5.0
- [ ] `gate_health` MCP tool with per-criterion pass rates — v0.4.1+
- [ ] `extends:` single-level spec inheritance — v0.4.1+
- [ ] `gate_sanity` CLI command — verify gates can actually fail — v0.4.1+
- [ ] `gate_history --summary` with trend aggregation — v0.4.1+
- [ ] Pruning metadata in GateRunRecord — v0.4.1+

## Out of Scope

- Real-time streaming (wrong transport model for MCP stdio)
- Predictive failure modeling (cold-start problem, needs 100+ runs)
- Full spec composition DAG (single-level inheritance first)
- Agent-driven mutation testing (cargo mutants territory)
- Federation / community benchmarks (product decision, not engineering)
- Self-amending specs (creates incentive to weaken standards)
