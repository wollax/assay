---
id: S02
parent: M005
milestone: M005
provides:
  - "`completed_chunks: Vec<String>` on `Milestone` with backward-compatible serde defaults; schema snapshot updated"
  - "`CycleStatus` derived view type (milestone_slug, milestone_name, phase, active_chunk_slug, completed_count, total_count) in assay-core::milestone::cycle"
  - "`active_chunk(milestone)` — returns lowest-order chunk not in completed_chunks"
  - "`cycle_status(assay_dir)` — returns first InProgress milestone's CycleStatus or None"
  - "`milestone_phase_transition(milestone, next)` — guarded state machine: Draft→InProgress, InProgress→Verify, Verify→Complete; all others Err"
  - "`cycle_advance(assay_dir, specs_dir, working_dir, milestone_slug)` — evaluates gates live, marks chunk complete, transitions to Verify on last chunk, saves atomically"
  - "MCP tools `cycle_status`, `cycle_advance`, `chunk_status` registered and tested (3 presence tests)"
  - "CLI `assay milestone status` — prints InProgress milestone progress tables"
  - "CLI `assay milestone advance` — calls cycle_advance, exits 0 on success / 1 on gate/precondition failure"
  - "10 cycle integration tests in crates/assay-core/tests/cycle.rs — all passing"
  - "3 MCP presence tests in assay-mcp; 3 CLI tests in assay-cli (total 3 milestone tests)"
requires:
  - slice: S01
    provides: "Milestone, ChunkRef, MilestoneStatus types; milestone_load, milestone_save, milestone_scan I/O functions; milestone_list and milestone_get MCP tools; assay milestone list CLI subcommand"
affects:
  - S04
  - S05
  - S06
key_files:
  - crates/assay-types/src/milestone.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap
  - crates/assay-core/src/milestone/cycle.rs
  - crates/assay-core/src/milestone/mod.rs
  - crates/assay-core/tests/cycle.rs
  - crates/assay-mcp/src/server.rs
  - crates/assay-cli/src/commands/milestone.rs
key_decisions:
  - "D071: CycleStatus lives in assay-core::milestone::cycle, not assay-types — derived view type, not persisted contract"
  - "D072: milestone_advance_cmd returns Ok(1) with eprintln on Err — CLI domain-failure contract (not panic/propagation)"
  - "D073: ChunkStatusResponse is a local server.rs struct, not in assay-types — serialization-only response shape"
  - "cycle_advance: 4 params (no config_timeout) — tests are authoritative; evaluate_all_gates gets None/None internally"
  - "AssayError::Io used for all cycle logical errors (no new error variant) — consistent with S01 patterns"
  - "cycle_advance: check gates first, mutate state only on pass, save atomically — gate failure leaves milestone unmodified"
patterns_established:
  - "Cycle integration tests use create_passing_spec/create_failing_spec helpers that write real gates.toml to tempdir/.assay/specs/<slug>/"
  - "cycle_advance spawn_blocking pattern in MCP: load config/dirs in async context, move owned values into closure"
  - "chunk_status early-return with { has_history: false } when no runs exist — graceful degradation"
  - "milestone_phase_transition used both as direct API and internally by cycle_advance's last-chunk logic"
observability_surfaces:
  - "cycle_status MCP tool — zero-side-effect JSON snapshot of active milestone/chunk/phase/progress counts"
  - "cycle_advance MCP tool — returns updated CycleStatus on success; domain_error distinguishing no-active-milestone vs gates-failed vs invalid-transition"
  - "chunk_status MCP tool — last gate run passed/failed/required_failed without triggering new evaluation"
  - "assay milestone status CLI — human-readable [x]/[ ] progress table for all InProgress milestones"
  - "assay milestone advance CLI stderr — AssayError::Io with operation label and path on failure"
  - "cat .assay/milestones/<slug>.toml — shows completed_chunks array and status field for external inspection"
drill_down_paths:
  - .kata/milestones/M005/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M005/slices/S02/tasks/T02-SUMMARY.md
  - .kata/milestones/M005/slices/S02/tasks/T03-SUMMARY.md
  - .kata/milestones/M005/slices/S02/tasks/T04-SUMMARY.md
duration: ~2h (T01: 20m, T02: 35m, T03: 20m, T04: 10m + overhead)
verification_result: passed
completed_at: 2026-03-19T00:00:00Z
---

