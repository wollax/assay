# Architecture & New Directions: Final Report

**Explorer:** explorer-architecture
**Challenger:** challenger-architecture
**Date:** 2026-02-28
**Rounds:** 3

---

## Corrected Thesis

**Original thesis:** "Token compression should be a first-class architectural concern in Assay."

**Corrected thesis:** "Compact output design should be a default, not a retrofit."

The original thesis led to 6 proposals that would have made compression a larger subsystem than gates, specs, and workflows combined — scope inversion for a product whose differentiator is quality-gated orchestration, not token optimization. The corrected thesis is a design principle, not a feature: it influences how every type and MCP tool is designed across all phases, costs zero lines of additional code, and prevents the need for after-the-fact compression systems.

---

## Proposals Explored and Killed

| # | Proposal | Verdict | Reason for Kill |
|---|----------|---------|-----------------|
| 1 | Token Budgets (full system) | Killed | Over-engineered: `TokenBudget`, `TokenBudgetReport`, budget strategies, escape hatches — 5 new concepts for output that doesn't exist yet. Token counting without a tokenizer is unreliable (20-30% error). The `gate/get_full_output` escape hatch undermines the budget: agents that see "truncated" will call it, spending MORE tokens. |
| 2 | Composable Middleware Pipeline | Killed | Premature abstraction. Directly contradicts the project's "enum dispatch, not trait objects" convention. The gate module is currently 4 lines of doc comments. A `Compressor` trait with `Send + Sync`, pipeline executor, phase tags, and ordering invariants is designing infrastructure for output that doesn't exist. |
| 3 | Bidirectional "Assay Lens" | Killed | Solves a problem that's already handled. MCP has typed tool responses — outbound schema enforcement is just good tool design (already requirement MCP-08), not a new system. The "lens" brand papered over two unrelated mechanisms (inbound pipeline, outbound schema extraction). Real agent output contains qualitative observations that schema extraction would discard. |
| 4 | Context-Pressure-Adaptive (full system) | Killed (concept survived) | The `context_pressure: f32` protocol extension, interpolation curves, and hysteresis are a control system — an entire engineering discipline for a supporting concern. The underlying insight (context is a depleting resource) is valid but collapses into Output Detail Levels driven dynamically by the orchestrator. See Surviving Idea #1. |
| 5 | Information Fidelity Gate | Killed | Practically circular. Structural fidelity (checking fields present after compression) is a unit test for the compressor, not a quality gate — gates evaluate external work quality, not internal pipeline integrity. Semantic fidelity doubles AI costs (one call to compress, one to verify). Backoff-on-failure creates unbounded retry loops. |

### Key Lessons from Killed Proposals

1. **Compression is a supporting concern, not a product.** Six proposals for compression in a project with 41 unfinished requirements and empty implementation modules is scope inversion.
2. **The project's own conventions constrain the solution space.** "Enum dispatch, not trait objects" eliminates trait-based middleware. "Add variants when consumed, not speculatively" eliminates upfront API design. These constraints are features, not limitations.
3. **RTK and Assay don't overlap.** RTK compresses raw shell output at the hook level. Assay produces structured gate results at the application level. Different layers, different concerns. Assay should not reimplement RTK's deterministic filtering.

---

## Surviving Ideas

### 1. Output Detail Levels

**What:** An `OutputDetail` enum on gate configuration that controls how much information `GateResult` includes.

```rust
enum OutputDetail {
    Full,       // everything: stdout, stderr, exit code, duration, all test names
    Standard,   // exit code + failures + summary (passing tests omitted)
    Compact,    // exit code + failure count only
}
```

Configurable per gate in TOML (`detail = "standard"`). The gate evaluation function uses a match arm to decide what to include. Zero runtime overhead, no token counting, no escape hatches.

**Why it survived:**
- Semantic truncation (knows failures > passing test names) is strictly better than blind byte truncation (`max_output_bytes`)
- Enum is consistent with project conventions
- Optional with sensible default (`Full`)
- Forward-compatible with the orchestrator: when it exists, it can set `OutputDetail` dynamically per agent/session. The mechanism already exists as an enum; the orchestrator is just a different actor driving it.

**When:** Add the type in Phase 3 (domain model). Implement rendering branches in Phase 7 (gate evaluation). Default to `Full` everywhere until the orchestrator exists. No dynamic switching in v0.1.

**Scope:** Small — one enum, one field on gate config, one match arm in gate rendering.

**Debate outcome:** Explorer proposed a full `TokenBudget` system with progressive disclosure, budget priorities, and `gate/get_full_output` escape hatches. Challenger killed it as over-engineered and proposed `max_output_bytes`. Explorer pushed back that byte truncation is blind/structural-destroying. Both converged on `OutputDetail` as the middle ground: semantic level selection without the budget machinery.

