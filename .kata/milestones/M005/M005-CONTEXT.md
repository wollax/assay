# M005: Spec-Driven Development Core — Context

**Gathered:** 2026-03-19
**Status:** Ready for planning

## Project Description

Assay is evolving from a gate runner into a full spec-driven development platform. M005 establishes the foundation: a milestone/chunk hierarchy, a guided wizard for authoring specs from plain-language descriptions, a development cycle state machine, gate-gated PR creation, and upgraded plugins for Claude Code and Codex. This is the layer that makes Assay accessible to beginning developers and drives the agent-first workflow.

## Why This Milestone

M001–M004 built a powerful gate evaluation engine — but using it requires manually writing TOML specs, knowing which commands to run, and managing worktrees by hand. M005 puts a workflow layer on top: the wizard removes the authoring friction, the cycle state machine tells the agent what to work on next, and the PR command closes the loop between verified code and shipped PRs.

Without M005, Assay is a tool for experts. With it, any developer can install Assay, describe a feature, and get a structured development cycle that an AI agent can execute autonomously.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Run `assay plan` (or `/assay:plan` in Claude Code), answer structured questions about what they want to build, and have Assay generate a complete milestone with chunk specs and gate criteria
- Open Claude Code or Codex, ask the agent to work on the next chunk, and watch it execute against verifiable criteria — then check status with `/assay:status`
- Run `assay pr create my-feature` and have Assay verify all gates pass before opening a PR — or get a clear report of which chunks are still failing
- See progress across chunks: which are done, which are in-flight, which are blocked

### Entry point / environment

- Entry point: `assay plan` CLI wizard, `/assay:plan` Claude Code skill, `assay pr create` CLI
- Environment: local dev, any project with a `.assay/` directory
- Live dependencies involved: `gh` CLI (for PR creation), `claude` or `codex` CLI (for agent execution)

## Completion Class

- Contract complete means: all new types round-trip through TOML, all MCP tools have schema tests, all CLI commands have integration tests, wizard generates valid spec files that pass `assay spec list` and `assay gate run`
- Integration complete means: a real `assay plan` → `assay gate run` → `assay pr create` flow works end-to-end in a real git repo with real specs
- Operational complete means: `just ready` passes; no regressions in existing 1271 tests

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- `assay plan` wizard on a fresh project produces milestone TOML + chunk specs that are scannable, listable, and immediately runnable via `assay gate run`
- The Claude Code plugin's `/assay:plan` skill calls `milestone_create` and `spec_create` MCP tools and produces the same artifacts
- `assay pr create` on a milestone where all gates pass creates a real PR via `gh`; on one with failing gates, returns a structured error listing the failing chunks
- `just ready` green (fmt + lint + test + deny)

## Risks and Unknowns

- **Interactive CLI wizard UX** — Rust TUI-in-CLI (dialoguer/inquire) is less battle-tested than Node.js; may have edge cases on Windows/tmux. Mitigate by using `dialoguer` (widely used). Retire in S03.
- **`gh` CLI availability** — PR creation depends on `gh` being installed and authenticated. Graceful error if missing. Retire in S04.
- **Backward compat for spec extension** — Adding `milestone`/`order` fields to GatesSpec with `serde(default)` must not break any of the 1271 existing tests. Retire in S01.

## Existing Codebase / Prior Art

- `crates/assay-types/src/gates_spec.rs` — GatesSpec, GateCriterion: add `milestone` + `order` fields here (backward-compat, Option with serde default)
- `crates/assay-core/src/spec/mod.rs` — spec loading/scanning: scan and load_spec_entry patterns to replicate for milestones
- `crates/assay-core/src/work_session.rs` — atomic file write pattern (tempfile-rename): reuse for milestone_save()
- `crates/assay-core/src/history/mod.rs` — JSON per-record pattern: replicate for TOML milestone records
- `crates/assay-mcp/src/server.rs` — MCP tool registration, `#[tool]` macro pattern, `ToolRouter`: add milestone/cycle tools here
- `crates/assay-cli/src/commands/` — existing command structure: add `milestone.rs`, `plan.rs`, `pr.rs` subcommands
- `plugins/claude-code/skills/` — existing skills structure: add plan.md, status.md, next-chunk.md
- `plugins/codex/` — AGENTS.md placeholder: fill in with workflow guide + 4 skills
- `Cargo.toml` — `dialoguer` crate for interactive prompts (or `inquire` for richer widgets)

> See `.kata/DECISIONS.md` for all architectural and pattern decisions — it is an append-only register; read it during planning, append to it during execution.

## Relevant Requirements

- R039 — Milestone concept: the foundational type this milestone creates
- R040 — Chunk-as-spec: backward-compatible extension to GatesSpec
- R041 — Milestone file I/O: TOML persistence for milestones
- R042 — Guided authoring wizard: the primary user-facing capability of this milestone
- R043 — Development cycle state machine: milestone phase transitions
- R044 — Cycle MCP tools: agent-consumable workflow control surface
- R045 — Gate-gated PR: delivery workflow closure
- R046 — Branch-per-chunk naming: worktree convention extension
- R047 — Claude Code plugin upgrade: Claude Code integration surface
- R048 — Codex plugin: Codex integration surface

## Scope

### In Scope

- `Milestone` + `ChunkRef` + `MilestoneStatus` types in assay-types
- `milestone_load()`, `milestone_save()`, `milestone_scan()` in assay-core
- Backward-compatible `milestone` + `order` fields on `GatesSpec`
- `milestone_list`, `milestone_get`, `cycle_status`, `cycle_advance`, `chunk_status`, `milestone_create`, `spec_create`, `pr_create` MCP tools
- `assay plan` interactive wizard CLI command
- `assay milestone list/status/advance` CLI commands
- `assay pr create` CLI command
- Claude Code plugin: 3 new skills + updated CLAUDE.md + 2 new hooks
- Codex plugin: AGENTS.md + 4 skills

### Out of Scope (M005)

- TUI (M006)
- Agent spawning / harness invocation from workflow layer (the agent runs inside Claude Code/Codex; Assay provides the spec and gate surface)
- OpenCode plugin (M008)
- PR labels, reviewers, templates (M008)
- Analytics (M008)
