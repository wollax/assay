# S02: Development Cycle State Machine

**Goal:** Build the cycle state machine on top of S01's foundation: add `completed_chunks` tracking to `Milestone`, implement `cycle.rs` with `cycle_status`/`cycle_advance`/`milestone_phase_transition`, wire three new MCP tools, and add two CLI subcommands.
**Demo:** `cycle_status` reports the active milestone/chunk/phase; `cycle_advance` runs gates for the active chunk, marks it complete, and advances the milestone to `Verify` when all chunks are done; `assay milestone status` prints a progress table; `assay milestone advance` runs the same advancement from the CLI.

## Must-Haves

- `Milestone` has `completed_chunks: Vec<String>` with backward-compatible serde defaults; schema snapshot updated
- `cycle_status(assay_dir)` returns the first `InProgress` milestone's progress or `None` if none exist
- `active_chunk(milestone)` returns the lowest-`order` chunk not in `completed_chunks`
- `milestone_phase_transition` guards transitions: `draft→in_progress` requires chunks; `in_progress→verify` requires all chunks complete; invalid transitions return `Err`
- `cycle_advance` evaluates gates live (via `evaluate_all_gates`), adds chunk to `completed_chunks` on required-gates-pass, saves milestone atomically, transitions to `Verify` when last chunk completes; returns `Err` when required gates fail
- MCP tools `cycle_status`, `cycle_advance`, `chunk_status` are registered and return correct JSON
- CLI `assay milestone status` prints InProgress milestone progress table
- CLI `assay milestone advance` calls cycle_advance and prints the result or a clear error
- Integration tests in `crates/assay-core/tests/cycle.rs` all pass
- `cargo test --workspace` green (1293+ tests); `just ready` green

## Proof Level

- This slice proves: integration (real file I/O + real gate subprocess evaluation in cycle_advance tests)
- Real runtime required: yes — `cycle_advance` tests use real shell commands as gate criteria
- Human/UAT required: no

## Verification

Primary test suite — must all pass:
```
cargo test -p assay-core --features assay-types/orchestrate --test cycle
```

MCP tool registration — 3 presence tests:
```
cargo test -p assay-mcp -- cycle
```

CLI subcommands — 2 tests:
```
cargo test -p assay-cli -- milestone
```

Full workspace + quality gate:
```
cargo test --workspace
just ready
```

**Test file:** `crates/assay-core/tests/cycle.rs` (created in T01, made green in T02)

Tests:
1. `test_cycle_status_no_milestones` — empty `.assay/` → `cycle_status` returns `Ok(None)`
2. `test_cycle_status_draft_milestone` — Draft milestone → returns `Ok(None)` (only InProgress counts)
3. `test_cycle_status_in_progress` — InProgress milestone with 2 chunks, 0 completed → correct `CycleStatus`
4. `test_active_chunk_sorted_by_order` — chunks stored with order=2 before order=1 → lowest order is active
5. `test_cycle_advance_marks_chunk_complete` — 2 chunks, first has passing gate → completed_chunks = ["chunk-a"]; milestone saved
6. `test_cycle_advance_all_chunks_move_to_verify` — advance past last chunk → milestone status = Verify
7. `test_cycle_advance_gates_fail_returns_error` — spec with `false` shell command → `Err` returned, milestone not modified
8. `test_milestone_phase_transition_valid` — draft→in_progress with chunks; in_progress→verify with empty active; verify→complete
9. `test_milestone_phase_transition_invalid` — verify→in_progress returns Err; draft→verify returns Err
10. `test_cycle_advance_no_active_milestone` — no InProgress milestone → `Err`

## Observability / Diagnostics

- Runtime signals: `AssayError::Io { operation, path }` on every milestone read/write failure; structured `CycleStatus` JSON in MCP response with `phase`, `active_chunk_slug`, `completed_count`, `total_count`
- Inspection surfaces: `cycle_status` MCP tool returns current milestone/chunk/phase; `assay milestone status` CLI prints human-readable table; gate failure in `cycle_advance` returns the `GateRunSummary` error message (required_failed count, which gates failed)
- Failure visibility: `cycle_advance` Err distinguishes: "no active milestone", "gates failed (N required criteria)", "spec not found for chunk <slug>", "invalid phase transition"; all surfaces `AssayError::Io` with operation label + path
- Redaction constraints: none — no secrets in cycle state

## Integration Closure

