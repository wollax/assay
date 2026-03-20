# Kata State

**Active Milestone:** M005 — Spec-Driven Development Core
**Active Slice:** S04 — Gate-Gated PR Workflow
**Active Task:** — (S04 not yet started)
**Phase:** Planning
**Last Updated:** 2026-03-20
**Requirements Status:** 14 active (R045–R059 minus R042) · 39 validated (R039–R044 all validated) · 2 deferred · 4 out of scope
**Test Count:** 1320+ (all passing)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)

## M005 Roadmap

- [x] S01: Milestone & Chunk Type Foundation `risk:high` — COMPLETE. Milestone/ChunkRef/MilestoneStatus types, atomic I/O, milestone_list/milestone_get MCP tools, assay milestone list CLI. R039, R040, R041 validated. 1293 tests green.
- [x] S02: Development Cycle State Machine `risk:high` — COMPLETE. cycle_status/cycle_advance/chunk_status MCP tools, milestone phase transitions (Draft→InProgress→Verify→Complete), CLI milestone status/advance subcommands. R043, R044 validated. 1308 tests green.
- [x] S03: Guided Authoring Wizard `risk:medium` — COMPLETE. T01✓ T02✓ T03✓ T04✓ wizard core (create_from_inputs, create_milestone_from_params, create_spec_from_params), assay plan CLI with TTY guard, milestone_create/spec_create MCP tools. R042 validated. 1320+ tests green.
- [ ] S04: Gate-Gated PR Workflow `risk:medium` — assay pr create CLI, pr_create MCP tool, branch-per-chunk naming, PR tracking in milestone (R045, R046)
- [ ] S05: Claude Code Plugin Upgrade `risk:low` — 3 new skills (/assay:plan, /assay:status, /assay:next-chunk), updated CLAUDE.md, Stop+PreCompact hooks (R047)
- [ ] S06: Codex Plugin `risk:low` — AGENTS.md workflow guide, 4 skills (gate-check, spec-show, cycle-status, plan) (R048)

## Recent Decisions

- D076: create_spec_from_params criteria as Vec<String> (descriptions only, no cmd; known limitation)
- D075: WizardChunkInput slug is caller-provided, not auto-derived
- D074: Test-first contract overrides plan API shapes for wizard
- D073: ChunkStatusResponse is a local struct in server.rs (D051 pattern)
- D072: cycle_advance CLI error exits code 1 via eprintln, not anyhow propagation

## Blockers

None.

## Next Action

Begin S04: Gate-Gated PR Workflow. Implement `assay pr create <milestone-slug>` CLI command that checks all chunk gates pass, then calls `gh pr create`. Implement `pr_create` MCP tool. Store PR number/URL in milestone TOML. (R045, R046)