# S02: Development Cycle State Machine

**Cycle state machine ships: `cycle_status`/`cycle_advance`/`chunk_status` MCP tools + `assay milestone status`/`advance` CLI commands; all 10 integration tests and 1308 workspace tests green.**

## What Happened

S02 built the development cycle state machine on top of S01's Milestone foundation in four tasks.

**T01** extended `Milestone` with `completed_chunks: Vec<String>` using backward-compatible serde defaults (`#[serde(default, skip_serializing_if = "Vec::is_empty")]`), regenerated the schema snapshot, and created `crates/assay-core/tests/cycle.rs` with all 10 fully-written integration tests. The test file compiled-failed as expected — the `cycle::` functions it references didn't exist yet.

**T02** implemented the full `cycle.rs` state machine. `active_chunk` sorts `ChunkRef` by `order` ascending and returns the first not in `completed_chunks`. `cycle_status` scans for the first `InProgress` milestone. `milestone_phase_transition` enforces strict guarded transitions (Draft→InProgress requires non-empty chunks; InProgress→Verify requires no active chunk). `cycle_advance` follows a fail-safe 10-step algorithm: locate milestone, identify active chunk, load spec, evaluate gates synchronously via `evaluate_all_gates`, fail immediately on required gate failures leaving the milestone unmodified, push to `completed_chunks`, auto-transition to Verify if no active chunk remains, then save atomically via `milestone_save`. Also fixed T01's test helpers which had incorrect TOML format (wrong table structure and field name `shell` vs `cmd`). All 10 integration tests passed.

**T03** wired the cycle state machine to the MCP transport layer. Added `CycleStatusParams`, `CycleAdvanceParams`, `ChunkStatusParams` param structs and a `ChunkStatusResponse` local response struct. Three `#[tool]`-annotated async methods: `cycle_status` (synchronous read, returns JSON or `"null"`), `cycle_advance` (spawn_blocking pattern, maps failures to `domain_error`), and `chunk_status` (reads history without running gates, returns early with `{ has_history: false }` when no runs exist). Added 3 presence tests.

**T04** completed the CLI surface with `Status` and `Advance { milestone: Option<String> }` variants on `MilestoneCommand`. `milestone_status_cmd` scans milestones, filters to InProgress, sorts chunks by order, and prints `[x]`/`[ ]` markers. `milestone_advance_cmd` wraps `cycle_advance` with the CLI domain-error contract (eprintln + `return Ok(1)`) rather than propagating errors.

## Verification

```bash
# 10 cycle integration tests
cargo test -p assay-core --features assay-types/orchestrate --test cycle
# → test result: ok. 10 passed; 0 failed

# 3 MCP presence tests (cycle_status, cycle_advance, chunk_status)
cargo test -p assay-mcp -- cycle
# → 3 passed; also chunk_status_tool_in_router passes in full suite

# 3 CLI milestone tests
cargo test -p assay-cli -- milestone
# → 3 passed (list, status, advance)

# Full workspace (1308 tests)
cargo test --workspace
# → all suites green, 0 failures

# Quality gate
just ready
# → "All checks passed."
```

## Requirements Advanced

- R043 (Development cycle state machine) — now validated: draft→in_progress→verify transitions guarded and proven by 8 phase-transition tests; `cycle_advance` enforces gate-pass precondition before completing a chunk
- R044 (Cycle MCP tools) — now validated: `cycle_status`, `cycle_advance`, `chunk_status` registered and tested; `cycle_advance` rejects advancement when required gates fail; structured CycleStatus JSON returned

## Requirements Validated

- R043 — state machine implemented with guarded transitions, tested by `test_milestone_phase_transition_valid`, `test_milestone_phase_transition_invalid`, and full cycle integration suite; all must-haves met
- R044 — all three cycle MCP tools present in router and returning correct JSON; `chunk_status` reads history without evaluation cost; `cycle_advance` returns `GateRunSummary`-derived error on required gate failure

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- `cycle_advance` core function takes 4 parameters (no `config_timeout`) — task plan showed 5; corrected at T02 because tests are authoritative. `evaluate_all_gates` receives `None/None` internally.
- `validate_path_component` removed from `chunk_status` implementation — it is `pub(crate)` in assay-core and inaccessible from assay-mcp. `history::list` rejects invalid slugs naturally (same as `gate_history`).
- T01 test helpers `create_passing_spec`/`create_failing_spec` had incorrect TOML format (`[gates]` wrapper section, `shell` field) — fixed in T02 since tests are the contract.
- `cargo fmt` required before `just ready` passed due to pre-existing formatting issue in `assay-mcp/src/server.rs` from T03.