### 2. Structured Wire Format

**What:** Design MCP tool responses for machine consumption from the start, separate from CLI display format.

- **Wire format (MCP):** Compact but readable field names (`pass`, `exit`, `dur`, `fail`, `total`). No prose, no ANSI, no formatting. Structured JSON designed for agents.
- **Display format (CLI/TUI):** Human-readable rendering of the same underlying data. Pretty-printed, potentially colored, formatted for terminal width.

Both derive from the same `GateResult` struct. Wire format uses `#[serde(rename)]` attributes for compact serialization. Display format uses a separate rendering function.

**Why it survived:**
- Inverts the compression problem: don't compress verbose output, don't be verbose in the first place
- Zero dependencies, zero runtime cost
- Clear separation between machine and human surfaces prevents the "pretty for who?" problem
- Aligns with the project's schemars pipeline: JSON Schema documents the wire format

**Refinements from debate:**
- Explorer originally proposed single-character field names (`p`, `x`, `d`). Challenger correctly identified this as a false economy: field names are a tiny fraction of total token cost compared to actual content (stdout/stderr), and single-char names make every debug session painful. Converged on short-but-readable names: `pass`, `exit`, `dur`, `fail`, `total`.

**When:** Phase 3 (type design with serde attributes) + Phase 8 (MCP tool responses use wire format, CLI uses display format).

**Scope:** Small — serde attributes on types, rendering functions for CLI. No new modules or abstractions.

### 3. Diff-Mode for Iterative Gate Runs

**What:** When an agent calls `gate/run` multiple times for the same spec during a fix cycle, return only what changed since the last call.

**Why it survived (the math):** An agent iterating on a spec with 50 tests, 47 passing, fixing one failure per iteration over 5 cycles receives 235 redundant passing-test entries. Diff-mode sends 47 once and 3 diffs per subsequent call — a 75%+ reduction on the dominant content (test results), not just the envelope (field names).

**Implementation shape:** In-memory `HashMap<(SessionId, SpecName), LastResult>` with TTL cleanup. If session state is lost, fall back to full results. Analogous to HTTP ETag/If-None-Match — a well-understood caching pattern, not novel session management.

**Debate outcome and timing:** Explorer proposed diff-mode as a Phase 8 design goal with a `since` parameter baked into the MCP tool schema from the start. Challenger pushed back on two points:

1. **Don't elevate a nice-to-have into a requirement.** Phase 8 has 6 MCP requirements including existential ones (tool descriptions, spawn_blocking bridge). Diff-mode is an optimization that competes for attention with things the server needs to function.
2. **Don't design the `since` API before seeing real gate output.** You don't know what a "change" means (test name? stdout hash? exit code delta?) until gates produce real results. "Add when consumed, not speculatively."

**Agreed resolution:** Diff-mode is a **design consideration** noted in Phase 8 context, not a requirement. Ship `gate/run` without `since` in v0.1. Observe real agent usage patterns. If repeated calls are a real (not hypothetical) token cost, add diff-mode in v0.2 with API shape informed by actual usage data.

**When:** Design consideration in Phase 8. Implementation in v0.2 if validated by real usage.

**Scope:** Medium when implemented — session-aware MCP handler, diff calculation logic, fallback-to-full on session miss.

---

## Design Principle for All Phases

The most valuable output of this brainstorm is not any specific proposal but a principle:

> **Design for compact output from the start.** Every type, every MCP tool response, every CLI output should be designed with token economy in mind — not compressed after the fact. This means: structured data over prose, separate wire vs. display formats, semantic detail levels over byte truncation, and only sending information the consumer needs.

This principle should influence type design in Phase 3, gate evaluation in Phase 7, and MCP tool responses in Phase 8. It costs zero additional code — it's a design mindset, not a feature.

---

## Summary

| Surviving Idea | Phase | Scope | Key Insight |
|----------------|-------|-------|-------------|
| Output Detail Levels | 3 (type) + 7 (impl) | Small | Semantic verbosity control via enum, not byte truncation |
| Structured Wire Format | 3 (types) + 8 (MCP) | Small | Don't compress — don't be verbose in the first place |
| Diff-Mode | Design consideration Phase 8, implement v0.2 | Medium | Target the dominant cost: redundant repeated results |

**Killed:** 5 of 6 original proposals (token budgets, middleware pipeline, bidirectional lens, context-pressure system, fidelity gate)

**Corrected thesis:** "Compact output design should be a default, not a retrofit."
