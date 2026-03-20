# Kata State

**Active Milestone:** M005 — Spec-Driven Development Core
**Active Slice:** M005 COMPLETE — all 6 slices done
**Active Task:** none
**Phase:** M005 Complete
**Last Updated:** 2026-03-20
**Requirements Status:** 10 active (R049–R059) · 43 validated (R039–R048 all validated) · 2 deferred · 4 out of scope
**Test Count:** 1331 (all passing)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)
- [x] M005: Spec-Driven Development Core (6/6 slices, 10 requirements validated [R039–R048], 1331 tests)

## M005 Roadmap

- [x] S01: Milestone & Chunk Type Foundation `risk:high` — COMPLETE. Milestone/ChunkRef/MilestoneStatus types, atomic I/O, milestone_list/milestone_get MCP tools, assay milestone list CLI. R039, R040, R041 validated. 1293 tests green.
- [x] S02: Development Cycle State Machine `risk:high` — COMPLETE. cycle_status/cycle_advance/chunk_status MCP tools, milestone phase transitions (Draft→InProgress→Verify→Complete), CLI milestone status/advance subcommands. R043, R044 validated. 1308 tests green.
- [x] S03: Guided Authoring Wizard `risk:medium` — COMPLETE. wizard core (create_from_inputs, create_milestone_from_params, create_spec_from_params), assay plan CLI with TTY guard, milestone_create/spec_create MCP tools. R042 validated. 1320+ tests green.
- [x] S04: Gate-Gated PR Workflow `risk:medium` — COMPLETE. pr_check_milestone_gates + pr_create_if_gates_pass, assay pr create CLI, pr_create MCP tool. R045, R046 validated. 1331 tests green.
- [x] S05: Claude Code Plugin Upgrade `risk:low` — COMPLETE. 3 new skills (/assay:plan, /assay:status, /assay:next-chunk), updated CLAUDE.md, Stop+PreCompact hooks. R047 validated.
- [x] S06: Codex Plugin `risk:low` — COMPLETE. AGENTS.md (34 lines), 5 skills (gate-check, spec-show, cycle-status, next-chunk, plan). R048 validated.

## Recent Decisions

- D081: cycle-status and next-chunk are separate Codex skills (overview vs chunk-detail intent separation)
- D080: All 6 Codex plugin files authored in single T01 pass (pure markdown, no split needed)
- D079: S04 test-first — tests/pr.rs written in T01 (red) before assay-core::pr exists in T02
- D078: ChunkGateFailure and PrCreateResult are local types in assay-core::pr (D073 pattern)
- D077: pr_create_if_gates_pass uses `gh --json number,url` for stable structured output

## Blockers

None.

## Next Action

M005 complete. Begin M006 planning: TUI as Primary Surface — real Ratatui TUI with project dashboard (R049), interactive wizard (R050), spec browser (R051), and provider config (R052).
