# Brainstorm Summary: Assay v0.4.0

**Date:** 2026-03-10
**Topic:** Headless agent orchestration with context-aware gate evaluation
**Teams:** 3 explorer/challenger pairs (quick-wins, high-value, radical)
**Reports:** [quickwins-report.md](quickwins-report.md) | [highvalue-report.md](highvalue-report.md) | [radical-report.md](radical-report.md)

---

## Executive Summary

20 proposals entered debate across 3 pairs. After 3 rounds of challenge per pair, the brainstorm converged on a clear v0.4.0 architecture: **`gate_evaluate` as the capstone MCP tool** supported by session persistence, context budgeting, and spec validation — plus quick-win observability improvements and radical "gates-as-data" seeds.

**Key architectural decision confirmed:** Context engine starts as workspace crate (`assay-context`), not separate repo. Extract to standalone repo only when Smelt actually needs to consume it with a stable API.

---

## Surviving Proposals by Category

### Quick Wins (~20-26h total)

| ID | Proposal | Scope | Theme |
|----|----------|-------|-------|
| QW-1 | Diff context attached to gate sessions (32 KiB cap) | 6-8h | v0.4.0 scaffolding |
| QW-2 | Better session-not-found errors (timeout vs missing) | 1-2h | Headless robustness |
| QW-3 | Worktree status relative to base branch (not upstream) | 2-3h | Correctness fix |
| QW-4 | Outcome-filtered `gate_history` (passed/failed/any + limit) | 2-3h | Agent observability |
| QW-5 | `spec_get` resolved config (timeout precedence visibility) | 3-4h | Agent observability |
| QW-6 | `warnings` field on mutating MCP tools (closes open issue) | 2-3h | Observability pattern |
| QW-7 | Growth rate metrics in `estimate_tokens` | 3h | Session management |

### High-Value Features (~4.5 weeks)

| # | Feature | Estimate | Key Decision |
|---|---------|----------|--------------|
| 1 | `spec_validate` — static spec health checker | 3 days | `check_commands` opt-in, validates `prompt` not `description` |
| 2 | Context engine workspace crate (`assay-context`) | 1 week | Workspace crate, not separate repo; byte-heuristic default |
| 3 | `WorkSession` persistence + session MCP tools | 2 weeks | `gate_evaluate` calls Rust functions, not MCP round-trips |
| 4 | `gate_evaluate` capstone — diff-aware headless evaluation | 2.5 weeks | Subprocess model: parse structured JSON, lenient deserialization |

### Radical Seeds (low-effort v0.4.0 versions)

| Feature | v0.4.0 Scope | Mechanism |
|---------|-------------|-----------|
| `gate_health` | MCP tool: per-criterion pass rates, failure flags | History aggregation over JSON |
| `extends:` spec inheritance | Single-level criteria inheritance | Spec TOML field |
| Pruning metadata in history | `pruning_context` field on `GateRunRecord` | Log alongside failures |
| `gate_sanity` | CLI command: verify gates can actually fail | Worktree + broken-state check |
| History summary | `gate_history --summary` flag | In-memory Rust aggregation |

---

## Deferred / Dropped

| Item | Reason | When |
|------|--------|------|
| Real-time gate output streaming | Wrong transport model (MCP stdio vs SSE) | v0.5.0 |
| Gate DAG (multi-level composition) | Complexity; single-level `extends:` suffices | v0.4.1 opportunistic |
| Criterion-level retry (`max_attempts`) | Independent, no v0.4.0 dependency | v0.4.1 |
| Predictive failure modeling | Cold-start problem; needs 100+ run corpus | v1.0 |
| Causal context analysis | Correlation ≠ causation; overclaims | v0.5.0+ |
| Agent-driven mutation testing | `cargo mutants` territory | v0.5.0+ |
| Federation / community benchmarks | Business/product decision, not engineering | Indefinite |
| Standalone gate dry-run mode | Subsumed by `spec_get` resolved config (QW-5) | Dropped |
| Adaptive/self-amending specs | Creates incentive to weaken standards | Merged into `gate_health` |

---

## Cross-Cutting Themes

1. **Gates-as-data**: The radical pair's strongest surviving insight. v0.4.0 should build the data foundation (instrument, aggregate, make queryable) so v1.0 intelligence features have material to work with.

2. **Subprocess model, not MCP round-trips**: `gate_evaluate` spawns Claude Code `--print --output-format json` and parses structured output. The evaluator never calls MCP tools. Session management within `gate_evaluate` uses direct Rust function calls, not MCP round-trips.

3. **Schema-first design**: Define `EvaluatorOutput` JSON schema before prompt engineering. Anchor the prompt to the schema. Lenient `serde_json::Value` intermediate parse.

4. **Observability as first-class**: `warnings` field on responses, growth rate metrics, outcome-filtered history, resolved config visibility — agents in headless mode can't read logs.

5. **Context engine stays in workspace**: Don't extract until Smelt actually consumes it. Path deps are fragile, git deps require SHA coordination, crates.io requires stable API.

---

## Recommended Implementation Order

```
Week 1:  spec_validate (no deps)
         + context engine crate (parallel)
         + WorkSession persistence (parallel)
         + Quick wins QW-6, QW-3, QW-2 (parallel, independent)

Week 2:  context engine complete
         WorkSession continuing (recovery logic)
         Quick wins QW-4, QW-5, QW-7

Week 3:  WorkSession complete
         gate_evaluate starts
         Quick win QW-1 (diff context on sessions)
         Radical seeds: gate_health, extends:, pruning metadata

Week 4-5: gate_evaluate prompt engineering + lenient parsing
           gate_sanity, gate_history --summary
           Integration tests
```

---

## Key Architectural Decisions to Carry Forward

1. `gate_evaluate` uses subprocess model (parse JSON output), not second MCP transport
2. `EvaluatorOutput` schema defined before prompt engineering
3. `WorkSession` (on-disk) is distinct from `AgentSession` (in-memory v0.3.0)
4. Session management within `gate_evaluate` is Rust function calls, not MCP round-trips
5. `skip_reason` on `CriterionResult`, not `GateKind::Skipped` (preserves counting semantics)
6. Context engine as workspace crate, extract to standalone repo only when Smelt needs it

---

*Synthesized from 3 explorer/challenger brainstorm pairs — 2026-03-10*
