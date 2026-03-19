# Sequencing Proposals for Assay Platform Expansion

## Context

Assay is at v0.4.0 (33,462 lines, 836 tests, 5 crates). v0.4.1 adds lightweight merge tools (`merge_check`, `merge_propose`). The pivot absorbs Smelt's orchestration into Assay, adds `assay-harness` for multi-agent prompt/settings management, and redirects Smelt to infrastructure-only.

**Current architecture strengths to build on:**
- `WorkSession` already has a phase state machine (Created → AgentRunning → GateEvaluated → Completed)
- Worktree lifecycle is in `assay-core::worktree` (create, list, status, cleanup)
- Gate evaluation via headless subprocess (`assay-core::evaluator`) — the evaluator pattern generalizes
- MCP server exposes 17 tools — proven extension surface
- `assay-types` is clean (serde + schemars, no business logic)

---

## Proposal 1: "Bottom-Up Absorption" (User's Proposed Ordering)

### What
1. **v0.4.1** — Merge Tools (phases 46-50): `merge_check`, `merge_propose`, worktree fixes
2. **v0.5.0** — Smelt Orchestration Absorption: worktree manager upgrade, session runner + manifest, merge pipeline (MergeRunner), DAG executor, scope isolation
3. **v0.6.0** — Harness Crate: HarnessProfile, HarnessAdapter trait, layered prompt builder, layered settings, hook contracts, Claude Code adapter
4. **v0.7.0** — Smelt Infrastructure Pivot: Smelt narrows to containers/pods/envs/credentials
5. **v0.8.0** — Cupel Integration: context optimization as a first-class Assay concern

### Why
Natural layering — each milestone builds on the previous. Merge tools prove the git surface, orchestration replaces manual workflows, harness wraps orchestration for multi-agent use. Lowest integration risk since nothing skips a layer.

### Scope
- v0.4.1: ~5 phases, 2-3 days (already planned)
- v0.5.0: ~8-10 phases, 7-10 days (heaviest lift — DAG executor, MergeRunner, session unification)
- v0.6.0: ~6-8 phases, 5-7 days (new crate, trait design, prompt layering)
- v0.7.0: ~3-4 phases, 2-3 days (Smelt-side extraction)
- v0.8.0: ~2-3 phases, 1-2 days (Cupel already has stable API)

### Risks
- **v0.5.0 is a monolith** — absorbing all of Smelt's orchestration in one milestone is the riskiest single step. If it takes longer than expected, downstream milestones slip.
- **Harness is blocked for too long** — can't validate the multi-agent story until v0.6.0, which means ~3 weeks without user-facing harness features.
- **Smelt pivot depends on full absorption** — v0.7.0 can't start until v0.5.0 is done; if orchestration absorption is incomplete, Smelt stays in limbo.

---

## Proposal 2: "Parallel Tracks" — Harness Early, Orchestration Incremental

### What
1. **v0.4.1** — Merge Tools (unchanged)
2. **v0.5.0** — Harness Foundation + Worktree Upgrade:
   - `assay-harness` crate with `HarnessAdapter` trait and `HarnessProfile` types
   - Absorb Smelt's worktree manager (orphan detection, collision prevention, session-aware lifecycle)
   - Claude Code adapter (first concrete adapter)
   - Layered prompt builder (project → spec layers only — no orchestration layer yet)
3. **v0.5.1** — Session Unification + Manifest System:
   - Unify SessionManifest TOML format (single + multi-session)
   - Absorb SessionRunner
   - Add orchestration layer to prompt builder
4. **v0.6.0** — Merge Pipeline + DAG:
   - MergeRunner absorption (sequential merge, AI conflict resolution, human fallback)
   - DAG executor/orchestrator
   - Scope isolation as gate checks
   - Full harness orchestration constraints layer
5. **v0.7.0** — Smelt Pivot + Cupel Integration (combined — both are now unblocked)

### Why
Getting `assay-harness` into developers' hands early validates the adapter abstraction before the full orchestration stack is ready. Worktree upgrades are a natural pairing because harness needs reliable worktree management. Splits the v0.5.0 monolith into two manageable milestones.

