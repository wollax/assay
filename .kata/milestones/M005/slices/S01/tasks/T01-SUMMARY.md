---
id: T01
parent: S01
milestone: M005
provides:
  - "GatesSpec extended with milestone: Option<String> and order: Option<u32> fields (serde default + skip_serializing_if)"
  - "MilestoneStatus enum (Draft/InProgress/Verify/Complete, serde snake_case, default=Draft)"
  - "ChunkRef struct (slug + order, deny_unknown_fields)"
  - "Milestone struct (full field set including chrono timestamps, deny_unknown_fields)"
  - "inventory::submit! schema entries for all three new types"
  - "All three types re-exported from assay-types crate root"
  - "Four new snapshot tests; gates-spec-schema snapshot updated"
key_files:
  - "crates/assay-types/src/milestone.rs"
  - "crates/assay-types/src/gates_spec.rs"
  - "crates/assay-types/src/lib.rs"
  - "crates/assay-types/tests/schema_snapshots.rs"
  - "crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap"
  - "crates/assay-types/tests/snapshots/schema_snapshots__chunk-ref-schema.snap"
  - "crates/assay-types/tests/snapshots/schema_snapshots__milestone-status-schema.snap"
  - "crates/assay-types/tests/snapshots/schema_snapshots__gates-spec-schema.snap"
key_decisions:
  - "Added milestone/order to GatesSpec after depends, before criteria — matches natural TOML authoring order and preserves deny_unknown_fields safety"
  - "Milestone uses deny_unknown_fields for strict TOML contract; slug is a required non-defaulted field that must match the filename"
  - "INSTA_UPDATE=always used to accept snapshots non-interactively (cargo insta review is interactive-only)"
patterns_established:
  - "New assay-types modules follow: types with schemars + inventory::submit! + deny_unknown_fields + cfg(test) roundtrip tests"
  - "GatesSpec struct literal updates required across assay-core tests when adding fields — use workspace-level cargo test to catch all breakages"
observability_surfaces:
  - "cargo test --workspace — full verification surface; all 1283 tests green"
  - "crates/assay-types/tests/snapshots/ — locked schema contracts for all three types and updated gates-spec-schema"
duration: 25min
verification_result: passed
completed_at: 2026-03-19T00:00:00Z
blocker_discovered: false
---

# T01: Define Milestone types in assay-types and extend GatesSpec

**`GatesSpec` extended with milestone/order fields; `Milestone`, `ChunkRef`, `MilestoneStatus` types created in assay-types with full serde/schemars/inventory contract and green workspace tests.**

## What Happened

Extended `GatesSpec` with two optional fields (`milestone: Option<String>`, `order: Option<u32>`) placed after `depends` and before `criteria`, both annotated with `#[serde(default, skip_serializing_if = "Option::is_none")]`. The `deny_unknown_fields` safety property is unchanged — existing TOML files without these fields parse correctly because serde only rejects unknown fields that are *present in the input*, not absent optional fields.

Created `crates/assay-types/src/milestone.rs` with:
- `MilestoneStatus` (enum, Copy, Default=Draft, serde snake_case)
- `ChunkRef` (struct, deny_unknown_fields, slug + order)
- `Milestone` (struct, deny_unknown_fields, full field set with chrono timestamps)
- `inventory::submit!` schema entries for all three types
- Roundtrip tests covering both full and minimal forms, deny_unknown_fields checks, and MilestoneStatus serde/default behavior

Updated `lib.rs` with `pub mod milestone;` and re-exports. Added four snapshot tests to `schema_snapshots.rs`; accepted snapshots via `INSTA_UPDATE=always`.

A secondary fix was required: `GatesSpec` struct literals in `assay-core/src/gate/mod.rs`, `assay-core/src/spec/mod.rs`, and `assay-types/tests/schema_roundtrip.rs` needed `milestone: None, order: None,` added. A targeted Python script applied these changes only to `GatesSpec {` blocks, not to `Spec {` blocks.

## Verification

```
cargo test --workspace  →  1283 tests, 0 failed
cargo test --workspace -- gates_spec_rejects_unknown_fields  →  ok
cargo test --workspace -- gates_spec_milestone_fields  →  3 tests ok
cargo test --workspace -- milestone  →  10 tests ok (types + snapshots)
ls crates/assay-types/tests/snapshots/*.snap.new  →  no pending files
```

Note: `cargo test -p assay-types` fails on the pre-existing `manifest.rs` feature-gating bug (unconditional `use crate::orchestrate::...` without `#[cfg(feature = "orchestrate")]`). This is a pre-existing issue unrelated to T01; workspace-level tests work correctly via feature unification from other crates.

## Diagnostics

- `cargo test --workspace -- milestone` — exercises all new type roundtrip, deny_unknown_fields, and snapshot tests
- `crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap` — locked Milestone JSON schema
- `toml::from_str::<GatesSpec>` returns `Err` with field name and line number on unknown fields — backward-compat diagnostic surface unchanged

## Deviations

- Snapshot acceptance used `INSTA_UPDATE=always` env var instead of `cargo insta review` (which is interactive and cannot run non-interactively)
- Required fixing struct literals in `assay-core` crates — not listed in task plan's Expected Output but necessary due to non-exhaustive struct pattern propagation

## Known Issues

- `cargo test -p assay-types` fails due to pre-existing `manifest.rs` feature-gating bug (not introduced by T01). All tests pass at workspace level.

## Files Created/Modified

- `crates/assay-types/src/milestone.rs` — new file: `MilestoneStatus`, `ChunkRef`, `Milestone` types + schema entries + tests
- `crates/assay-types/src/gates_spec.rs` — added `milestone`/`order` fields to `GatesSpec`; updated existing struct literals; added 2 new tests
- `crates/assay-types/src/lib.rs` — added `pub mod milestone;` and re-exports
- `crates/assay-types/tests/schema_snapshots.rs` — 4 new snapshot test functions
- `crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__chunk-ref-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__milestone-status-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__gates-spec-schema.snap` — updated snapshot (new fields reflected)
- `crates/assay-core/src/gate/mod.rs` — updated GatesSpec struct literals (5 locations)
- `crates/assay-core/src/spec/mod.rs` — updated GatesSpec struct literals (4 locations)
- `crates/assay-types/tests/schema_roundtrip.rs` — updated GatesSpec struct literal (1 location)
