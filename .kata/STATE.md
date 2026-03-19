# Kata State

**Active Milestone:** M005 — Spec-Driven Development Core
**Active Slice:** S01 — Milestone & Chunk Type Foundation
**Active Task:** T01 — Define Milestone types in assay-types and extend GatesSpec
**Phase:** Planned — ready to execute
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

## Recent Decisions

- D069: `Milestone.slug` is stored in TOML (like `GatesSpec.name`), not derived from filename — authoritative round-trip
- D070: S01 verification strategy is contract + integration proof (no agent spawning; no end-to-end workflow)

## Next Action

Execute M005/S01/T01: Define Milestone types in assay-types and extend GatesSpec.

Read `.kata/milestones/M005/slices/S01/S01-PLAN.md` for full task list and verification criteria. Branch already exists: `kata/assay/M005/S01`.

T01: `crates/assay-types/src/gates_spec.rs` (add milestone/order fields) + `crates/assay-types/src/milestone.rs` (new — Milestone, ChunkRef, MilestoneStatus) + schema snapshots. Run `cargo insta review` to accept new snapshots. Verify `cargo test -p assay-types` green including `gates_spec_rejects_unknown_fields`.