### Scope
- v0.5.0: ~6-7 phases, 5-6 days (new crate + worktree upgrade)
- v0.5.1: ~4-5 phases, 3-4 days (session work)
- v0.6.0: ~6-8 phases, 5-7 days (merge + DAG — still the heaviest)
- v0.7.0: ~4-5 phases, 3-4 days (Smelt pivot + Cupel combined)

### Risks
- **Harness without orchestration is a half-story** — the Claude Code adapter can only manage single-agent flows until v0.6.0. Could feel premature.
- **Prompt layering refactored twice** — adding orchestration layer in v0.5.1 means revisiting prompt builder.
- **Session unification is a natural unit with worktree** — splitting them creates integration seams.

---

## Proposal 3: "Thin Vertical Slices" — Feature-Complete Narrow Paths

### What
1. **v0.4.1** — Merge Tools (unchanged)
2. **v0.5.0** — Single-Agent Harness End-to-End:
   - `assay-harness` crate with types + Claude Code adapter
   - Absorb Smelt worktree manager
   - Single-session manifest
   - One complete flow: spec → worktree → agent → gate → merge (for one agent)
3. **v0.6.0** — Multi-Agent Orchestration:
   - DAG executor from Smelt
   - Multi-session manifest
   - MergeRunner with sequential merge
   - Scope isolation as gate checks
   - Harness orchestration layer (multi-agent prompts/settings)
4. **v0.6.1** — Conflict Resolution + Polish:
   - AI conflict resolution, human fallback
   - Smelt pivot (extract infrastructure)
   - Cupel integration
   - Codex/OpenCode adapter stubs

### Why
Each milestone delivers a complete, demo-able capability. v0.5.0 proves the single-agent story end-to-end. v0.6.0 adds the multi-agent dimension. No milestone is "just plumbing" — every one has user-visible value. This is the Assay v0.1.0 philosophy (thin vertical slice) applied to the expansion.

### Scope
- v0.5.0: ~8-10 phases, 6-8 days (crosses boundaries but produces a complete flow)
- v0.6.0: ~8-10 phases, 7-10 days (multi-agent is inherently complex)
- v0.6.1: ~5-6 phases, 3-5 days (polish + pivots)

### Risks
- **v0.5.0 crosses too many boundaries** — touching harness, worktree, session, and merge in one milestone is ambitious. Different from "monolith" — it's narrow but deep.
- **Partial Smelt absorption in v0.5.0** — some Smelt code absorbed, some not, creating a fork maintenance burden.
- **MergeRunner split** — basic merge in v0.5.0 (from `merge_propose`), full MergeRunner in v0.6.0 creates potential rework.

---

## Proposal 4: "Types-First Expansion" — Schema Before Implementation

### What
1. **v0.4.1** — Merge Tools (unchanged)
2. **v0.5.0** — Expansion Type Foundation:
   - Add all new types to `assay-types`: `HarnessProfile`, `HarnessAdapter` trait shape (as types), `SessionManifest`, `DAGNode`, `MergeStrategy`, `ScopeConstraint`, `OrchestratorConfig`
   - Schema generation for all new types
   - No implementation — just the type contracts
3. **v0.5.1** — Worktree + Session Absorption (against stable types):
   - Absorb Smelt worktree manager into `assay-core::worktree`
   - SessionRunner + manifest implementation against `SessionManifest` type
   - Session unification
4. **v0.6.0** — Harness + Orchestration (against stable types):
   - `assay-harness` crate implementing `HarnessProfile`/`HarnessAdapter`
   - DAG executor implementing `DAGNode` types
   - MergeRunner implementing `MergeStrategy` types
   - Scope isolation implementing `ScopeConstraint` types
5. **v0.6.1** — Integration + Pivots:
   - Claude Code + Codex adapters
   - Smelt infrastructure pivot
   - Cupel integration

### Why
Assay's strongest pattern is types-first (PROJECT.md: "Start with domain model before any UI/orchestration — wrong types means rewriting everything"). This approach locks the expansion's API surface early, enabling parallel work on implementation. Every previous milestone validated this — types have been stable through 45 phases.

