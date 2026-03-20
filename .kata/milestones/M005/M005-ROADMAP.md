# M005: Spec-Driven Development Core

**Vision:** A beginning developer installs Assay, runs `assay plan` or `/assay:plan` in Claude Code, describes a feature, and gets a structured milestone with verifiable chunk specs. An AI agent works through each chunk. When all gates pass, `assay pr create` opens a PR automatically. Assay transforms from a gate runner into a guided development cycle platform.

## Success Criteria

- `assay plan` wizard on a blank project produces valid milestone TOML + chunk spec files that pass `assay gate run`
- `cycle_status` MCP tool reports the current milestone, active chunk, and phase; `cycle_advance` moves to the next chunk after gates pass
- `assay pr create` opens a real GitHub PR only when all milestone chunk gates pass; returns structured failure list when they don't
- Claude Code plugin surfaces the full workflow: `/assay:plan`, `/assay:status`, `/assay:next-chunk`
- Codex plugin has AGENTS.md workflow guide and 4 working skills
- All 1271+ existing tests pass; `just ready` green

## Key Risks / Unknowns

- **Interactive wizard on non-TTY / tmux** — dialoguer/inquire behavior in edge-case terminals. Mitigate by testing on macOS terminal + CI non-TTY. Retire in S03.
- **`gh` CLI availability for PR creation** — PR command must degrade gracefully when `gh` is not installed or not authenticated. Retire in S04.
- **Backward compatibility for GatesSpec extension** — adding `milestone`/`order` fields must not break any of the 1271 existing tests. Retire in S01.

## Proof Strategy

- GatesSpec backward compat → retire in S01 by proving all 1271 existing tests still pass after adding `Option<String>` + `Option<u32>` fields with serde defaults
- Wizard usability → retire in S03 by running `assay plan` on a real project and verifying generated files pass `assay spec list` + `assay gate run`
- PR creation → retire in S04 by testing with real `gh` CLI in a real git repo: gates pass → PR opens; gates fail → structured error

## Verification Classes

- Contract verification: type round-trip tests, TOML parse/serialise, MCP schema tests, CLI output tests
- Integration verification: `assay plan` → `assay gate run` → `assay pr create` end-to-end in a real git repo with real specs
- Operational verification: `just ready` (fmt + lint + test + deny) green
- UAT / human verification: run `assay plan` by hand, use Claude Code `/assay:plan` skill against a real project

## Milestone Definition of Done

This milestone is complete only when all are true:

- All 6 slices delivered and verified
- `Milestone`, `ChunkRef`, `MilestoneStatus` types exist in assay-types with schema snapshots
- `milestone_list`, `milestone_get`, `cycle_status`, `cycle_advance`, `chunk_status`, `milestone_create`, `spec_create`, `pr_create` MCP tools registered and tested
- `assay plan` wizard generates valid milestone + chunk spec files
- `assay pr create` creates a real PR when gates pass; errors with chunk failure list when they don't
- Claude Code plugin has 3 new skills + updated CLAUDE.md + 2 new hooks
- Codex plugin has AGENTS.md + 4 skills
- `just ready` green, no regressions

## Requirement Coverage

- Covers: R039, R040, R041, R042, R043, R044, R045, R046, R047, R048
- Partially covers: none
- Leaves for later: R049–R059 (TUI, agent harness, OpenCode, PR advanced, analytics)
- Orphan risks: none

## Slices

- [x] **S01: Milestone & Chunk Type Foundation** `risk:high` `depends:[]`
  > After this: `assay milestone list` shows milestones from `.assay/milestones/`; existing specs with added `milestone` field still pass all gate runs; `milestone_list` and `milestone_get` MCP tools return structured data.

- [x] **S02: Development Cycle State Machine** `risk:high` `depends:[S01]`
  > After this: `cycle_status` reports the active milestone/chunk/phase; `cycle_advance` moves to the next chunk when gates pass; `assay milestone status` prints a readable progress summary.

- [x] **S03: Guided Authoring Wizard** `risk:medium` `depends:[S01]`
  > After this: `assay plan` interactively collects a goal + chunk breakdown + criteria per chunk and writes valid milestone TOML + gates.toml files; `milestone_create` and `spec_create` MCP tools do the same programmatically.