- Upstream surfaces consumed: `Milestone`, `ChunkRef`, `MilestoneStatus` from `assay-types::milestone`; `milestone_load`, `milestone_save`, `milestone_scan` from `assay-core::milestone`; `evaluate_all_gates` from `assay-core::gate`; `load_spec_entry_with_diagnostics` + `SpecEntry` from `assay-core::spec`; `history::list` + `history::load` from `assay-core::history`; `validate_path_component` from `assay-core::history`; `resolve_cwd`, `load_config`, `resolve_working_dir`, `domain_error` helpers in `assay-mcp::server`
- New wiring introduced: `cycle.rs` module in `assay-core::milestone` → called by MCP tools via `spawn_blocking` and by CLI directly; `CycleStatus` serialized as JSON in MCP responses; `AssayServer` gains 3 tool methods; `MilestoneCommand` gains `Status` and `Advance` variants
- What remains before milestone is truly usable end-to-end: S03 (wizard creates milestones), S04 (PR gating), S05/S06 (plugins)

## Tasks

- [x] **T01: Add `completed_chunks` to Milestone and write failing cycle integration tests** `est:45m`
  - Why: Extends the Milestone type with the field that tracks which chunks are done (the central S02 data model change), and establishes the integration test suite that drives T02 implementation
  - Files: `crates/assay-types/src/milestone.rs`, `crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap`, `crates/assay-core/tests/cycle.rs`
  - Do: (1) Add `completed_chunks: Vec<String>` to `Milestone` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`; update both struct literals in the `#[cfg(test)]` block to include `completed_chunks: vec![]`; (2) Run `INSTA_UPDATE=always cargo test -p assay-types` to regenerate the milestone schema snapshot; (3) Create `crates/assay-core/tests/cycle.rs` with all 10 tests listed in the Verification section — these tests call `cycle::cycle_status`, `cycle::active_chunk`, `cycle::cycle_advance`, `cycle::milestone_phase_transition` which don't exist yet, so they fail to compile; (4) Confirm `cargo test --workspace` (with the feature flag) still passes at 1293+
  - Verify: `INSTA_UPDATE=always cargo test -p assay-types` exits 0; `cargo test --workspace` passes; `grep completed_chunks crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap` shows the field in the snapshot
  - Done when: `Milestone` has the new field with correct serde attributes; schema snapshot updated; `cycle.rs` test file exists with all test stubs (compilation failure is expected at this stage — the test file references `cycle::` functions that don't yet exist)

- [x] **T02: Implement `cycle.rs` state machine in assay-core** `est:60m`
  - Why: The core business logic of S02 — makes the T01 integration tests pass by implementing all cycle functions
  - Files: `crates/assay-core/src/milestone/cycle.rs`, `crates/assay-core/src/milestone/mod.rs`
  - Do: (1) Create `crates/assay-core/src/milestone/cycle.rs`; define `CycleStatus { milestone_slug, milestone_name, phase, active_chunk_slug, completed_count, total_count }` with `#[derive(Debug, Clone, Serialize)]`; (2) Implement `pub fn active_chunk(milestone: &Milestone) -> Option<&ChunkRef>` — sort chunks by `order` ascending, return first not in `completed_chunks`; (3) Implement `pub fn cycle_status(assay_dir: &Path) -> Result<Option<CycleStatus>>` — `milestone_scan`, find first `InProgress`, build `CycleStatus` using `active_chunk`; multiple InProgress → return first alphabetically; (4) Implement `pub fn milestone_phase_transition(milestone: &mut Milestone, next: MilestoneStatus) -> Result<()>` — valid transitions: `Draft→InProgress` (requires chunks.len() >= 1), `InProgress→Verify` (requires `active_chunk(m).is_none()`), `Verify→Complete` (always valid); all others return `AssayError::Io` with descriptive message (no new error variant, consistent with S01 patterns); (5) Implement `pub fn cycle_advance(assay_dir: &Path, specs_dir: &Path, working_dir: &Path, milestone_slug: Option<&str>, config_timeout: Option<u64>) -> Result<CycleStatus>` — load target milestone (first InProgress if slug is None), get active chunk via `active_chunk`, validate chunk is Some (else Err), load the chunk spec via `load_spec_entry_with_diagnostics(slug, specs_dir)`, evaluate via `evaluate_all_gates` (Directory entry), check `summary.enforcement.required_failed == 0`, add chunk slug to `completed_chunks`, check if `active_chunk` is now None → call `milestone_phase_transition(m, Verify)`, update `updated_at`, `milestone_save`; return updated `CycleStatus`; (6) Add `pub mod cycle;` to `milestone/mod.rs` and re-export `CycleStatus`, `cycle_status`, `cycle_advance`, `milestone_phase_transition`, `active_chunk`
  - Verify: `cargo test -p assay-core --features assay-types/orchestrate --test cycle` passes all 10 tests; `cargo test --workspace` green
  - Done when: All 10 cycle integration tests pass; `just ready` green

- [x] **T03: Add `cycle_status`, `cycle_advance`, `chunk_status` MCP tools** `est:45m`
  - Why: Exposes the cycle state machine to MCP consumers (agent tools); completes R044
  - Files: `crates/assay-mcp/src/server.rs`
  - Do: (1) Add `CycleStatusParams {}`, `CycleAdvanceParams { milestone_slug: Option<String> }`, `ChunkStatusParams { chunk_slug: String }` near the other milestone param structs; (2) Define a local `ChunkStatusResponse { chunk_slug, has_history, latest_run_id, passed, failed, required_failed }` struct with `#[derive(Serialize)]` in server.rs (consistent with D051 pattern); (3) Add `cycle_status` tool method: resolve cwd + assay_dir, call `assay_core::milestone::cycle_status(&assay_dir)`, serialize result as JSON (null for None, CycleStatus JSON for Some); (4) Add `cycle_advance` tool method: resolve cwd + config, call `tokio::task::spawn_blocking(move || assay_core::milestone::cycle_advance(&assay_dir, &specs_dir, &working_dir, slug.as_deref(), config_timeout))`, serialize `CycleStatus` on Ok or return `domain_error` on Err; (5) Add `chunk_status` tool method: resolve cwd + assay_dir, validate slug via `validate_path_component`, call `assay_core::history::list(&assay_dir, &params.chunk_slug)`, get last run_id, if Some call `history::load`, build `ChunkStatusResponse`; (6) Add 3 presence tests mirroring `milestone_list_tool_in_router`
  - Verify: `cargo test -p assay-mcp -- cycle` passes 3 tests; `cargo test --workspace` green
  - Done when: `cycle_status`, `cycle_advance`, `chunk_status` appear in `tool_router.list_all()`; all 3 presence tests pass; existing 4 milestone tool tests still pass

- [ ] **T04: Add `assay milestone status` and `assay milestone advance` CLI subcommands** `est:30m`
  - Why: Makes the cycle state machine accessible from the CLI; completes the CLI surface for R043/R044
  - Files: `crates/assay-cli/src/commands/milestone.rs`
  - Do: (1) Add `Status` and `Advance { milestone_slug: Option<String> }` variants to `MilestoneCommand`; (2) Add arms in `handle()`; (3) Implement `milestone_status_cmd()` — call `milestone_scan`, filter to `InProgress`, for each milestone: print header `MILESTONE: <slug> (<status>)`, then for each chunk print `  [x] <slug>  (complete)` or `  [ ] <slug>  (active)` based on `completed_chunks`; if no InProgress milestones print "No active milestones."; (4) Implement `milestone_advance_cmd(milestone_slug: Option<String>)` — resolve `project_root()`, compute `specs_dir` as `project_root.join(".assay/specs")`, `working_dir` = `project_root`, call `assay_core::milestone::cycle_advance(&assay_dir, &specs_dir, &working_dir, milestone_slug.as_deref(), None)`, on Ok print summary, on Err print descriptive error and return exit code 1; (5) Add test `milestone_status_no_milestones` (empty .assay dir, expect "No active milestones."); add test `milestone_advance_no_active_milestone` (no milestones → advance exits with error)
  - Verify: `cargo test -p assay-cli -- milestone` passes 3+ tests; `cargo test --workspace` green; `just ready` green
  - Done when: `assay milestone status` and `assay milestone advance` are registered CLI subcommands; both tests pass; `just ready` green

## Files Likely Touched

- `crates/assay-types/src/milestone.rs`
- `crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap`
- `crates/assay-core/src/milestone/cycle.rs` (new)
- `crates/assay-core/src/milestone/mod.rs`
- `crates/assay-core/tests/cycle.rs` (new)
- `crates/assay-mcp/src/server.rs`
- `crates/assay-cli/src/commands/milestone.rs`
