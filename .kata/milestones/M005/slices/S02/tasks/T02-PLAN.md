---
estimated_steps: 6
estimated_files: 2
---

# T02: Implement `cycle.rs` state machine in assay-core

**Slice:** S02 — Development Cycle State Machine
**Milestone:** M005

## Description

Creates the core business logic for S02: `crates/assay-core/src/milestone/cycle.rs` containing `CycleStatus`, `active_chunk`, `cycle_status`, `milestone_phase_transition`, and `cycle_advance`. All functions are pure sync — consistent with D001 (closures, no traits) and D007 (sync core). The MCP layer wraps `cycle_advance` in `spawn_blocking` in T03. After this task, all 10 cycle integration tests from T01 pass.

## Steps

1. Create `crates/assay-core/src/milestone/cycle.rs`. Add `use` imports for: `std::path::Path`, `chrono::Utc`, `serde::Serialize`, `assay_types::{ChunkRef, Milestone, MilestoneStatus}`, `crate::error::{AssayError, Result}`, `crate::gate::evaluate_all_gates`, `crate::history::validate_path_component`, `crate::milestone::{milestone_load, milestone_save, milestone_scan}`, `crate::spec::{load_spec_entry_with_diagnostics, SpecEntry}`.

2. Define `CycleStatus`:
   ```rust
   #[derive(Debug, Clone, Serialize)]
   pub struct CycleStatus {
       pub milestone_slug: String,
       pub milestone_name: String,
       pub phase: MilestoneStatus,
       pub active_chunk_slug: Option<String>,
       pub completed_count: usize,
       pub total_count: usize,
   }
   ```

3. Implement `pub fn active_chunk(milestone: &Milestone) -> Option<&ChunkRef>`:
   - Collect sorted refs: `let mut ordered: Vec<&ChunkRef> = milestone.chunks.iter().collect(); ordered.sort_by_key(|c| c.order);`
   - Return `ordered.into_iter().find(|c| !milestone.completed_chunks.contains(&c.slug))`

4. Implement `pub fn cycle_status(assay_dir: &Path) -> Result<Option<CycleStatus>>`:
   - Call `milestone_scan(assay_dir)?`
   - Filter to `InProgress` milestones; take first (already sorted alphabetically by `milestone_scan`)
   - If none, return `Ok(None)`
   - Build and return `Ok(Some(CycleStatus { ... }))`

5. Implement `pub fn milestone_phase_transition(milestone: &mut Milestone, next: MilestoneStatus) -> Result<()>`:
   - Match on `(current, next)`:
     - `(Draft, InProgress)`: requires `!milestone.chunks.is_empty()`, else Err
     - `(InProgress, Verify)`: requires `active_chunk(milestone).is_none()`, else Err
     - `(Verify, Complete)`: always Ok
     - All other combinations: return `AssayError::Io` with operation "milestone phase transition" and descriptive message (e.g. `"cannot transition from {current:?} to {next:?}"`)
   - On Ok: set `milestone.status = next` and `milestone.updated_at = Utc::now()`

