# M003: Conflict Resolution & Polish — Context

**Gathered:** 2026-03-16
**Status:** Future milestone — detail planning deferred until M002 completes

## Project Description

M003 adds AI-powered conflict resolution, additional harness adapters (Codex, OpenCode), and type unification across the session hierarchy. This is the polish milestone that makes the multi-agent system production-grade.

## Why This Milestone

M002 merges sessions sequentially, but merge conflicts are inevitable when agents modify overlapping files. M003 adds AI conflict resolution — when a merge conflict arises, an evaluator agent resolves it. Additional adapters broaden the harness beyond Claude Code. Type unification reduces the cognitive load of 5+ session concepts.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Have merge conflicts automatically resolved by an AI evaluator
- Use Codex or OpenCode as alternative agent harnesses
- See a unified session model across all Assay tools

### Entry point / environment

- Entry point: same pipeline as M001/M002, with conflict resolution as an automatic pipeline stage
- Environment: local dev
- Live dependencies involved: `git` CLI, `claude` CLI, `codex` CLI (optional), `opencode` CLI (optional)

## Completion Class

- Contract complete means: conflict resolution produces valid merge results, adapter stubs compile
- Integration complete means: a real merge conflict is resolved by AI, and at least one non-Claude adapter works
- Operational complete means: none beyond M002's operational requirements

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- A multi-session orchestration with overlapping file changes produces a merge conflict that is automatically resolved
- At least one non-Claude-Code adapter can launch an agent and produce results through the pipeline
- SessionCore composition correctly unifies common fields across session types

## Risks and Unknowns

- AI conflict resolution quality — may produce subtly wrong merges
- Codex/OpenCode CLI interfaces may differ significantly from Claude Code
- SessionCore `#[serde(flatten)]` may have edge cases with `deny_unknown_fields`

## Existing Codebase / Prior Art

- Everything from M001 and M002
- Brainstorm migration report recommends `SessionCore` with `#[serde(flatten)]`
- Brainstorm architecture report notes evaluator.rs unification deferred until second adapter materializes

> See `.kata/DECISIONS.md` for all architectural and pattern decisions.

## Relevant Requirements

- R024: Codex and OpenCode harness adapters
- R025: SessionCore struct composition for type unification
- R026: AI conflict resolution via evaluator

## Scope

### In Scope

- AI conflict resolution via evaluator agent
- Codex harness adapter
- OpenCode harness adapter (stub if CLI not stable enough)
- SessionCore struct composition
- Cupel integration for orchestrated sessions

### Out of Scope / Non-Goals

- Full kanban TUI
- Plugin framework
- PR lifecycle management beyond merge propose

## Technical Constraints

- Adapters follow the closure/callback pattern established in M001
- SessionCore uses `#[serde(flatten)]` — test against `deny_unknown_fields`
- Conflict resolution is an optional pipeline stage, not mandatory

## Open Questions

- What quality bar for AI conflict resolution? — Decide during M003 discuss phase
- Should SessionCore be a breaking change or additive? — Decide based on M002 API surface
