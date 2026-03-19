# Kata State

**Active Milestone:** M005 — Spec-Driven Development Core
**Active Slice:** S02 — Development Cycle State Machine
**Active Task:** (none — S02 planning next)
**Phase:** Planning
**Last Updated:** 2026-03-19
**Requirements Status:** 18 active (R042–R059) · 35 validated (R039–R041 newly validated) · 2 deferred · 4 out of scope
**Test Count:** 1293 (all passing — S01 complete)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)

## M005 Roadmap

- [x] S01: Milestone & Chunk Type Foundation `risk:high` — COMPLETE. Milestone/ChunkRef/MilestoneStatus types, atomic I/O, milestone_list/milestone_get MCP tools, assay milestone list CLI. R039, R040, R041 validated. 1293 tests green.
- [ ] S02: Development Cycle State Machine `risk:high` — cycle_status/cycle_advance/chunk_status MCP tools, milestone phase transitions, milestone-level gate aggregation, CLI milestone subcommand (R043, R044)
- [ ] S03: Guided Authoring Wizard `risk:medium` — assay plan CLI wizard, milestone_create/spec_create MCP tools, generates milestone TOML + gates.toml (R042)
- [ ] S04: Gate-Gated PR Workflow `risk:medium` — assay pr create CLI, pr_create MCP tool, branch-per-chunk naming, PR tracking in milestone (R045, R046)
- [ ] S05: Claude Code Plugin Upgrade `risk:low` — 3 new skills, updated CLAUDE.md, Stop+PreCompact hooks (R047)
- [ ] S06: Codex Plugin `risk:low` — AGENTS.md workflow guide, 4 skills (R048)

## Recent Decisions

- D062: Milestone persistence as TOML files in `.assay/milestones/` (not JSON)
- D063: Chunk = spec with `milestone`/`order` metadata fields on GatesSpec (backward-compat)
- D064: `assay-core::milestone` module mirrors `assay-core::spec` structure
- D065: PR creation shells out to `gh` CLI (consistent with D008 git-CLI-first)
- D067: New MCP tools use `milestone_`, `cycle_`, `spec_create`, `pr_create` prefixes
- D069: `Milestone.slug` stored in TOML body (filename is lookup key, TOML is authoritative)

## Blockers

None.

## Next Action

Plan and execute M005/S02: Development Cycle State Machine.

S01 ✅ complete — Milestone/ChunkRef/MilestoneStatus types, milestone I/O, milestone_list/milestone_get MCP tools (24 total), assay milestone list CLI, just ready green, 1293 tests.

S02 depends on S01 (done). S02 adds: MilestoneStatus state machine transition guards in assay-core, cycle_status/cycle_advance/chunk_status MCP tools, `assay milestone status` / `assay milestone advance` CLI subcommands. Branch: `kata/root/M005/S02`.

Note: MCP tool count is now 24. S02 tests that assert tool count must be updated when adding new tools.