### Scope
- v0.5.0: ~3-4 phases, 2-3 days (types only — fast)
- v0.5.1: ~5-6 phases, 4-5 days (worktree + session impl)
- v0.6.0: ~8-10 phases, 7-9 days (harness + orchestration — parallel tracks)
- v0.6.1: ~4-5 phases, 3-4 days (integration)

### Risks
- **Types without implementation are speculative** — we might get the types wrong because we haven't implemented against them yet. Smelt's types evolved through usage; Assay would be guessing.
- **Analysis paralysis** — debating type shapes without concrete use cases can slow down v0.5.0 despite its small scope.
- **Integration risk deferred** — all the hard integration work lands in v0.6.0, which becomes the bottleneck.

---

## Proposal 5: "Smelt Migration Sprint" — Fast Absorption, Then Build

### What
1. **v0.4.1** — Merge Tools (unchanged)
2. **v0.5.0** — Aggressive Smelt Absorption (2-week sprint):
   - Absorb ALL of Smelt's orchestration in one focused sprint
   - Worktree manager, SessionRunner, MergeRunner, DAG executor, scope isolation
   - Smelt immediately pivots to infrastructure-only
   - Session/manifest unification
3. **v0.6.0** — Harness + Adapters (building on absorbed orchestration):
   - `assay-harness` with full orchestration context available
   - All prompt/settings layers (project → spec → workflow → orchestration)
   - Claude Code adapter, Codex adapter stub
   - Hook contracts
4. **v0.6.1** — Cupel + Polish:
   - Cupel integration
   - Additional adapters
   - End-to-end workflow validation

### Why
Rip the band-aid off. Smelt absorption is the hardest, most uncertain work — do it first while the team has full context on both codebases. Once absorbed, Smelt is free to pivot immediately (no limbo period). Harness then builds on a complete orchestration layer rather than a partial one.

### Scope
- v0.5.0: ~12-15 phases, 10-14 days (aggressive but focused)
- v0.6.0: ~6-8 phases, 5-7 days (clear scope, stable foundation)
- v0.6.1: ~3-4 phases, 2-3 days (polish)

### Risks
- **v0.5.0 is huge** — 12-15 phases is 50% larger than the biggest milestone so far (v0.2.0 at 15 phases took a full week). Risk of burnout or quality drop.
- **No validation until v0.6.0** — absorbed orchestration isn't user-testable until harness adapters exist.
- **Big-bang integration** — absorbing everything at once means bugs surface simultaneously rather than incrementally.
- **Smelt pivot too early** — if absorption has gaps, Smelt can't backfill because it's already pivoted.

---

## Comparative Summary

| Proposal | Milestones | Heaviest MS | Harness Available | Smelt Freed | Key Bet |
|----------|-----------|-------------|-------------------|-------------|---------|
| 1. Bottom-Up | 5 | v0.5.0 (10d) | v0.6.0 (~week 4) | v0.7.0 (~week 5) | Layer discipline |
| 2. Parallel Tracks | 4 | v0.6.0 (7d) | v0.5.0 (~week 2) | v0.7.0 (~week 4) | Early adapter feedback |
| 3. Vertical Slices | 3 | v0.6.0 (10d) | v0.5.0 (~week 2) | v0.6.1 (~week 5) | Demo-able increments |
| 4. Types-First | 4 | v0.6.0 (9d) | v0.6.0 (~week 4) | v0.6.1 (~week 5) | Type stability |
| 5. Migration Sprint | 3 | v0.5.0 (14d) | v0.6.0 (~week 4) | v0.5.0 (~week 2) | Speed over safety |

## My Recommendation

**Proposal 3 (Thin Vertical Slices)** is the fastest safe path. It mirrors Assay's proven development philosophy (v0.1.0 was a thin vertical slice that proved the concept), delivers user-visible value at every milestone, and avoids the monolithic absorption risk of proposals 1 and 5. The single-agent end-to-end flow in v0.5.0 validates the critical harness + worktree + gate + merge integration before attempting multi-agent orchestration.

**Proposal 2 (Parallel Tracks)** is a strong alternative if we believe adapter feedback is more valuable than end-to-end flow validation.
