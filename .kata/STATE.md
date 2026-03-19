# Kata State

**Active Milestone:** M005 — Spec-Driven Development Core
**Active Slice:** — (planning complete; ready to execute S01)
**Active Task:** —
**Phase:** M005 planned — S01 next
**Last Updated:** 2026-03-19
**Requirements Status:** 21 active (R039–R059) · 32 validated · 2 deferred · 4 out of scope
**Test Count:** 1271 (all passing — M004 complete)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)

## M005 Roadmap

- [ ] S01: Milestone & Chunk Type Foundation `risk:high` — Milestone/ChunkRef types in assay-types, file I/O in assay-core, backward-compat GatesSpec extension, milestone_list/milestone_get MCP tools (R039, R040, R041, R044 partial)
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
- D066: Wizard uses `dialoguer` crate for interactive prompts
- D067: New MCP tools use `milestone_`, `cycle_`, `spec_create`, `pr_create` prefixes
- D068: TUI is the preferred primary surface (drives M006+ architecture)

## Blockers

None.

## Next Action

Begin M005/S01: Milestone & Chunk Type Foundation. Branch: `kata/assay/M005/S01`. Read M005-CONTEXT.md and M005-ROADMAP.md S01 boundary map before starting. Add `dialoguer` to Cargo.toml workspace deps for S03 (can be added now to unblock S03 later).

Tasks:
1. Add `Milestone`, `ChunkRef`, `MilestoneStatus` types to `assay-types/src/milestone.rs` with schema snapshots
2. Add `milestone: Option<String>`, `order: Option<u32>` to `GatesSpec` in `assay-types/src/gates_spec.rs` (serde default)
3. Add `assay-core::milestone` module: `milestone_load()`, `milestone_save()`, `milestone_scan()`
4. Register `milestone_list` and `milestone_get` MCP tools in `assay-mcp/src/server.rs`
5. Verify all 1271 existing tests still pass
