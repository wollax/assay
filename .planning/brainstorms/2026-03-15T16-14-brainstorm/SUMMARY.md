# Brainstorm Summary: Assay Platform Expansion

**Date:** 2026-03-15
**Topic:** Absorbing Smelt orchestration + building assay-harness crate
**Pairs:** 3 (sequencing, architecture, migration)
**Rounds:** 2-3 per pair, full convergence on all

---

## Surviving Proposals

### Sequencing: "Thin Vertical Slices" (3 milestones)

| Milestone | Goal | Phases | Duration |
|-----------|------|--------|----------|
| v0.5.0 | Single-agent harness end-to-end: manifest → worktree → agent → gate → merge | 8-10 | 6-8 days |
| v0.6.0 | Multi-agent orchestration: DAG executor, parallel sessions, sequential merge | 8-10 | 7-10 days |
| v0.6.1 | Conflict resolution + Cupel integration + additional adapters | 5-6 | 3-5 days |

**Total:** ~4-5 weeks after v0.4.1 ships.

[Full report](sequencing-report.md)

### Architecture: Core Expansion with Callback Inversion

| Component | Location | Rationale |
|-----------|----------|-----------|
| Orchestration (DAG, merge, manifest, scope) | `assay-core/src/orchestrate/` module | ~2 new modules + extensions, not enough for a crate |
| Harness adapters | New `assay-harness` leaf crate | Implementations depend on core, not vice versa |
| DTOs (HarnessProfile, SessionManifest, etc.) | `assay-types` | Cross-crate via serialization |
| Control inversion | Closures/callbacks, NOT traits | Zero-trait codebase convention preserved |

**Dep graph (unchanged direction):**
```
assay-cli ──→ assay-core ──→ assay-types
assay-tui ──→ assay-core ──→ assay-types
assay-mcp ──→ assay-core ──→ assay-types
assay-harness ──→ assay-core ──→ assay-types  (new leaf)
```

[Full report](architecture-report.md)

### Migration: 4-Phase Incremental Plan

| Phase | What | Why |
|-------|------|-----|
| 0 | AgentSession persistence | In-memory sessions don't survive restart — blocks parallel orchestration |
| 1 | `assay-orchestrator` crate (proof of concept, feature-gated) | Proves mapping without touching existing crates |
| 2 | Additive `orchestrate_*` MCP tools | No changes to existing 18 tools |
| 3 | `SessionCore` struct composition | Unify types after API stabilizes through usage |

[Full report](migration-report.md)

---

## Cross-Cutting Themes

### 1. Composition over extension
All three pairs independently converged on **not modifying** existing types/enums/tools. `OrchestratorSession` wraps `Vec<WorkSession>`. New `orchestrate_*` tools sit alongside existing tools. `SessionCore` uses `#[serde(flatten)]`, not enum variant extension.

### 2. Session vocabulary is overloaded
Five "session" concepts identified: `AgentSession`, `WorkSession`, `SessionPhase`, plus Smelt's `SessionManifest` and `SessionRunner`. Cleanup recommended as first commit:
- `AgentSession` → `GateEvalContext`
- Smelt manifest → `RunManifest`
- Smelt runner → `RunExecutor`

### 3. Serde boundaries are the real constraint
`SessionPhase` rejects unknown variants. Branch naming `assay/{spec_slug}` is hardcoded. These aren't just tech debt — they're architectural walls that determine what can be additive vs. breaking.

### 4. Zero-trait convention is load-bearing
The codebase deliberately avoids traits (0 across 33k lines). All three pairs respected this: closures for control inversion, enum dispatch for variants, struct composition for shared data.

### 5. Smelt pivot is not an Assay milestone
Track Smelt's infrastructure pivot on Smelt's roadmap, unblocked by v0.5.0 absorption. Including it in Assay inflates milestone count with work that doesn't touch Assay's codebase.

---

## Key Architectural Decisions

| # | Decision | Source |
|---|----------|--------|
| 1 | Worktrees stay spec-scoped; session linkage is additive (`session_id: Option<String>`) | Sequencing |
| 2 | `OrchestratorSession` composes `Vec<WorkSession>` — linear state machine preserved | Sequencing |
| 3 | `HarnessAdapter` starts minimal (3 methods), grows via default methods in v0.6.0 | Sequencing |
| 4 | `[[sessions]]` array in manifest from day one (avoids breaking change for multi-agent) | Sequencing |
| 5 | Orchestration absorbed as `assay-core` module, not separate crate | Architecture |
| 6 | Closures/callbacks for control inversion, not traits | Architecture |
| 7 | `evaluator.rs` unification with harness deferred until second adapter materializes | Architecture |
| 8 | `HarnessProfile` in `assay-types`; implementation types stay in `assay-harness` | Architecture |
| 9 | Don't add optional params to existing MCP tools — use new namespaced tools | Migration |
| 10 | Struct composition (`SessionCore`) over trait for type unification | Migration |

---

## Tension Between Reports

The **sequencing** and **migration** reports have a structural disagreement worth noting:

- **Sequencing** recommends orchestration as a module in `assay-core` (aligned with architecture report)
- **Migration** recommends a separate `assay-orchestrator` crate as a proof-of-concept with feature gates, planned for later inlining

**Resolution:** The migration report's phased approach (separate crate → inline) is a **migration strategy**, not a permanent architecture decision. Both agree on the end state (orchestration in assay-core). The question is whether to start there or arrive there. Migration's argument: a separate crate is rollback-safe. Architecture's argument: a module avoids premature crate boundaries. **Recommend: start as module in assay-core** (architecture wins) but **use feature gates** (migration's safety mechanism) — `cfg(feature = "orchestrate")` on the module.

---

## Recommended Sequencing (Synthesized)

Incorporating findings from all three reports:

**Prerequisite (fold into v0.4.1 or v0.5.0 Phase 0):**
- AgentSession persistence (write-through cache)
- Session vocabulary cleanup (`AgentSession` → `GateEvalContext`)

**v0.5.0 — Single-Agent Harness End-to-End**
- `assay-harness` leaf crate with Claude Code adapter
- Minimal `HarnessAdapter` (closures, not trait — 3 callback types)
- Worktree enhancements (orphan detection, collision prevention)
- `RunManifest` with `[[sessions]]` array
- End-to-end: manifest → worktree → agent → gate → merge propose

**v0.6.0 — Multi-Agent Orchestration**
- `assay-core::orchestrate` module (feature-gated)
- `OrchestratorSession` composing `Vec<WorkSession>`
- DAG executor + MergeRunner absorbed from Smelt
- `orchestrate_*` MCP tools (additive, no changes to existing)
- Harness orchestration layer (scope, multi-agent prompts)

**v0.6.1 — Conflict Resolution + Polish**
- AI conflict resolution via evaluator
- Cupel integration for orchestrated sessions
- Codex/OpenCode adapter stubs
- `SessionCore` struct composition for type unification

---

*Synthesized from 3 explorer/challenger pairs — 2026-03-15*
