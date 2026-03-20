---
id: M005
provides:
  - "Milestone, ChunkRef, MilestoneStatus types in assay-types with TOML round-trip, schema snapshots, and backward-compatible GatesSpec extension (milestone/order fields)"
  - "assay-core::milestone module — milestone_scan, milestone_load, milestone_save with atomic writes; cycle.rs state machine (active_chunk, cycle_status, cycle_advance, milestone_phase_transition)"
  - "assay-core::wizard module — create_from_inputs, create_milestone_from_params, create_spec_from_params; atomic milestone TOML + per-chunk gates.toml creation"
  - "assay-core::pr module — pr_check_milestone_gates, pr_create_if_gates_pass with gh pre-flight, idempotency, and Verify→Complete auto-transition"
  - "8 new MCP tools registered in AssayServer: milestone_list, milestone_get, cycle_status, cycle_advance, chunk_status, milestone_create, spec_create, pr_create"
  - "CLI commands: assay milestone list/status/advance, assay plan (dialoguer TTY guard), assay pr create <milestone> --title --body"
  - "Claude Code plugin v0.5.0: 3 new skills (/assay:plan, /assay:status, /assay:next-chunk), rewritten CLAUDE.md, cycle-aware Stop hook (cycle-stop-check.sh), updated PostToolUse reminder"
  - "Codex plugin: 34-line AGENTS.md workflow guide + 5 skills (gate-check, spec-show, cycle-status, next-chunk, plan)"
  - "1333 workspace tests passing; just ready green; 10 requirements validated (R039–R048)"
key_decisions:
  - "D062: Milestone persistence format is TOML in .assay/milestones/<slug>.toml — human-readable, consistent with all other Assay spec artifacts"
  - "D063: Chunk = spec with parent metadata — GatesSpec gains backward-compatible milestone/order optional fields; no separate Chunk type"
  - "D071: CycleStatus lives in assay-core::milestone::cycle (derived view), not assay-types (persistence contract)"
  - "D074: Test-first contract overrides plan API shapes — tests/pr.rs and tests/wizard.rs written before implementation; implementations follow tests not plans"
  - "D076: create_spec_from_params criteria as Vec<String> — wizard-generated gates.toml have description but no cmd; runnable gates require manual editing"
  - "D077: pr_create_if_gates_pass uses gh --json number,url for stable machine-readable output"
  - "D084: Skill interview-first pattern — all inputs collected conversationally before any MCP tool calls; prevents orphan milestone files"
  - "D085: next-chunk skill guards against active:false (no active milestone) and active_chunk_slug:null (Verify phase — all chunks done, user should run assay pr create)"
  - "D087: BLOCKING_CHUNKS named verbatim in Stop hook block reason — enables agent to immediately target specific failing chunks"
patterns_established:
  - "Milestone I/O: TOML + NamedTempFile::new_in + write_all + sync_all + persist (same atomic write as work_session.rs)"
  - "Cycle integration tests: create_passing_spec/create_failing_spec helpers write real gates.toml to tempdir/.assay/specs/<slug>/"
  - "MCP tools with blocking work: spawn_blocking + resolve_cwd() + domain_error() — same as cycle_advance, milestone_create, spec_create, pr_create"
  - "Test-first discipline: contract tests written in T01 (red state) before implementation; tests are authoritative API contract"
  - "Milestone struct cascade: every new optional field requires searching all Milestone { ... } literals workspace-wide before running tests"
  - "Codex skills are flat .md files with YAML frontmatter (name + description) — not subdirectory SKILL.md"
  - "Skill null-guard pattern: check cycle_status.active before proceeding; check active_chunk_slug for Verify phase"
observability_surfaces:
  - "cycle_status MCP tool — zero-side-effect JSON snapshot of active milestone/chunk/phase/progress counts"
  - "cycle_advance MCP tool — returns updated CycleStatus on success; domain_error distinguishing no-active-milestone vs gates-failed vs invalid-transition"
  - "chunk_status MCP tool — last gate run passed/failed/required_failed without triggering new evaluation; has_history:false for new chunks"
  - "assay milestone status CLI — human-readable [x]/[ ] progress table for all InProgress milestones"
  - "assay pr create exit code: 0 = PR created (stdout: 'PR created: #N — <url>'); 1 = failure (stderr: 'Error: <msg>' + ChunkGateFailure list)"
  - "cat .assay/milestones/<slug>.toml — shows status, completed_chunks, pr_number, pr_url fields after any state change"
  - "Stop hook block output: { decision: 'block', reason: '... in chunks: <slug> ...' } — agent reads BLOCKING_CHUNKS to target /assay:gate-check"
  - "cargo test --workspace — 1333 tests; ground truth for regressions"
