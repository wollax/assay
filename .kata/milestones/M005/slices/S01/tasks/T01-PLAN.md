---
estimated_steps: 7
estimated_files: 4
---

# T01: Define Milestone types in assay-types and extend GatesSpec

**Slice:** S01 — Milestone & Chunk Type Foundation
**Milestone:** M005

## Description

Extend `GatesSpec` with backward-compatible `milestone` and `order` fields (D063), then create the `Milestone`, `ChunkRef`, and `MilestoneStatus` types in `assay-types`. Lock schema snapshots via `cargo insta review`. This task establishes the type contract that every downstream task, crate, and test depends on.

The critical backward-compat guarantee: adding `milestone` and `order` to `GatesSpec` with `#[serde(default, skip_serializing_if = "Option::is_none")]` is safe because `deny_unknown_fields` only rejects fields present in the TOML input that aren't on the Rust struct. Existing TOML files that omit these fields parse fine. The `gates_spec_rejects_unknown_fields` test remains valid and must pass unchanged.

## Steps

1. In `crates/assay-types/src/gates_spec.rs`, add `milestone: Option<String>` and `order: Option<u32>` to the `GatesSpec` struct after `depends` and before `criteria`, each annotated with `#[serde(default, skip_serializing_if = "Option::is_none")]`. Add two new tests: `gates_spec_milestone_fields_roundtrip` (round-trip TOML with both fields set) and `gates_spec_milestone_fields_absent_from_legacy_toml` (existing TOML without the fields still deserializes correctly and fields are `None`).

2. Create `crates/assay-types/src/milestone.rs`. Define:
   - `MilestoneStatus` enum: variants `Draft`, `InProgress`, `Verify`, `Complete`; `#[serde(rename_all = "snake_case")]`; derive `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default` (default = `Draft`).
   - `ChunkRef` struct: `slug: String`, `order: u32`; `deny_unknown_fields`; derive `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema`.
   - `Milestone` struct: `slug: String` (stored in TOML — the authoritative identifier; mirrors `GatesSpec.name` convention), `name: String`, `description: Option<String>` (serde default + skip_serializing_if), `status: MilestoneStatus` (serde default), `chunks: Vec<ChunkRef>` (serde default), `depends_on: Vec<String>` (serde default + skip_serializing_if empty), `pr_branch: Option<String>` (serde default + skip_serializing_if), `pr_base: Option<String>` (serde default + skip_serializing_if), `created_at: DateTime<Utc>`, `updated_at: DateTime<Utc>`; `deny_unknown_fields`; derive `Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema`. Note: `slug` is a required TOML field (no `serde(default)`) — the TOML file contains `slug = "my-feature"` and the filename `my-feature.toml` must match.
   - Add `inventory::submit!` `SchemaEntry` blocks for all three types: names `"milestone"`, `"chunk-ref"`, `"milestone-status"`.
   - Add a `#[cfg(test)]` block with `milestone_toml_roundtrip` test covering all fields including `None` optionals.

3. In `crates/assay-types/src/lib.rs`:
   - Add `pub mod milestone;` to the module declarations.
   - Add `pub use milestone::{ChunkRef, Milestone, MilestoneStatus};` to the pub-use block.

4. Verify `chrono` is accessible in `assay-types` (it is — `GateRunRecord` uses it). Add `use chrono::{DateTime, Utc};` in `milestone.rs`.

5. In `crates/assay-types/tests/schema_snapshots.rs`, add four snapshot tests:
   - `milestone_schema_snapshot` → `schemars::schema_for!(assay_types::Milestone)` → `assert_json_snapshot!("milestone-schema", ...)`
   - `chunk_ref_schema_snapshot` → `"chunk-ref-schema"`
   - `milestone_status_schema_snapshot` → `"milestone-status-schema"`
   - `gates_spec_schema_updated_snapshot` → re-run the existing `gates_spec_schema_snapshot` logic — this will detect the new fields and require snapshot update.

6. Run `cargo test -p assay-types` — tests with new snapshots will fail with "snapshot not found" messages. Run `cargo insta review` to accept all new and updated snapshots. Re-run `cargo test -p assay-types` to confirm all pass.

7. Confirm `gates_spec_rejects_unknown_fields` still passes (the existing test must be unchanged and must still pass).

## Must-Haves

- [ ] `GatesSpec` has `milestone: Option<String>` and `order: Option<u32>` fields with `serde(default, skip_serializing_if = "Option::is_none")`
- [ ] Existing TOML files without `milestone`/`order` fields still parse correctly (tested by `gates_spec_milestone_fields_absent_from_legacy_toml`)
- [ ] `gates_spec_rejects_unknown_fields` test passes unchanged
- [ ] `Milestone`, `ChunkRef`, `MilestoneStatus` types exist with `deny_unknown_fields`, `JsonSchema`, and `inventory::submit!` schema entries
- [ ] All three types are re-exported from `assay-types` crate root
- [ ] Schema snapshots accepted: `milestone-schema`, `chunk-ref-schema`, `milestone-status-schema`, updated `gates-spec-schema`
- [ ] `cargo test -p assay-types` fully green

## Verification

- `cargo test -p assay-types` — all tests pass including existing gate spec tests and new milestone type tests
- `cargo test -p assay-types -- gates_spec_rejects_unknown_fields` — must pass (backward compat proof)
- `cargo test -p assay-types -- gates_spec_milestone_fields` — new tests pass
- `cargo test -p assay-types -- milestone` — new type tests pass
- No pending `*.snap.new` files in `crates/assay-types/tests/snapshots/`

## Observability Impact

- Signals added/changed: `GatesSpec` now carries milestone metadata; downstream tools reading spec TOML files will silently see `None` for `milestone`/`order` on existing specs — no behavioral change
- How a future agent inspects this: `cargo test -p assay-types` is the verification surface; schema snapshots in `crates/assay-types/tests/snapshots/` show the locked type contract
- Failure state exposed: `toml::from_str::<GatesSpec>` returns an `Err` on unknown fields — error message includes the offending key and line number

## Inputs

- `crates/assay-types/src/gates_spec.rs` — existing struct to extend
- `crates/assay-types/src/orchestrate.rs` — model for enum + struct patterns, `inventory::submit!` usage, `deny_unknown_fields`, `serde(rename_all)`
- `crates/assay-types/tests/schema_snapshots.rs` — model for snapshot test format
- `.kata/DECISIONS.md` D062 (TOML format), D063 (chunk=spec, GatesSpec fields), D064 (milestone in assay-core)

## Expected Output

- `crates/assay-types/src/gates_spec.rs` — `GatesSpec` struct with two new optional fields
- `crates/assay-types/src/milestone.rs` — new file with `Milestone`, `ChunkRef`, `MilestoneStatus` types + schema entries + round-trip test
- `crates/assay-types/src/lib.rs` — new module declaration and re-exports
- `crates/assay-types/tests/schema_snapshots.rs` — 4 new snapshot test functions
- `crates/assay-types/tests/snapshots/` — new `.snap` files accepted by `cargo insta review`
