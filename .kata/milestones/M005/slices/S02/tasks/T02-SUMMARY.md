---
id: T02
parent: S02
milestone: M005
provides:
  - "`CycleStatus` struct (#[derive(Debug, Clone, Serialize)]) with 6 fields: milestone_slug, milestone_name, phase, active_chunk_slug, completed_count, total_count"
  - "`active_chunk(milestone) -> Option<&ChunkRef>` — sorts chunks by order ascending, excludes completed_chunks slugs"
  - "`cycle_status(assay_dir) -> Result<Option<CycleStatus>>` — returns None when no InProgress milestone; Some(CycleStatus) for first InProgress milestone found"
  - "`milestone_phase_transition(milestone, next) -> Result<()>` — Draft→InProgress (requires chunks), InProgress→Verify (requires no active chunk), Verify→Complete; all others Err"
  - "`cycle_advance(assay_dir, specs_dir, working_dir, milestone_slug: Option<&str>) -> Result<CycleStatus>` — evaluates gates, marks chunk complete, transitions to Verify when last chunk done, saves atomically"
  - "All functions re-exported from `assay_core::milestone` via `pub use cycle::{...}`"
key_files:
  - crates/assay-core/src/milestone/cycle.rs
  - crates/assay-core/src/milestone/mod.rs
  - crates/assay-core/tests/cycle.rs
key_decisions:
  - "cycle_advance signature uses 4 params (no config_timeout) to match test call sites; evaluate_all_gates gets None/None for cli_timeout/config_timeout"
  - "AssayError::Io used for all logical errors (no-active-milestone, invalid-phase, gates-failed) with operation label + path for localized failure diagnosis"
  - "io::Error::other() used instead of io::Error::new(ErrorKind::Other, ...) to satisfy clippy::io_other_error lint"
  - "Test helpers create_passing_spec/create_failing_spec fixed from wrong [gates]/shell format to correct GatesSpec format (name at root, [[criteria]], cmd field)"
patterns_established:
  - "cycle.rs functions are pure sync (D007); no async, no traits — consistent with D001"
  - "cycle_advance: check gates first, mutate state only on pass, save atomically — gate failure leaves milestone unmodified"
  - "milestone_phase_transition is both directly usable and called internally by cycle_advance when last chunk completes"
observability_surfaces:
  - "CycleStatus.completed_count/total_count shows quantitative progress at every call site"
  - "cycle_status(assay_dir) gives current phase/chunk snapshot without mutation"
  - "AssayError::Io { operation, path } on every failure — operation labels: 'cycle_advance', 'milestone phase transition'"
  - "Gate failure Err includes required_failed count and chunk slug; phase transition Err names both from/to states"
  - "cat .assay/milestones/<slug>.toml shows completed_chunks array and status for external inspection"
duration: 35min
verification_result: passed
completed_at: 2026-03-19T00:00:00Z
blocker_discovered: false
---

# T02: Implement `cycle.rs` state machine in assay-core

**`CycleStatus`, `active_chunk`, `cycle_status`, `milestone_phase_transition`, and `cycle_advance` implemented in cycle.rs; all 10 integration tests green; lint clean.**

## What Happened

Created `crates/assay-core/src/milestone/cycle.rs` with the full development cycle state machine. All five public items (`CycleStatus`, `active_chunk`, `cycle_status`, `milestone_phase_transition`, `cycle_advance`) are pure sync functions consistent with D001 (closures, no traits) and D007 (sync core).

`active_chunk` sorts `ChunkRef` by `order` ascending and returns the first chunk whose slug doesn't appear in `completed_chunks`. `cycle_status` scans for the first `InProgress` milestone and builds a `CycleStatus` snapshot. `milestone_phase_transition` enforces a strict state machine: Draft→InProgress requires non-empty chunks; InProgress→Verify requires no active chunk; Verify→Complete is unconditional; all other transitions return a descriptive `AssayError::Io`.

`cycle_advance` follows a 10-step algorithm: locate milestone (by slug or auto-scan), verify InProgress, identify active chunk, validate slug, load directory spec, evaluate gates synchronously, fail-fast on required gate failures (milestone state unchanged), push to `completed_chunks`, auto-transition to Verify if no active chunk remains, save atomically via `milestone_save`, return updated status.

Wired into `milestone/mod.rs` with `pub mod cycle;` and re-exports of all five public items.

Two bugs in the T01 test helpers were also fixed: `create_passing_spec` and `create_failing_spec` wrote TOML with `[gates]` wrapper section and `shell` field, but `GatesSpec` deserializes directly from root level and uses `cmd` not `shell`. Fixed to use the correct format (`name` at root, `[[criteria]]`, `cmd` field). The unused `CycleStatus` import in the test was also removed to satisfy `clippy::unused_imports`.

## Verification

```
cargo test -p assay-core --features assay-types/orchestrate --test cycle
# → test result: ok. 10 passed; 0 failed

cargo test --workspace
# → all suites green, no regressions

just lint
# → Finished dev profile — no errors or warnings
```

## Diagnostics

- `cycle_status(&assay_dir)` — zero-side-effect snapshot of current cycle position
- `cat .assay/milestones/<slug>.toml` — shows `status`, `completed_chunks`, `updated_at`
- All errors surface as `AssayError::Io { operation: "cycle_advance" | "milestone phase transition", path, source }` with descriptive messages
- Gate failure message format: `"chunk '<slug>' gates failed: N required criteria did not pass"`
- Phase transition failure message names both from and to states: `"invalid milestone phase transition: cannot transition from Draft to Verify"`

## Deviations

`cycle_advance` drops the `config_timeout: Option<u64>` parameter specified in the task plan — the integration tests call it with 4 arguments (no timeout arg), so the signature was matched to the tests, passing `None/None` to `evaluate_all_gates` internally. This is a correct deviation since tests are authoritative.

T01 test helpers `create_passing_spec` / `create_failing_spec` contained incorrect TOML format (wrong field names, wrong table structure). These were fixed as part of T02 since making tests pass requires both the implementation and valid test fixtures.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/milestone/cycle.rs` — new: CycleStatus, active_chunk, cycle_status, milestone_phase_transition, cycle_advance
- `crates/assay-core/src/milestone/mod.rs` — added `pub mod cycle;` and re-exports
- `crates/assay-core/tests/cycle.rs` — fixed create_passing_spec/create_failing_spec TOML format; removed unused CycleStatus import