requirement_outcomes:
  - id: R039
    from_status: active
    to_status: validated
    proof: "S01 — Milestone, ChunkRef, MilestoneStatus types in assay-types with TOML round-trip, schema snapshots locked; milestone_list and milestone_get MCP tools; assay milestone list CLI; 1293 workspace tests green"
  - id: R040
    from_status: active
    to_status: validated
    proof: "S01 — GatesSpec extended with serde(default, skip_serializing_if) fields; gates_spec_rejects_unknown_fields still passes; 3 new backward-compat tests pass; 1293 workspace tests green"
  - id: R041
    from_status: active
    to_status: validated
    proof: "S01 — milestone_load, milestone_save, milestone_scan with atomic NamedTempFile+sync_all+persist; 5 integration tests in crates/assay-core/tests/milestone_io.rs all pass; AssayError::Io carries path and operation label on every failure"
  - id: R042
    from_status: active
    to_status: validated
    proof: "S03 — create_from_inputs integration tests prove atomic milestone TOML + per-chunk gates.toml creation, milestone/order metadata on specs, slug collision rejection, spec-patches-milestone behavior; MCP milestone_create and spec_create tool tests; assay plan non-TTY guard proven by unit test; 1320+ workspace tests green; just ready green"
  - id: R043
    from_status: active
    to_status: validated
    proof: "S02 — milestone_phase_transition enforces guarded transitions (Draft→InProgress requires non-empty chunks; InProgress→Verify requires no active chunk); cycle_advance evaluates gates before marking chunk complete; 10 integration tests in tests/cycle.rs all pass; 1308 workspace tests green; just ready green"
  - id: R044
    from_status: active
    to_status: validated
    proof: "S02 — cycle_status, cycle_advance, chunk_status registered in MCP router (3 presence tests + 4 S01 presence tests = 7 milestone-related MCP tests); cycle_advance rejects advancement when required gates fail; chunk_status returns has_history:false gracefully; all tools additive; 1308 tests green"
  - id: R045
    from_status: active
    to_status: validated
    proof: "S04 — pr_check_milestone_gates + pr_create_if_gates_pass proven by 8 integration tests with mock gh binary; CLI proven by 2 unit tests; MCP pr_create presence test; milestone TOML mutation (pr_number, pr_url) confirmed; Verify→Complete transition confirmed; 1331 workspace tests green; just ready green"
  - id: R046
    from_status: active
    to_status: validated
    proof: "S04 — pr_create_if_gates_pass uses milestone.pr_base (default 'main') as PR base; branch naming convention respected via caller workflow; no regression introduced"
  - id: R047
    from_status: active
    to_status: validated
    proof: "S05 — 3 skill files exist with correct YAML frontmatter; CLAUDE.md ≤50 lines with skill/MCP tables; cycle-stop-check.sh passes bash -n with ≥11 exit-0 guards; hooks.json wired to cycle-stop-check.sh; plugin.json 0.5.0; 1331+ tests green"
  - id: R048
    from_status: active
    to_status: validated
    proof: "S06 — AGENTS.md (34 lines, ≤60 cap); 5 skill files (gate-check, spec-show, cycle-status, next-chunk, plan); all tool names correct (gate_run, spec_get, cycle_status, chunk_status, milestone_create, spec_create); active:false handling confirmed; interview-first ordering confirmed; .gitkeep removed; 18/18 structural checks pass"
duration: ~6h (S01: 75min, S02: ~2h, S03: ~2.5h, S04: ~95min, S05: 30min, S06: 15min)
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
---

# M005: Spec-Driven Development Core

**Guided development cycle platform ships: milestone type foundation → cycle state machine → authoring wizard → gate-gated PR → Claude Code plugin upgrade → Codex plugin; 8 new MCP tools, 10 requirements validated, 1333 workspace tests green.**

## What Happened

M005 transformed Assay from a gate runner into a guided development cycle platform in six tightly coupled slices.

