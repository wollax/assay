# Dogfooding & Minimum Viable Loop Ideas

**Explorer:** explorer-dogfood
**Date:** 2026-03-10

---

## Idea 1: Spec-Gated v0.4.0 — Use Existing Specs to Gate Own Development

### What

Write assay specs for each v0.4.0 phase (35-45) and use assay's own `gate_run` + stop-hook to enforce them during development. Instead of waiting for the full orchestration loop to be built, use the primitives that already exist (specs, gates, worktrees, MCP tools, Claude Code plugin) as the development workflow for building the rest of assay.

Concretely:
- Create `.assay/specs/phase-35-observability.toml` with deterministic criteria (compile, test, clippy) AND descriptive criteria capturing the phase's success criteria
- The existing stop-hook (`stop-gate-check.sh`) already blocks Claude Code from stopping when gates fail
- The existing `self-check.toml` already gates formatting/linting/tests
- Add per-phase specs that capture phase-specific requirements (e.g., "gate_history accepts outcome parameter")

### Why

**This requires zero new code.** Everything needed already exists:
- Spec parsing and evaluation: `spec_get`, `gate_run`
- Stop-hook enforcement: blocks agent completion until gates pass
- Worktree isolation: `worktree create/cleanup`
- Plugin integration: skills + hooks already wired

This is the fastest path to "assay manages assay" because it uses v0.3.0 capabilities as-is. It also stress-tests the spec format, gate evaluation, and plugin hooks under real conditions — any friction discovered becomes a v0.4.0 improvement.

### Scope

~2-4 hours to write 11 phase specs. Zero code changes.

### Risks

- Specs are currently limited to deterministic shell commands + descriptive AgentReport criteria. The descriptive criteria can't be auto-evaluated until `gate_evaluate` ships (Phase 43). This means the "agent-evaluated" half of the dual-track is advisory-only during dogfooding.
- Writing specs that are too detailed creates maintenance burden when requirements evolve.
- Risk of "spec theater" — writing specs that pass trivially and don't actually catch real issues.

---

## Idea 2: Worktree-Per-Phase Development — Use Assay Worktrees for Assay Development

### What

Instead of developing on `main` or manually-created branches, use `assay worktree create <phase-spec>` to create isolated worktrees for each v0.4.0 phase. This exercises the worktree lifecycle (create → develop → gate → cleanup) using assay's own tooling.

The flow:
1. `assay worktree create phase-35-observability` → creates worktree + branch
2. Developer/agent works in the worktree
3. `assay gate run phase-35-observability` → validates work
4. Manual merge back (until merge-back is built in v0.5.0)
5. `assay worktree cleanup phase-35-observability` → removes worktree

### Why

This tests worktree management under real multi-phase conditions. Today worktrees exist as a feature but haven't been used in anger for multi-phase development. Using them to build v0.4.0 would reveal:
- Path resolution bugs when specs reference parent project files from worktree
- Worktree status accuracy (ahead/behind counts — FIX-01 is literally a v0.4.0 requirement)
- Cleanup reliability after merge
- The UX gap of "no merge-back" — which directly informs v0.5.0 priorities

### Scope

~1 hour setup. Ongoing discipline to use the workflow. Zero code changes.

### Risks

- Manual merge-back is friction. Without automated merge, developers might abandon the workflow after the novelty wears off.
- Worktree-per-phase may be overkill for single-developer projects — the isolation benefit is strongest when multiple agents work concurrently.
- If a worktree bug blocks development, you're blocked on fixing a feature while trying to use it (bootstrap paradox).

---

## Idea 3: "Assay Bootstraps Assay" Progressive Strategy

### What

A phased approach to self-hosting where each v0.4.0 phase adds to assay's own dogfooding capability:

**Phase A (now, v0.3.0 primitives):** Spec-gated development (Idea 1) + worktree workflow (Idea 2). Manual agent launching, manual merging.

**Phase B (after Phase 40-41, sessions land):** Add WorkSession tracking to assay's own development. Each phase gets a session: `session_create` → work → `session_update` with gate run references. This provides structured history of how phases were developed.

**Phase C (after Phase 43, gate_evaluate lands):** Use `gate_evaluate` to auto-evaluate agent criteria on assay's own phase specs. The agent-evaluated track is no longer advisory-only — it actually runs headless evaluation.

**Phase D (v0.5.0, merge-back):** Full loop. Specs → worktree → agent → gate → merge, all managed by assay.

### Why

This avoids the "big bang" dogfooding problem. Instead of waiting until the full loop exists to start eating your own dog food, you start with what you have and progressively add capability. Each phase's dogfooding experience directly validates the feature being built.

