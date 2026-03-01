# Brainstorm Summary

**Date:** 2026-02-28
**Session:** 3 explorer/challenger pairs, 3 rounds of debate each
**Subject:** Assay first milestone scoping — what goes into v0.1?

---

## Quick Wins (Foundation Items)

7 proposals, ~13-16 hours total. Ordered by dependency chain.

| # | Proposal | Effort | Key Decision |
|---|----------|--------|-------------|
| 1 | Error Types + Result Alias | 1.5hrs | `AssayError` with `#[non_exhaustive]`, start with `Io` only, add variants per-consumer |
| 2 | Domain Model Hardening | 3-4hrs | `GateKind` enum with `#[serde(tag = "type")]`, `GateResult` with stdout/stderr evidence, types stay pub DTOs |
| 3 | Schema Generation Pipeline | 1hr | Standalone example binary + `just schemas` |
| 4 | Config Loading (Core Only) | 1.5-2hrs | Free functions in assay-core, `toml` dep on core only, `from_str()` for testability |
| 5 | Spec + Config Validation | 1.5-2hrs | Free functions in assay-core, trim-then-validate, structured `Validation { field, message }` error |
| 6 | Gate Evaluation (Sync) | 2-2.5hrs | Explicit `working_dir` param, sync with doc'd async guidance, `GateResult` with evidence |
| 7 | CLI Subcommands | 2-3hrs | `init` + `validate` + `gate run`, thin delegation to core, template-based init |

**Key insight:** Build all core logic first as free functions in assay-core, then wire CLI as a final capstone. Previous brainstorm interleaved core + CLI.

**Design principles established:**
1. assay-types = pub DTOs, zero logic
2. assay-core = free functions, all behavior
3. CLI = thin last-mile wiring
4. Add error variants when consumed, not speculatively
5. Config (Gate) ≠ State (GateResult) — never mix them

[Full report](quickwins-report.md)

---

## High-Value Features (Milestone Scope)

The central question: ship the north star in v0.1 or build foundation first?

**Answer: Ship the north star in v0.1 — a thin vertical slice.**

| Component | v0.1 Scope | CLI Surface | MCP Surface |
|-----------|-----------|-------------|-------------|
| Error types | Unified `AssayError` | — | — |
| Config | `.assay/config.toml` + `.assay/specs/` | `assay init` | — |
| Specs | TOML files with criteria (optional `cmd`) | `assay spec show` | `spec/get` |
| Gates | Command gates only, structured `GateResult` | `assay gate run` | `gate/run` |
| MCP server | stdio via `rmcp`, 2 tools | `assay mcp serve` | `spec/get`, `gate/run` |
| Plugin | `.mcp.json` + CLAUDE.md snippet | — | — |

**Key debate outcome:** MCP belongs in milestone 1. Without it, Assay is a CLI task runner with no differentiator. Four arguments settled it:
1. Previous brainstorm already converged on "agents first, not last"
2. CLI-only has no market differentiator
3. MCP is ~200-400 LOC thin transport
4. Two consumers reveal abstraction failures early

**Risk mitigation:** Days 1-2 MCP spike with `rmcp`. GO/NO-GO decision. Fallback: CLI-only v0.1, MCP in v0.2.

**Timeline:** 4 weeks (revised from 3 after quality bar discussion).

**Criteria design:** Optional `cmd` field on criteria. Forward-compatible with dual-track: `cmd` = deterministic, `prompt` = agent-evaluated (v0.2).

[Full report](highvalue-report.md)

---

## Radical / Paradigm Shifts

6 proposals explored, 4 killed/deferred, 2 structural insights survived.

| # | Proposal | Outcome | Key Insight |
|---|----------|---------|-------------|
| 1 | Plugin-First / Outside-In | **Reframed** | Plugin as parallel research instrument, not precursor |
| 2 | Dual-Track Gate Demo | **Adopted** | Prove the differentiator first — gates before config |
| 3 | MCP Server First | **Killed** | Transport ≠ product |
| 4 | Top-Down Orchestrator | **Deferred** | Inverted difficulty ordering |
| 5 | Dogfooding Spec | **Deferred to M2** | Circular dependency fatal in M1 |
| 6 | Schema-Driven Everything | **Killed** | Contradicts design-for-extraction |

**Proposed M1 structure (3 tracks):**
- **Track A (Rust, ~1.5-2 weeks):** error types → gate types → deterministic eval → `assay check`
- **Track B (Plugin research, ~2-3 days parallel):** prototype to learn agent UX patterns
- **Track C (Agent gate, ~3-5 days after A):** experimental agent-evaluated criteria via subprocess

**Critical framing:** M1 is a FOUNDATION milestone (internal proof). M2 is a LAUNCH milestone (external demo). This resolves the tension between "ship differentiator early" and "ship something compelling."

**Honest agent invocation scoping:** Clean contract (`AgentEvalRequest`/`AgentEvalResponse` in types), ugly transport (subprocess, gets replaced), proper isolation (`--features agent-eval`).

[Full report](radical-report.md)

---

## Cross-Cutting Themes

1. **Gates are the product.** All three pairs converged on gates as the category-defining feature. Quick wins sequenced gate evaluation as a core deliverable. High-value put gate/run in both CLI and MCP. Radical said ship gates FIRST before anything else.

2. **Two competing visions for v0.1 that need reconciliation:**
   - **High-value pair:** Thin vertical slice (config → specs → gates → MCP → plugin) — 4 weeks
   - **Radical pair:** Gates-first with experimental agent eval — ~2-3 weeks, defer config/specs/MCP to M2
   - Both are viable. The key question: does v0.1 need to demonstrate the full agent workflow, or just prove the differentiator works?

3. **Core logic before surfaces.** Quick wins pair established that all domain logic should be free functions in assay-core, with CLI/MCP/TUI as thin wrappers. This was independently validated by the high-value pair's two-consumer architecture argument.

4. **Types are DTOs, behavior lives elsewhere.** Consistent across all pairs: assay-types stays pub fields, no logic. Validation, config loading, gate evaluation — all free functions in assay-core.

5. **Agent interaction is the hardest unsolved problem.** The radical pair was the only one to scope it honestly. The resolution — clean contract, ugly transport, feature flag — is the pragmatic path forward.

6. **Plugin research as parallel track.** Novel process innovation from the radical pair. Learning agent UX patterns by building a throwaway plugin alongside the real Rust implementation.

---

## The Decision Point

The brainstorm surfaced two viable paths for v0.1:

### Path A: Vertical Slice (from High-Value pair)
- Full loop: init → specs → gates → MCP → plugin
- 4 weeks, higher scope, compelling external demo
- Risk: MCP dependency (mitigated by spike)

### Path B: Gates First (from Radical pair)
- Focused: error types → gate types → deterministic eval → `assay check`
- 2-3 weeks, lower scope, internal proof of concept
- Plus experimental agent eval track
- Risk: "just gates" may not feel like a milestone

### Path C: Hybrid
- Combine quick wins foundation + high-value vertical slice + radical's plugin research track
- Foundation (quick wins 1-6) → MCP spike → vertical slice integration → plugin research findings
- 4 weeks with highest coverage

**Recommendation:** This is a user decision. Path A ships the most compelling product. Path B ships fastest with lowest risk. Path C is most thorough but most ambitious.

---

## What All Pairs Agreed On (Deferred to M2+)

- Markdown spec bodies / spec lifecycle
- File/Threshold/Composite/Agent gates (as production features)
- Workflow state machine
- Structured review system
- TUI features beyond skeleton
- Plugin SDK
- Spec dependencies/versioning
- Dogfooding (Assay validates itself)