- [x] **S04: Gate-Gated PR Workflow** `risk:medium` `depends:[S01,S02]`
  > After this: `assay pr create my-feature` opens a real GitHub PR only when all chunk gates pass; failing chunks are listed with their failed criteria; PR number and URL are stored in the milestone file.

- [ ] **S05: Claude Code Plugin Upgrade** `risk:low` `depends:[S01,S02,S03,S04]`
  > After this: Claude Code users can `/assay:plan` to start the wizard, `/assay:status` to see cycle progress, and `/assay:next-chunk` to get the active chunk context; Stop hook reports incomplete chunks.

- [ ] **S06: Codex Plugin** `risk:low` `depends:[S01,S02]`
  > After this: Codex users have AGENTS.md with the full workflow guide and 4 skills (gate-check, spec-show, cycle-status, plan) that make Assay's development cycle usable inside Codex.

## Boundary Map

### S01 → S02
Produces:
  `assay-types/src/milestone.rs` → `Milestone`, `ChunkRef`, `MilestoneStatus` (types + schema snapshots)
  `assay-core/src/milestone/mod.rs` → `milestone_load()`, `milestone_save()`, `milestone_scan()`
  `assay-types/src/gates_spec.rs` → `milestone: Option<String>`, `order: Option<u32>` fields added
  MCP: `milestone_list`, `milestone_get` tools registered

Consumes: nothing (leaf node)

### S01 → S03
Produces (same as above, consumed by wizard):
  `milestone_save()` — wizard writes generated milestones
  Milestone TOML format — wizard generates to this schema

Consumes: nothing

### S02 → S04
Produces:
  `assay-core/src/milestone/cycle.rs` → `cycle_status()`, `cycle_advance()`, `milestone_phase_transition()`
  MCP: `cycle_status`, `cycle_advance`, `chunk_status` tools
  CLI: `assay milestone status`, `assay milestone advance`

Consumes from S01:
  `Milestone`, `MilestoneStatus`, `milestone_load()`, `milestone_save()`, `milestone_scan()`

### S02 → S05/S06
Produces (consumed by plugin skills):
  `cycle_status` MCP tool — skills call this to show progress
  `cycle_advance` MCP tool — skills call this to advance
  `chunk_status` MCP tool — skills call this to show gate status

### S03 → S05/S06
Produces (consumed by plugin skills):
  `milestone_create` MCP tool — `/assay:plan` skill calls this
  `spec_create` MCP tool — `/assay:plan` skill calls this per chunk

Consumes from S01:
  `milestone_save()`, `Milestone` type, milestone TOML format

### S04 → S05
Produces:
  `assay-core/src/pr.rs` → `pr_create_if_gates_pass()`, `pr_check_milestone_gates()`
  CLI: `assay pr create`
  MCP: `pr_create` tool
  Milestone state: `pr_number: Option<u64>`, `pr_url: Option<String>` fields

Consumes from S01:
  `Milestone`, `milestone_load()`, `milestone_save()`
Consumes from S02:
  `cycle_advance()` (called after PR created to complete milestone)
  Gate pass check via existing `gate::evaluate_all_gates()`

### S05 (Claude Code plugin)
Produces:
  `plugins/claude-code/skills/plan.md` — calls `milestone_create` + `spec_create`
  `plugins/claude-code/skills/status.md` — calls `cycle_status`
  `plugins/claude-code/skills/next-chunk.md` — calls `cycle_status` + `chunk_status`
  `plugins/claude-code/CLAUDE.md` — updated workflow guide
  `plugins/claude-code/scripts/cycle-stop-check.sh` — Stop hook
  `plugins/claude-code/scripts/milestone-checkpoint.sh` — PreCompact hook

Consumes from S01–S04:
  All 8 new MCP tools (milestone_list, milestone_get, cycle_status, cycle_advance, chunk_status, milestone_create, spec_create, pr_create)

### S06 (Codex plugin)
Produces:
  `plugins/codex/AGENTS.md` — full workflow guide
  `plugins/codex/skills/gate-check.md` — ported from claude-code
  `plugins/codex/skills/spec-show.md` — ported from claude-code
  `plugins/codex/skills/cycle-status.md` — new
  `plugins/codex/skills/plan.md` — new

Consumes from S01–S02:
  `milestone_list`, `cycle_status`, `milestone_create`, `spec_create` MCP tools