The strategic insight: **dogfooding doesn't require the full loop.** The value curve is:
- Phase A (specs + worktrees) captures ~40% of the value (quality enforcement, isolation)
- Phase B (sessions) adds ~20% (structured tracking, history)
- Phase C (gate_evaluate) adds ~25% (automated agent evaluation)
- Phase D (merge-back) adds ~15% (full automation)

This means you get 40% of the dogfooding value with zero new code today.

### Scope

Phase A: 2-4 hours (spec writing)
Phase B: built into Phase 40-41 development naturally
Phase C: built into Phase 43 development naturally
Phase D: v0.5.0 scope

### Risks

- "Progressive" can become "we'll do it later" if there's no commitment. Phase A needs to happen immediately, not "after we plan it more."
- Each phase transition requires updating specs and workflow, which is overhead.
- The phases map linearly to v0.4.0 roadmap phases — if the roadmap changes, the dogfooding strategy may need to adapt.

---

## Idea 4: External Project Dogfooding — Use Assay on a Real Side Project

### What

Instead of (or in addition to) using assay on itself, pick a small external project and use assay to manage its development. This could be a Rust CLI tool, a small web project, or even a plugin for another tool. The key: it's a *different* codebase with *different* specs, exercising assay as a user would.

Candidates:
- A new Kata skill or Claude Code plugin (small, well-defined scope)
- An open-source Rust utility (exercises the general-purpose workflow)
- An assay plugin for another agent system (Codex, OpenCode — exercises the plugin ecosystem)

### Why

Self-referential dogfooding has a blind spot: you know how your tool works, so you unconsciously work around its limitations. An external project reveals UX friction that self-hosting masks:
- Spec authoring pain points (what's missing from the TOML format?)
- Gate configuration verbosity (too many fields? too few?)
- MCP tool discoverability (do agents naturally find and use the tools?)
- Onboarding experience (what does a new user need to know?)

This is the "real" dogfooding — using the tool as your target user would.

### Scope

~4-8 hours to set up assay on a side project and develop one feature with it.

### Risks

- Scope creep: the side project becomes interesting and steals time from v0.4.0.
- Too early: v0.3.0 may not be polished enough for external use, creating a discouraging experience.
- Selection bias: choosing a project that happens to fit assay's current limitations rather than one that exposes gaps.

---

## Idea 5: Milestone Versioning — v0.4.0 Scope Adjustment Based on Dogfooding Priority

### What

Restructure the remaining work based on when each feature enables dogfooding, rather than the current technical-dependency order:

**Option A: Keep v0.4.0 as-is, add dogfooding overlay.** Don't change the roadmap. Just add phase specs and use existing primitives (Idea 1+2). v0.4.0 ships as planned.

**Option B: Split v0.4.0 — ship "verify" before "plan".** Current v0.4.0 focuses on the verify/test part of the loop (gate_evaluate, sessions). This is correct — it completes the middle of the loop. The planning end (spec authoring, plan decomposition) and merge end (branch strategy, conflict resolution) become v0.5.0 and v0.6.0 respectively.

Split:
- v0.4.0: Headless Orchestration (as planned — 11 phases)
- v0.5.0: Merge-Back Workflow (branch strategy, conflict resolution, merge gates)
- v0.6.0: Planning Primitives (spec authoring tools, plan decomposition, spec-from-issue)

**Option C: Pull merge-back into v0.4.0, defer some observability.** The merge-back gap is the biggest friction in dogfooding (Idea 2). If worktree-per-phase is the strategy, then automated merge-back has higher dogfooding value than, say, growth rate metrics (OBS-04) or spec cross-dependency validation (SPEC-04).

Swaps:
- Pull in: `worktree merge <spec>` basic merge command
- Defer to v0.4.1: OBS-04 growth rate metrics, SPEC-04 cross-spec dependencies

### Why

The question "should v0.4.0 change?" depends on what we optimize for:
- **Technical correctness** → Option A (current plan is sound)
- **Fastest path to full loop** → Option B (verify first, plan and merge next)
- **Maximum dogfooding value** → Option C (merge-back removes the biggest friction)

My recommendation: **Option A with Idea 3's progressive overlay.** The v0.4.0 roadmap is well-scoped and the brainstorm that produced it was thorough. Changing scope now to add merge-back would delay the headless evaluation capstone (gate_evaluate), which is the project's core differentiator. Better to dogfood with manual merging for one milestone and ship merge-back in v0.5.0 when the full loop picture is clearer.

### Scope

Option A: 0 hours (no change)
Option B: 2-4 hours to restructure roadmap
Option C: 1-2 weeks to implement basic merge command

### Risks

- Option A risk: manual merge friction causes dogfooding abandonment.
- Option B risk: splitting across more milestones means more planning overhead and slower time-to-full-loop.
- Option C risk: pulling merge-back into v0.4.0 increases scope by ~30% and risks shipping none of it well.
