# Solo Workflow Tighten — Pre-Implementation Review

## Session Goal

Review the `solo-workflow-tighten` change proposal before implementation. Discuss, research, brainstorm, and finalize answers to open questions. Update the design and specs with resolved decisions. When all questions are resolved, the proposal is ready for `/opsx:apply`.

## Context

Read these files in order to get full context:

1. `openspec/explore-solo-workflow.md` — Original exploration with Q&A
2. `openspec/workflow-current-state.md` — Current workflow with Mermaid diagrams
3. `openspec/workflow-desired-state.md` — Desired state with resolved decisions
4. `openspec/changes/solo-workflow-tighten/proposal.md` — What and why
5. `openspec/changes/solo-workflow-tighten/design.md` — How (decisions, risks, open questions)
6. `openspec/changes/solo-workflow-tighten/specs/` — All 9 capability specs (WHEN/THEN scenarios)
7. `openspec/changes/solo-workflow-tighten/tasks.md` — 42 implementation tasks

## Open Questions to Resolve

These are from `design.md § Open Implementation Questions` plus threads that surfaced during exploration:

### Q1: Backward compatibility for spec status field

Existing `gates.toml` files have no `status` field. Two options:

- **Safe:** Default to `draft`. Every existing spec starts as draft regardless of history.
- **Smart:** On first load, check gate history — if a passing run exists, infer `verified`.

Tradeoffs: Safe is simple but means existing passing specs show as "draft" until next gate run. Smart is more correct but adds load-time complexity and couples spec loading to gate history.

### Q2: Skill alias mechanism

The proposal deprecates `/assay:status` → `/assay:focus`, `/assay:gate-check` → `/assay:check`, etc. with a one-version alias period. Questions:

- Do Claude Code plugin skills support aliases natively (one SKILL.md, multiple trigger names)?
- If not, do we create separate SKILL.md files for each old name that just redirect?
- How do Codex and OpenCode handle skill aliases?

Research the actual plugin skill mechanisms for each harness to determine the right approach.

### Q3: Protected branch detection

The branch isolation heuristic needs to know which branches are "protected." Options:

- **Hardcoded defaults:** `["main", "master", "develop"]` — simple, covers 95% of cases
- **Git config aware:** Read `init.defaultBranch` from git config, detect branch protection rules
- **User configurable only:** No defaults, user must set `[workflow] protected_branches`

Consider: is reading git config (`git config init.defaultBranch`) reliable cross-platform? Do forge-level branch protection rules have a local representation? What about monorepos with non-standard default branches?

### Q4: `quick: true` flag vs structural inference

Quick milestones (from `plan quick`) need to be distinguishable. Two approaches:

- **Explicit flag:** `quick = true` field on `Milestone` struct. Clear, queryable, forward-compatible.
- **Structural inference:** Detect "1 chunk where chunk.slug == milestone.slug" pattern. No schema change but fragile (what if someone manually creates a 1-chunk milestone?).

Consider: is there a use case for explicitly creating a 1-chunk milestone that is NOT quick? If so, inference breaks.

### Q5: UAT configuration

The verify phase supports optional UAT (agent-assisted human verification). Where does "UAT enabled" live?

- **Per-spec:** A field in `gates.toml` — allows some specs to require UAT and others not
- **Per-project:** A `[workflow] uat_enabled = true` config — global toggle
- **Both:** Project-level default, spec-level override

Consider: what does the UAT session actually look like? Is it a separate Claude Code session? A TUI screen? A skill? We designed the handoff (spec + gate_run_id) but not the UAT experience itself.

### Q6: Explore phase — what context is loaded and how?

The explore skill "loads project context" but we didn't specify exactly what that means:

- Which files does it read? All specs? Config? Recent gate history? Git log?
- How does it present this to the agent? As a structured summary? Raw file contents?
- Is there a size budget to avoid blowing up the context window?
- Does it load differently when specs exist vs. a fresh project?

### Q7: Spec status and cycle_advance interaction

The current `cycle_advance` runs gates and advances if they pass. With spec status:

- Should `cycle_advance` refuse to run if the spec status is still `draft`? (Enforcing the review step)
- Or should it be permissive — run gates regardless of status, and auto-promote?
- Does this differ between solo (permissive) and full mode (strict)?

## How to Work

1. Read the context files listed above
2. For each question, discuss tradeoffs, research the codebase where needed, and propose a resolution
3. When we agree on an answer, update the relevant artifact (`design.md`, spec files, or `tasks.md`)
4. When all questions are resolved, remove the "Open Implementation Questions" section from `design.md` and replace it with the resolved decisions
5. Final check: re-read `tasks.md` to see if any tasks need updating based on resolved questions

When done: the change is ready for `/opsx:apply`.
