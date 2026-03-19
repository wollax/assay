---
estimated_steps: 4
estimated_files: 3
---

# T01: Add `completed_chunks` to Milestone and write failing cycle integration tests

**Slice:** S02 — Development Cycle State Machine
**Milestone:** M005

## Description

Extends the `Milestone` type in `assay-types` with the `completed_chunks: Vec<String>` field that tracks which chunks in a milestone have been verified and advanced past. This is the central data model change for S02 — the entire cycle state machine is derived from this field at runtime. Also creates the integration test file `crates/assay-core/tests/cycle.rs` with all 10 tests that will drive T02 implementation (they are expected to fail to compile at this stage because the `cycle::` functions don't exist yet).

## Steps

1. Open `crates/assay-types/src/milestone.rs`. Add `completed_chunks: Vec<String>` to the `Milestone` struct with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. Place it after `chunks` and before `depends_on` for logical grouping.
2. Update both struct literals in the `#[cfg(test)]` block in `milestone.rs` (the full `milestone_toml_roundtrip` test and `milestone_minimal_toml_roundtrip` test) to include `completed_chunks: vec![]`. Also verify `skip_serializing_if` works by adding an assertion in `milestone_minimal_toml_roundtrip` that the serialized TOML does not contain `completed_chunks`.
3. Run `INSTA_UPDATE=always cargo test -p assay-types` to regenerate the `schema_snapshots__milestone-schema.snap` snapshot. The new field appears as `"completed_chunks": { "type": "array", "items": { "type": "string" } }` with a default of `[]`.
4. Create `crates/assay-core/tests/cycle.rs` with the following 10 test stubs. Each test imports `assay_core::milestone::cycle::*` and `assay_core::milestone::{milestone_save, milestone_scan}`. Tests should compile (once cycle.rs exists in T02) but the test file itself only needs to be syntactically valid now — write the full test bodies as described in the Verification section of S02-PLAN.md. Write complete test implementations (not just stubs — they will compile error on missing `cycle::` imports, but the test logic should be fully written):
   - `test_cycle_status_no_milestones`: create temp dir, call `cycle_status(&assay_dir)`, assert `Ok(None)`
   - `test_cycle_status_draft_milestone`: save a Draft milestone, call `cycle_status`, assert `Ok(None)`
   - `test_cycle_status_in_progress`: save InProgress milestone with 2 chunks, assert returned `CycleStatus` has correct fields
   - `test_active_chunk_sorted_by_order`: milestone with chunks [{slug:"b", order:2}, {slug:"a", order:1}], assert `active_chunk` returns the order=1 chunk
   - `test_cycle_advance_marks_chunk_complete`: create a spec dir `.assay/specs/chunk-a/gates.toml` with a passing shell command (`[gates]\n[[gates.criteria]]\nname="pass"\nshell="true"`), save milestone with chunk-a, call `cycle_advance`, assert `completed_chunks` contains "chunk-a" and milestone is saved
   - `test_cycle_advance_all_chunks_move_to_verify`: two chunks, advance twice (chunk-a passes, chunk-b passes), assert milestone status becomes `Verify`
   - `test_cycle_advance_gates_fail_returns_error`: create spec with failing shell (`[gates]\n[[gates.criteria]]\nname="fail"\nshell="false"`), call `cycle_advance`, assert `Err`; verify milestone is unchanged (completed_chunks still empty)
   - `test_milestone_phase_transition_valid`: test Draft→InProgress (with chunks), InProgress→Verify (completed_chunks full), Verify→Complete
   - `test_milestone_phase_transition_invalid`: verify→in_progress returns Err; draft→verify returns Err
   - `test_cycle_advance_no_active_milestone`: no InProgress milestone → `cycle_advance` returns Err with descriptive message

## Must-Haves

- [ ] `Milestone` struct has `completed_chunks: Vec<String>` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
- [ ] Both test struct literals in `milestone.rs` include `completed_chunks: vec![]`
- [ ] Schema snapshot `schema_snapshots__milestone-schema.snap` updated to include `completed_chunks` field
- [ ] `crates/assay-core/tests/cycle.rs` exists with all 10 test functions fully written
- [ ] `cargo test --workspace` passes at 1293+ (type extension is backward-compatible; existing tests still green)
- [ ] `cargo test -p assay-types` passes (including updated snapshot tests)

## Verification

```bash
# Schema snapshot accepted
INSTA_UPDATE=always cargo test -p assay-types

# Check snapshot contains new field
grep "completed_chunks" crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap

# Workspace passes
cargo test --workspace

# Confirm test file exists and has 10 test functions
grep "^fn test_" crates/assay-core/tests/cycle.rs | wc -l
# should output 10
```

## Observability Impact

- Signals added/changed: `CycleStatus` will surface `completed_count` and `total_count` (derived from `completed_chunks.len()` and `chunks.len()`); `completed_chunks` in persisted TOML is the durable state signal for cycle progress
- How a future agent inspects this: `cat .assay/milestones/<slug>.toml | grep completed_chunks` shows which chunks are done; `milestone_get` MCP tool returns the full milestone including `completed_chunks`
- Failure state exposed: If schema snapshot drifts, test failure message includes expected vs. actual diff; if serde deserialization fails, `AssayError::Io { operation: "parsing milestone TOML", path }` surfaces the exact file and TOML error

## Inputs

- `crates/assay-types/src/milestone.rs` — existing `Milestone` struct to extend (established in S01)
- `crates/assay-core/tests/milestone_io.rs` — reference pattern for integration test structure using `tempfile::tempdir()`, `milestone_save`, assertions on I/O results
- `.kata/milestones/M005/slices/S02/S02-RESEARCH.md` — `CycleStatus` struct definition and all 10 test descriptions

## Expected Output

- `crates/assay-types/src/milestone.rs` — `Milestone` has `completed_chunks` field; test literals updated
- `crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap` — updated with `completed_chunks` in JSON schema
- `crates/assay-core/tests/cycle.rs` — new file with all 10 tests fully written (will compile once T02 creates `cycle.rs`)