6. Implement `pub fn cycle_advance(assay_dir: &Path, specs_dir: &Path, working_dir: &Path, milestone_slug: Option<&str>, config_timeout: Option<u64>) -> Result<CycleStatus>`:
   - **Find milestone**: if `milestone_slug` is Some, call `milestone_load(assay_dir, slug)?`; if None, scan and find first `InProgress` — if none, return `Err(AssayError::Io { operation: "cycle_advance", ... })` with message "no active (in_progress) milestone found"
   - **Check status**: if milestone.status != InProgress, return Err "milestone '<slug>' is not in_progress (status: ...)"
   - **Get active chunk**: call `active_chunk(&milestone)` — if None, return Err "milestone '<slug>' has no active chunk (all chunks may be complete — call milestone_phase_transition)"
   - **Load spec**: validate slug via `validate_path_component`; call `load_spec_entry_with_diagnostics(&active_slug, specs_dir)` — on Err, return the error
   - **Evaluate gates**: extract `GatesSpec` from `SpecEntry::Directory { gates, .. }` (for `SpecEntry::Legacy`, return Err "chunk '<slug>' is a legacy spec; directory specs required for milestone chunks"); call `evaluate_all_gates(&gates, working_dir, None, config_timeout)` — this is sync
   - **Check result**: if `summary.enforcement.required_failed > 0`, return `Err(AssayError::Io)` with message `"chunk '<slug>' gates failed: {required_failed} required criteria did not pass"`
   - **Mark complete**: `milestone.completed_chunks.push(active_slug); milestone.updated_at = Utc::now();`
   - **Check if all done**: if `active_chunk(&milestone).is_none()`, call `milestone_phase_transition(&mut milestone, Verify)?`
   - **Save**: call `milestone_save(assay_dir, &milestone)?`
   - **Return**: build and return `Ok(CycleStatus { ... })`

   Wire into `milestone/mod.rs`: add `pub mod cycle;` and `pub use cycle::{active_chunk, cycle_advance, cycle_status, milestone_phase_transition, CycleStatus};`

## Must-Haves

- [ ] `CycleStatus` is `#[derive(Debug, Clone, Serialize)]` and has all 6 fields
- [ ] `active_chunk` sorts by `order` ascending and excludes `completed_chunks` slugs
- [ ] `cycle_status` returns `Ok(None)` for no InProgress milestones; returns `Ok(Some(CycleStatus))` with correct fields for InProgress
- [ ] `milestone_phase_transition` rejects `Draft→Verify`, `InProgress→Complete`, `Verify→InProgress`, etc.
- [ ] `cycle_advance` evaluates gates live (not history); adds to `completed_chunks` only on required-gates-pass; transitions to `Verify` when last chunk complete; saves milestone atomically via `milestone_save`
- [ ] All 10 tests in `crates/assay-core/tests/cycle.rs` pass
- [ ] `cargo test --workspace` green

## Verification

```bash
# Cycle integration tests — all 10 must pass
cargo test -p assay-core --features assay-types/orchestrate --test cycle

# Full workspace — must not regress
cargo test --workspace

# Code quality
just lint
```

## Observability Impact

- Signals added/changed: `cycle_advance` returns `CycleStatus` with `completed_count`/`total_count` showing quantitative progress; `milestone_phase_transition` error message names both from and to state so a future agent knows exactly what transition failed
- How a future agent inspects this: `cat .assay/milestones/<slug>.toml` shows `completed_chunks` array and `status`; `cycle_status(assay_dir)` call at start of any session gives current position; `AssayError::Io` messages include `operation` and `path` for localized failure diagnosis
- Failure state exposed: gate failure Err includes required_failed count and chunk slug; phase transition Err includes current/requested state; spec-not-found Err includes the chunk slug; all errors propagate through `?` to MCP/CLI caller

## Inputs

- `crates/assay-core/tests/cycle.rs` — tests to make pass (created in T01)
- `crates/assay-core/src/milestone/mod.rs` — where to add `pub mod cycle;` and re-exports
- `crates/assay-core/src/gate/mod.rs:171` — `evaluate_all_gates(gates, working_dir, cli_timeout, config_timeout) -> GateRunSummary`
- `crates/assay-core/src/spec/mod.rs` — `load_spec_entry_with_diagnostics(slug, specs_dir)` and `SpecEntry` enum
- `crates/assay-core/src/history/mod.rs` — `validate_path_component` (re-exported path guard)
- `crates/assay-core/src/milestone/mod.rs` — `milestone_load`, `milestone_save`, `milestone_scan`
- `crates/assay-types/src/enforcement.rs` — `EnforcementSummary.required_failed` for gate-pass check

## Expected Output

- `crates/assay-core/src/milestone/cycle.rs` — new: `CycleStatus`, `active_chunk`, `cycle_status`, `milestone_phase_transition`, `cycle_advance`
- `crates/assay-core/src/milestone/mod.rs` — `pub mod cycle;` + re-exports added