## Known Limitations

- `cycle_advance` resolves the "first InProgress milestone alphabetically" when no slug is provided — no user control over which milestone to advance when multiple are in_progress simultaneously. Adequate for single-milestone workflows (typical use case). Multi-milestone disambiguation can be added in S04/S05 if needed.
- `chunk_status` requires a prior `gate_run` to return meaningful data — there is no "run gates on demand" shortcut from chunk_status; users must run `assay gate run <chunk-slug>` first.
- Milestone phase transitions do not validate that chunks' spec files actually exist on disk — `Draft→InProgress` allows transitioning to active work even if chunks reference missing specs. `cycle_advance` discovers the missing spec at advancement time via `load_spec_entry_with_diagnostics` returning an error.

## Follow-ups

- S04 will consume `cycle_advance` (called after PR is created to complete the milestone) and the gate pass check via `evaluate_all_gates`.
- S05/S06 plugins will call `cycle_status`, `cycle_advance`, and `chunk_status` MCP tools for their skill implementations.
- If multi-milestone workflows become common, add explicit `milestone_slug` disambiguation to `cycle_status` (it always returns the first InProgress alphabetically today).

## Files Created/Modified

- `crates/assay-types/src/milestone.rs` — added `completed_chunks: Vec<String>` field with serde attributes; updated two test struct literals
- `crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap` — regenerated with `completed_chunks` in JSON schema
- `crates/assay-core/src/milestone/cycle.rs` — new: CycleStatus, active_chunk, cycle_status, milestone_phase_transition, cycle_advance
- `crates/assay-core/src/milestone/mod.rs` — added `pub mod cycle;` and re-exports for all 5 public items
- `crates/assay-core/tests/cycle.rs` — new: 10 integration tests; test helpers fixed in T02
- `crates/assay-core/tests/milestone_io.rs` — `make_milestone` helper updated with `completed_chunks: vec![]`
- `crates/assay-mcp/src/server.rs` — added CycleStatusParams, CycleAdvanceParams, ChunkStatusParams, ChunkStatusResponse; 3 tool methods; 3 presence tests
- `crates/assay-cli/src/commands/milestone.rs` — added Status and Advance variants; milestone_status_cmd and milestone_advance_cmd; 2 new tests (3 total)

## Forward Intelligence

### What the next slice should know
- `cycle_advance` is the single authoritative function for advancing milestone state — S04 should call this (not invent a parallel advancement mechanism) after PR creation
- `ChunkRef.order` is the ordering field for active chunk selection — chunks without order (None) sort after all ordered chunks in `active_chunk`
- `milestone_scan` returns all milestones; `cycle_status` filters to the first InProgress — S04/S05 should call `cycle_status` to get the current position, not scan manually
- The `--features assay-types/orchestrate` flag is required for workspace tests due to a pre-existing `manifest.rs` feature gate; see S02-RESEARCH.md

### What's fragile
- `cycle_advance` with `milestone_slug: None` auto-selects the first InProgress milestone alphabetically — if tests or S04 code creates multiple InProgress milestones, auto-selection may return a surprising target
- `chunk_status` defers slug validation to `history::list` rather than explicit `validate_path_component` — this means invalid slugs return an I/O error rather than a validation error; the error message is still actionable but different in shape

### Authoritative diagnostics
- `cargo test -p assay-core --features assay-types/orchestrate --test cycle` — fastest way to confirm cycle state machine integrity (10 tests, ~0.2s)
- `cat .assay/milestones/<slug>.toml` — ground truth for milestone status and completed_chunks after any cycle_advance call
- `cycle_status` MCP tool — zero-side-effect snapshot of current cycle position; start here before debugging any advancement issue

### What assumptions changed
- Task plan showed `cycle_advance` with 5 parameters including `config_timeout: Option<u64>` — the actual function takes 4 (no timeout); tests were authoritative and the plan was wrong
- T01 test helpers were assumed correct but contained wrong TOML format — `[gates]` section wrapper and `shell` field don't exist in GatesSpec; GatesSpec deserializes from root with `cmd` field
