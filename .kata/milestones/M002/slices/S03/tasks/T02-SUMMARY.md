---
id: T02
parent: S03
milestone: M002
provides:
  - order_sessions() function with CompletionTime and FileOverlap strategies
  - CompletedSession operational type for merge ordering input
  - All serializable merge report types (MergeStrategy, MergePlan, MergePlanEntry, MergeSessionStatus, MergeSessionResult, MergeReport, ConflictAction)
key_files:
  - crates/assay-core/src/orchestrate/ordering.rs
  - crates/assay-types/src/orchestrate.rs
  - crates/assay-types/tests/schema_snapshots.rs
key_decisions:
  - CompletedSession is an operational type in assay-core (not serializable), used as input to order_sessions()
  - Three-level deterministic tiebreaking for both strategies — timestamp/overlap, then topo_order, then session_name
  - FileOverlap uses swap_remove for O(1) extraction from remaining set; determinism preserved via tiebreakers
patterns_established:
  - Ordering strategies as pure functions returning (Vec<CompletedSession>, MergePlan) tuple for both result and observability
  - MergePlan entries carry human-readable reason strings for per-session placement rationale
observability_surfaces:
  - MergePlan with per-session ordering rationale (position + reason string)
  - MergeReport with aggregate counts and per-session MergeSessionResult
  - MergeSessionStatus distinguishes merged/skipped/conflict-skipped/aborted/failed
duration: 5m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Add merge ordering strategies and orchestrate merge types

**Implemented CompletionTime and FileOverlap merge ordering strategies as pure functions, with all serializable merge report types and schema snapshots.**

## What Happened

All four steps were already implemented (types, schema snapshots, ordering logic, tests). Verified the implementation is complete and correct:

1. **Serializable types** (`assay-types/src/orchestrate.rs`): `MergeStrategy`, `MergePlan`, `MergePlanEntry`, `MergeSessionStatus`, `MergeSessionResult`, `MergeReport`, `ConflictAction` — all with appropriate derives (`Serialize`, `Deserialize`, `JsonSchema`), `deny_unknown_fields` on persistence types, `rename_all = "snake_case"` on enums, and inventory schema registry submissions.

2. **Schema snapshots** (`assay-types/tests/schema_snapshots.rs`): 7 new feature-gated snapshot tests for merge ordering/runner types, all locked.

3. **Ordering logic** (`assay-core/src/orchestrate/ordering.rs`): `CompletedSession` operational struct, `order_sessions()` dispatcher, `order_by_completion_time()` (sort by timestamp, topo_order, name), `order_by_file_overlap()` (greedy least-overlap-first with merged file set tracking).

4. **Unit tests**: 8 tests covering both strategies, tiebreaking, edge cases (empty, single), and determinism.

## Verification

- `cargo test -p assay-core --features orchestrate -- orchestrate::ordering` — **8 passed** ✅
- `cargo test -p assay-types --features orchestrate` — **50 schema snapshots passed** (including 7 merge types) ✅
- `cargo test -p assay-core --features orchestrate -- merge::tests` — **27 passed** (T01 merge tests still green) ✅
- `cargo clippy -p assay-core -p assay-types --features orchestrate -- -D warnings` — **no warnings** ✅

### Slice-level verification status (intermediate — T02 of 3):
- `cargo test -p assay-core --features orchestrate -- merge::tests` — ✅ 27 passed
- `cargo test -p assay-core --features orchestrate -- orchestrate::ordering` — ✅ 8 passed
- `cargo test -p assay-core --features orchestrate -- orchestrate::merge_runner` — 0 tests (T03 not yet implemented)
- `cargo test -p assay-types --features orchestrate -- merge_runner` — 0 tests (T03 types not yet needed)
- `just ready` — deferred to T03 (final task)

## Diagnostics

- Deserialize `MergeReport` from JSON to see which sessions merged, which were skipped, and why
- `MergePlan.entries` shows the ordering decision for each session (position + reason string)
- `MergeSessionResult.error` carries failure messages; `MergeSessionStatus` distinguishes outcome classes
- For ordering debugging: `MergePlanEntry.reason` includes overlap counts or completion timestamps

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-types/src/orchestrate.rs` — 8 new types with full derives, `deny_unknown_fields`, inventory registration, and serde roundtrip tests
- `crates/assay-types/tests/schema_snapshots.rs` — 7 new feature-gated schema snapshot tests
- `crates/assay-core/src/orchestrate/ordering.rs` — new: `CompletedSession`, `order_sessions()`, CompletionTime/FileOverlap strategies, 8 unit tests
- `crates/assay-core/src/orchestrate/mod.rs` — added `pub mod ordering`
