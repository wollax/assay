# Kata State

**Active Milestone:** M005 — Spec-Driven Development Core
**Active Slice:** S02 — Development Cycle State Machine
**Active Task:** T03 — Wire three cycle MCP tools
**Phase:** Executing
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

- D071: CycleStatus lives in assay-core::milestone::cycle (derived view, not persisted)
- D072: cycle_advance CLI error exits code 1 via eprintln, not anyhow propagation
- D073: ChunkStatusResponse is a local struct in server.rs (D051 pattern)
- D069: Milestone.slug stored in TOML body (filename is lookup key, TOML is authoritative)
- D067: New MCP tools use `milestone_`, `cycle_`, `spec_create`, `pr_create` prefixes

## Blockers

None.

## Next Action

Execute T03: Wire `cycle_status`, `cycle_advance`, and `milestone_phase_transition` as three new MCP tools in `crates/assay-mcp/src/server.rs`. Run `cargo test -p assay-mcp -- cycle` — 3 presence tests must pass.