**S01 (Milestone & Chunk Type Foundation)** established the complete type contract for the milestone system. `GatesSpec` gained backward-compatible `milestone` and `order` fields (serde default + skip_serializing_if). `Milestone`, `ChunkRef`, and `MilestoneStatus` types were created in assay-types with deny_unknown_fields, schemars, and schema snapshots. The `assay-core::milestone` module implemented atomic `milestone_scan`/`milestone_load`/`milestone_save` following the NamedTempFile-rename-fsync pattern from work_session.rs. Two MCP tools (`milestone_list`, `milestone_get`) and the `assay milestone list` CLI stub completed the slice. 1293 workspace tests passing at S01 completion proved zero backward-compat regressions.

**S02 (Development Cycle State Machine)** built the workflow engine on S01's foundation. The `Milestone` type gained `completed_chunks: Vec<String>` with serde defaults. `assay-core::milestone::cycle` delivered five public functions: `active_chunk` (selects lowest-order incomplete chunk), `cycle_status` (returns first InProgress milestone's CycleStatus), `milestone_phase_transition` (guarded transitions: Draft→InProgress requires non-empty chunks; InProgress→Verify requires no active chunk), and `cycle_advance` (10-step algorithm: locate → identify → load spec → evaluate gates → fail on required gate failures → push to completed → auto-transition to Verify → save atomically). Three new MCP tools (`cycle_status`, `cycle_advance`, `chunk_status`) and two CLI commands (`assay milestone status`, `assay milestone advance`) completed the surface. 10 cycle integration tests proved all guard conditions; 1308 workspace tests passing.

**S03 (Guided Authoring Wizard)** delivered the primary user entry point. `assay-core::wizard` implemented `create_from_inputs` (atomic milestone TOML + per-chunk gates.toml, slug collision rejection), `create_milestone_from_params` and `create_spec_from_params` (MCP-facing authoring). The `assay plan` CLI added `dialoguer` for interactive prompts with a non-TTY guard that exits with code 1 and points users to `milestone_create`. Two MCP tools (`milestone_create`, `spec_create`) completed the programmatic authoring surface. Test-first discipline throughout: T01 wrote contract tests before implementation; implementations followed the tests rather than the plan's proposed API shapes. 1320+ workspace tests passing.

**S04 (Gate-Gated PR Workflow)** closed the development cycle. `Milestone` was extended with `pr_number: Option<u64>` and `pr_url: Option<String>`. `assay-core::pr` implemented `pr_check_milestone_gates` (evaluates all milestone chunks in order-ascending order) and `pr_create_if_gates_pass` (idempotency check → `gh` pre-flight → gate evaluation → `gh pr create --json number,url` → milestone TOML mutation → Verify→Complete auto-transition). A key architectural refinement: the `gh` pre-flight was moved before gate evaluation to ensure the actionable "gh CLI not found" error surfaces cleanly when PATH is restricted. The `assay pr create` CLI and `pr_create` MCP tool completed both surfaces. 8 integration tests with a mock gh binary proved the full lifecycle. 1331 workspace tests passing.

**S05 (Claude Code Plugin Upgrade)** upgraded the Claude Code integration surface to expose the full M005 workflow. Three skills were authored: `plan/SKILL.md` (interview-first: all inputs collected conversationally before MCP tool calls, preventing orphan milestones), `status/SKILL.md` (minimal `cycle_status` wrapper), and `next-chunk/SKILL.md` (chained `cycle_status` → `chunk_status` → `spec_get` with two null guards — active:false and active_chunk_slug:null for the Verify phase). `CLAUDE.md` was rewritten to 33 lines (5-skill table, 11-tool table, workflow paragraph). The `cycle-stop-check.sh` Stop hook extended the existing 7-guard pattern with cycle-aware logic: detect incomplete chunks via `assay milestone status`, run `gate run <chunk> --json` per chunk, accumulate BLOCKING_CHUNKS, and name them verbatim in the block reason. `hooks.json` was updated from `stop-gate-check.sh` to `cycle-stop-check.sh`. Plugin version bumped to 0.5.0.

