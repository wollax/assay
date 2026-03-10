# Dogfooding & Minimum Viable Loop — Final Report

**Explorer:** explorer-dogfood | **Challenger:** challenger-dogfood
**Date:** 2026-03-10
**Rounds:** 2 rounds of debate, converged on Round 2

---

## Executive Summary

Five proposals were explored and pressure-tested. The debate converged on a key distinction: **dogfooding assay's quality gates (lightweight, now) vs. dogfooding assay as a full orchestrator (v0.5.0+, after gate_evaluate ships).** The minimum viable loop already exists with v0.3.0 primitives but delivers limited incremental value over existing checks. The right v0.4.0 posture is a small, structured research experiment — not a workflow commitment.

---

## The Core Question (Resolved)

> Is the goal to dogfood assay's quality gates, or to dogfood assay as a full development orchestrator?

**Answer:** Both, but sequenced:
- **v0.4.0:** Dogfood quality gates as a lightweight research exercise. Goal: design insights for spec_validate (Phase 37) and worktree fixes (Phase 36).
- **v0.5.0:** Dogfood quality gates + merge-back on an external project with gate_evaluate available. Goal: validate the full verify loop.
- **v0.6.0+:** Dogfood orchestration — spec authoring, plan decomposition, session management. Requires a dedicated design brainstorm on the Kata → Assay transition.

---

## Recommendations

### 1. Keep v0.4.0 Scope Unchanged

The 11-phase, 28-requirement roadmap was produced by a thorough brainstorm and is well-scoped. Don't pull in merge-back or planning primitives. Don't defer phases to make room for dogfooding infrastructure. Kata continues orchestrating through v0.4.0.

### 2. Run a Lightweight Dogfooding Experiment (~4 hours)

Write 2-3 experimental phase specs (Phase 35: Observability Foundation, Phase 37: Spec Validation) to exercise spec authoring. Use worktrees for these same phases to exercise worktree ergonomics. Choose phases that don't touch worktree code (avoids bootstrap paradox).

**Critical: include an evaluate-after-first-phase checkpoint.** After completing Phase 35 with the dogfooding overlay:
- Write a 30-minute retrospective (`dogfood-retro.md`)
- Assess: What did spec authoring reveal about TOML format friction? What did worktree usage reveal about ergonomics?
- If findings are actionable → continue for Phase 37
- If findings are "nothing we didn't already know" → drop the experiment, don't sink-cost through remaining phases

### 3. Frame Success as "Design Insights," Not "Bugs Caught"

The v0.4.0 dogfooding experiment is research, not workflow. The specs are not expected to catch bugs that `just ready` + test suite wouldn't catch. The metric is:
- What spec authoring friction was discovered? (Informs Phase 37: spec_validate)
- What worktree ergonomic issues were found? (Informs Phase 36: FIX-01)
- What gate output readability issues surfaced? (Informs Phase 43: gate_evaluate prompt engineering)

Do not evaluate the experiment by "how many failures did dogfooding prevent."

### 4. Identify an External Dogfooding Target Project During v0.4.0

Choose a project with genuine motivation to complete (a real Kata skill, Rust CLI utility, or agent plugin — not a contrived test bed). Don't use assay on it yet during v0.4.0 — the tool is too rough and the experience would be discouraging. Instead, use it as a design compass: "Would this feature make sense for Project X?" shapes development decisions.

Defer actual external dogfooding to v0.5.0 kickoff, when spec_validate, better error messages, and gate_evaluate are available.

### 5. Plan Full Dogfooding for v0.5.0 Kickoff

After gate_evaluate ships (Phase 43), the full dual-track loop becomes available:
- Use assay on the external target project for one feature cycle
- Use assay on its own development with headless gate evaluation
- Use findings to inform v0.5.0 priorities (merge-back, spec authoring, etc.)

### 6. Name the Kata → Assay Transition as an Open Design Question

Kata orchestrates planning, phasing, execution, verification, debugging, milestone management, brainstorming, and PR review (30+ skills). Assay gates quality and is building toward orchestration. These are complementary today, but the PROJECT.md vision implies eventual convergence.

**This transition requires its own brainstorm or design document for v0.6.0+.** Open questions:
- Which Kata capabilities should assay subsume? (Spec authoring? Phase management? Execution?)
- Which should remain external? (Brainstorming? PR review? Debugging?)
- Is the transition gradual (assay acquires capabilities one by one) or structural (assay provides a spec/gate/session layer that Kata consumes)?
- Does assay need a plugin/adapter system to integrate with Kata rather than replace it?

The progressive bootstrap strategy (Idea 3) provides a mental model but does not automatically lead to this transition. Deliberate architectural decisions are required.

---

## What Was Dropped and Why

| Original Proposal | Disposition | Reason |
|---|---|---|
| 11 phase specs mirroring roadmap | **Dropped** | Spec theater — duplicates Kata's PLAN.md success criteria and `just ready` checks. Near-zero incremental enforcement value. |
| Worktree-per-phase as standard workflow | **Dropped** | Bootstrap paradox for worktree-related phases. Overhead without parallelism benefit for single-developer workflow. |
| 40/20/25/15 progressive value curve | **Dropped** | Fabricated numbers. Honest assessment: ~10% value from Phase A, ~50% from Phase C (gate_evaluate). |
| External dogfooding during v0.4.0 | **Deferred to v0.5.0** | Testing a rough product tells you "it's rough" — not useful. ROI improves after spec_validate and gate_evaluate ship. |
| Changing v0.4.0 scope (Options B/C) | **Dropped** | Roadmap is well-scoped. Adding merge-back risks the capstone. Splitting adds planning overhead. |

---

## What Survived and Why

| Recommendation | Survives Because |
|---|---|
| 2-3 experimental specs with retro checkpoint | Costs ~4 hours, produces spec authoring UX data that directly informs Phase 37 (spec_validate). Kill switch prevents waste. |
| Worktree usage for 2 non-worktree phases | Produces ergonomics data for Phase 36 (FIX-01). Avoids bootstrap paradox by choosing safe phases. |
| External project identification (not usage) | Zero-cost design compass. "Would this work for Project X?" improves feature design without running assay on anything. |
| Full dogfooding at v0.5.0 kickoff | Right timing: gate_evaluate available, spec_validate available, tool is mature enough for real use. |
| Kata → Assay transition as open question | Prevents premature assumptions about what "full self-hosting" means. Requires dedicated design work. |

---

## The Minimum Viable Loop (Exists Today)

```
spec (TOML) → worktree create → manual agent work → gate_run → manual merge → worktree cleanup
```

This loop works with v0.3.0 primitives. It's not automated, the agent-evaluated track is advisory-only, and merge-back is manual. But it's sufficient for a research experiment during v0.4.0 and becomes the foundation for full dogfooding when gate_evaluate and merge-back ship.

---

*Consolidated from 5 proposals after 2 rounds of explorer/challenger debate — 2026-03-10*