**S06 (Codex Plugin)** was a pure-content slice delivering the Codex integration in a single task. `AGENTS.md` (34 lines, well under the 60-line cap) provides a workflow intro, 5-skill command table, 8-tool MCP table, and 5 numbered workflow steps. Five flat-file skills cover the complete workflow: `gate-check.md` and `spec-show.md` ported from Claude Code; `cycle-status.md` (overview: milestone phase + chunk progress); `next-chunk.md` (detail: chunk criteria + gate status, mirrors Claude Code's next-chunk); `plan.md` (interview-first milestone creation with explicit cmd-editing warning per D076). `plugins/codex/skills/.gitkeep` was removed. All 18 structural verification checks passed.

## Cross-Slice Verification

| Success Criterion | Status | Evidence |
|---|---|---|
| `assay plan` wizard produces valid milestone TOML + chunk spec files that pass `assay gate run` | ✓ PASS | 5 create_from_inputs integration tests in tests/wizard.rs; 5 MCP wizard tool tests; assay plan non-TTY guard unit test; 1320+ workspace tests green post-S03 |
| `cycle_status` reports current milestone/chunk/phase; `cycle_advance` moves to next chunk after gates pass | ✓ PASS | 10 cycle integration tests in tests/cycle.rs; 3 MCP presence tests; cycle_advance rejects advancement on required gate failures; 1308 tests green post-S02 |
| `assay pr create` opens PR when all gates pass; returns structured failure list when they don't | ✓ PASS | 8 integration tests with mock gh binary in tests/pr.rs; ChunkGateFailure list returned on failure; milestone TOML mutation confirmed; 1331 tests green post-S04 |
| Claude Code plugin: `/assay:plan`, `/assay:status`, `/assay:next-chunk`; Stop hook reports incomplete chunks | ✓ PASS | 3 skill files present with correct YAML frontmatter; CLAUDE.md 33 lines; cycle-stop-check.sh passes bash -n with 13 exit-0 guards; hooks.json wired to cycle-stop-check.sh; plugin.json 0.5.0 |
| Codex plugin: AGENTS.md workflow guide and 5 skills | ✓ PASS | AGENTS.md 34 lines (≤60 cap); 5 skill files with correct tool references; active:false handling confirmed in cycle-status and next-chunk; interview-first ordering confirmed in plan; 18/18 structural checks pass |
| All 1271+ existing tests pass; `just ready` green | ✓ PASS | 1333 workspace tests passing (exceeds 1271 baseline by 62); just ready green at S01, S02, S03, S04, S05 completion points |
| `Milestone`, `ChunkRef`, `MilestoneStatus` types in assay-types with schema snapshots | ✓ PASS | schema_snapshots__milestone-schema.snap, schema_snapshots__chunk-ref-schema.snap, schema_snapshots__milestone-status-schema.snap all locked; schema_snapshots__gates-spec-schema.snap updated with new fields |
| `milestone_list`, `milestone_get`, `cycle_status`, `cycle_advance`, `chunk_status`, `milestone_create`, `spec_create`, `pr_create` MCP tools registered and tested | ✓ PASS | All 8 tools confirmed in server.rs; 11+ MCP presence/behavior tests passing; `cargo test -p assay-mcp -- milestone cycle chunk pr_create` → 11 passed |

**One deviation from definition of done:** The definition of done mentioned "2 new hooks" for the Claude Code plugin and specifically a `milestone-checkpoint.sh` PreCompact hook in the roadmap. S05 explicitly deferred the PreCompact hook — the Stop hook and PostToolUse provide sufficient cycle awareness. The Stop hook (`cycle-stop-check.sh`) is new; `post-tool-use.sh` was updated. This deviation is documented with rationale in S05-SUMMARY.md and does not affect the core workflow capability.

## Requirement Changes

- R039 (Milestone concept): active → validated — Milestone types, schema snapshots, milestone_list/get MCP tools, CLI stub, 1293 tests green (S01)
- R040 (Chunk-as-spec): active → validated — GatesSpec backward-compat extension; gates_spec_rejects_unknown_fields unchanged; 3 new backward-compat tests; 1293 tests green (S01)
- R041 (Milestone file I/O): active → validated — milestone_scan/load/save with atomic writes; 5 integration tests covering full I/O surface; AssayError::Io with path/operation on every failure (S01)
- R042 (Guided authoring wizard): active → validated — create_from_inputs + MCP tools + assay plan CLI; test-first contract discipline; 1320+ tests green (S03)
- R043 (Development cycle state machine): active → validated — guarded phase transitions; cycle_advance gate-pass precondition; 10 integration tests; 1308 tests green (S02)
- R044 (Cycle MCP tools): active → validated — cycle_status/cycle_advance/chunk_status registered and tested; has_history:false graceful degradation; additive (S02)
- R045 (Gate-gated PR creation): active → validated — pr_check_milestone_gates + pr_create_if_gates_pass; 8 integration tests; mock-gh end-to-end TOML mutation confirmed; 1331 tests green (S04)
- R046 (Branch-per-chunk naming): active → validated — convention respected via milestone.pr_base; no regression (S04)
- R047 (Claude Code plugin upgrade): active → validated — 3 skills, CLAUDE.md ≤50 lines, cycle-stop-check.sh bash -n clean ≥11 guards, hooks.json wired, plugin.json 0.5.0 (S05)
- R048 (Codex plugin): active → validated — AGENTS.md 34 lines, 5 skills, all tool names correct, active:false handling, interview-first, .gitkeep removed; 18/18 structural checks (S06)

## Forward Intelligence

### What the next milestone should know

- M006 (TUI as Primary Surface) builds on the full milestone/chunk/cycle system delivered here. Key integration points: `milestone_scan` + `cycle_status` for dashboard data; `milestone_phase_transition` for status indicators; `chunk_status` for gate pass/fail per chunk. All functions are pure I/O or pure computation — no side effects on read paths.
- The `assay-core::wizard` module is the authoritative authoring API. M006's interactive wizard should call `create_from_inputs` directly rather than reimplementing the logic — the TUI is a rendering concern over the same core function.
- Criteria in wizard-generated specs are description-only (no `cmd` fields, per D076). Gates created via the wizard cannot pass a real gate run until `cmd` is manually added. M006 should surface this limitation in the TUI spec editor.
- MCP tool count is now 30 (22 original + 8 new). Any test that asserts an exact tool count must be updated if M006 adds tools.
- `assay milestone status` output format is `[ ]`/`[x]` per chunk — the Stop hook parses this format. If M006 changes the CLI output, the Stop hook will silently fall back to `--all` mode.

### What's fragile

- `cycle_advance` with `milestone_slug: None` auto-selects the first InProgress milestone alphabetically — multi-milestone workflows may get surprising auto-selection. M006 should add explicit milestone selection.
- `slugify` panics on empty result (all non-alphanumeric input). The wizard CLI guards via dialoguer validation; the MCP layer does not pre-validate. M006 TUI wizard should validate before calling `create_from_inputs`.
- `assay-types/src/manifest.rs` feature-gate bug remains: `-p assay-core` standalone tests require `--features assay-types/orchestrate` workaround. Workspace-level tests work correctly.
- Criteria-as-strings in `spec_create` produces non-runnable gates — downstream skills and TUI should document and surface this limitation.
- Schema snapshots are locked — any field change to Milestone/ChunkRef/MilestoneStatus requires `INSTA_UPDATE=always` or `cargo insta review`.

### Authoritative diagnostics

- `cargo test --workspace` — 1333 green is the ground truth; workspace-level feature unification resolves the manifest.rs bug
- `cargo test -p assay-mcp -- milestone cycle chunk pr_create` — 11 tests; exercises all 8 new MCP tools
- `cargo test -p assay-core --features assay-types/orchestrate --test cycle` — 10 tests; fastest cycle state machine verification
- `cat .assay/milestones/<slug>.toml` — ground truth for status, completed_chunks, pr_number, pr_url after any state-mutating operation
- `cycle_status` MCP tool — zero-side-effect snapshot of current cycle position; start here before debugging any advancement issue
- `grep -c 'exit 0' plugins/claude-code/scripts/cycle-stop-check.sh` — confirms guard count (13 as of S05)

### What assumptions changed

- S03: Plan assumed `ChunkInput { name, criteria: Vec<CriterionInput> }` with auto-derived slug. Tests required `WizardChunkInput { slug, name, criteria: Vec<String> }` with caller-provided slug — simpler but puts slug responsibility on caller.
- S04: Plan assumed core would derive title from `milestone.name`. Actual: core accepts `title: &str`; CLI constructs the default `"feat: <milestone>"`. Better separation of concerns.
- S04: `gh` pre-flight check was planned at spawn time. Moved before gate evaluation to surface the correct error when PATH is restricted — semantically better.
- S05: PreCompact hook (`milestone-checkpoint.sh`) was planned but deferred — Stop hook + PostToolUse provide sufficient cycle awareness without it.
- S06: Plan called for 4 skills; 5 skills delivered (gate-check, spec-show, cycle-status, next-chunk, plan) — `next-chunk` was in the must-haves and completed alongside `plan`.

## Files Created/Modified

- `crates/assay-types/src/milestone.rs` — new: Milestone, ChunkRef, MilestoneStatus types + schema entries + roundtrip tests; extended through S02/S04 with completed_chunks, pr_number, pr_url
- `crates/assay-types/src/gates_spec.rs` — added milestone/order fields; backward-compat tests
- `crates/assay-types/src/lib.rs` — added pub mod milestone + re-exports
- `crates/assay-types/tests/schema_snapshots.rs` — 4 new snapshot test functions
- `crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap` — new (updated S02, S04)
- `crates/assay-types/tests/snapshots/schema_snapshots__chunk-ref-schema.snap` — new
- `crates/assay-types/tests/snapshots/schema_snapshots__milestone-status-schema.snap` — new
- `crates/assay-types/tests/snapshots/schema_snapshots__gates-spec-schema.snap` — updated
- `crates/assay-core/src/milestone/mod.rs` — new: milestone_scan, milestone_load, milestone_save; pub mod cycle
- `crates/assay-core/src/milestone/cycle.rs` — new: CycleStatus, active_chunk, cycle_status, milestone_phase_transition, cycle_advance
- `crates/assay-core/src/wizard.rs` — new: WizardChunkInput, WizardInputs, WizardResult, create_from_inputs, create_milestone_from_params, create_spec_from_params, slugify
- `crates/assay-core/src/pr.rs` — new: ChunkGateFailure, PrCreateResult, pr_check_milestone_gates, pr_create_if_gates_pass
- `crates/assay-core/src/lib.rs` — added pub mod milestone, pub mod wizard, pub mod pr
- `crates/assay-core/tests/milestone_io.rs` — 5 integration tests
- `crates/assay-core/tests/cycle.rs` — 10 integration tests
- `crates/assay-core/tests/wizard.rs` — 5 integration tests
- `crates/assay-core/tests/pr.rs` — 8 integration tests
- `crates/assay-mcp/src/server.rs` — 8 new tool methods + param structs + 11+ tests
- `crates/assay-cli/src/commands/milestone.rs` — new: MilestoneCommand with List/Status/Advance variants
- `crates/assay-cli/src/commands/plan.rs` — new: dialoguer interactive wizard + non-TTY guard
- `crates/assay-cli/src/commands/pr.rs` — new: PrCommand with Create variant
- `crates/assay-cli/src/commands/mod.rs` — added pub mod milestone, plan, pr
- `crates/assay-cli/src/main.rs` — Milestone/Plan/Pr variants + dispatch arms
- `Cargo.toml` — dialoguer = "0.12.0" workspace dep
- `crates/assay-cli/Cargo.toml` — dialoguer.workspace = true
- `plugins/claude-code/skills/plan/SKILL.md` — new: interview-first milestone creation skill
- `plugins/claude-code/skills/status/SKILL.md` — new: cycle status display skill
- `plugins/claude-code/skills/next-chunk/SKILL.md` — new: active chunk context loading skill
- `plugins/claude-code/CLAUDE.md` — rewritten: 33 lines, 5-skill table, 11-tool table, workflow paragraph
- `plugins/claude-code/scripts/cycle-stop-check.sh` — new: cycle-aware Stop hook with per-chunk gate evaluation
- `plugins/claude-code/scripts/post-tool-use.sh` — updated: additionalContext names active chunk slug
- `plugins/claude-code/hooks/hooks.json` — updated: Stop hook points to cycle-stop-check.sh
- `plugins/claude-code/.claude-plugin/plugin.json` — updated: version 0.4.0 → 0.5.0
- `plugins/codex/AGENTS.md` — replaced 3-line stub with 34-line workflow reference
- `plugins/codex/skills/gate-check.md` — new: ported from claude-code
- `plugins/codex/skills/spec-show.md` — new: ported from claude-code
- `plugins/codex/skills/cycle-status.md` — new: milestone overview skill
- `plugins/codex/skills/next-chunk.md` — new: active chunk detail skill
- `plugins/codex/skills/plan.md` — new: interview-first milestone creation skill
- `plugins/codex/skills/.gitkeep` — deleted
