---
id: T01
parent: S01
milestone: M002
provides:
  - depends_on field on ManifestSession with backward-compatible serde attributes
  - orchestrate feature gate pattern on assay-core
  - orchestrate/dag.rs placeholder module behind feature gate
key_files:
  - crates/assay-types/src/manifest.rs
  - crates/assay-core/Cargo.toml
  - crates/assay-core/src/lib.rs
  - crates/assay-core/src/orchestrate/mod.rs
  - crates/assay-core/src/orchestrate/dag.rs
key_decisions:
  - depends_on field placed last in ManifestSession to minimize diff noise in existing struct literals
patterns_established:
  - "Feature gate pattern: `#[cfg(feature = \"orchestrate\")] pub mod orchestrate;` in lib.rs with `[features] orchestrate = []` in Cargo.toml"
  - "Serde backward compat pattern: `#[serde(default, skip_serializing_if = \"Vec::is_empty\")]` for new Vec fields"
observability_surfaces:
  - none — type-level change with no runtime behavior
duration: ~10min
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Add depends_on field and establish feature gate

**Added `depends_on: Vec<String>` to `ManifestSession` with backward-compatible serde defaults and established the `cfg(feature = "orchestrate")` module pattern on assay-core.**

## What Happened

Added the `depends_on` field to `ManifestSession` in assay-types with `#[serde(default, skip_serializing_if = "Vec::is_empty")]` so existing manifests without the field parse unchanged. Updated all struct literal usages in assay-core (manifest.rs validation tests and pipeline.rs test helpers) to include `depends_on: vec![]`.

Ran `cargo insta test -p assay-types --review` and accepted the two updated schema snapshots (`manifest-session-schema` and `run-manifest-schema`).

Added `[features] orchestrate = []` to assay-core's Cargo.toml. Created `crates/assay-core/src/orchestrate/mod.rs` (re-exports `dag`) and `crates/assay-core/src/orchestrate/dag.rs` (placeholder `DependencyGraph` struct with one passing test). Wired `#[cfg(feature = "orchestrate")] pub mod orchestrate;` in lib.rs.

## Verification

- `cargo check -p assay-core` — ✅ compiles without orchestrate feature
- `cargo check -p assay-core --features orchestrate` — ✅ compiles with orchestrate feature
- `cargo check -p assay-core --no-default-features` — ✅ compiles
- `cargo test -p assay-types` — ✅ 76 tests passed (35 lib + 26 integration + 15 other), schema snapshots accepted
- `cargo test -p assay-core` — ✅ 664 tests passed (feature off, orchestrate absent)
- `cargo test -p assay-core --features orchestrate` — ✅ 665 tests passed (placeholder test included)

### Slice-level verification (partial — T01 is first of 4 tasks):
- `cargo test -p assay-core --features orchestrate` — ✅ passes
- `cargo test -p assay-core` (without feature) — ✅ passes
- `cargo test -p assay-types` — ✅ passes
- `cargo insta test --review` — ✅ no pending snapshots (accepted above)
- `just ready` — not yet (deferred to T04, the final integration task)

## Diagnostics

None — this is a type-level change. Schema snapshots in `crates/assay-types/tests/snapshots/` show the new `depends_on` field.

## Deviations

Had to update 7 `ManifestSession` struct literals across `crates/assay-core/src/manifest.rs` (3) and `crates/assay-core/src/pipeline.rs` (4) that used exhaustive field initialization. This was expected — `deny_unknown_fields` + exhaustive construction requires updating all call sites.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-types/src/manifest.rs` — added `depends_on: Vec<String>` field with doc comment and serde attributes
- `crates/assay-core/Cargo.toml` — added `[features] orchestrate = []` section
- `crates/assay-core/src/lib.rs` — added `#[cfg(feature = "orchestrate")] pub mod orchestrate;`
- `crates/assay-core/src/orchestrate/mod.rs` — new module root with `pub mod dag`
- `crates/assay-core/src/orchestrate/dag.rs` — placeholder `DependencyGraph` struct with one passing test
- `crates/assay-core/src/manifest.rs` — added `depends_on: vec![]` to 3 struct literals in tests
- `crates/assay-core/src/pipeline.rs` — added `depends_on: vec![]` to 4 struct literals in tests
- `crates/assay-types/tests/snapshots/schema_snapshots__manifest-session-schema.snap` — updated schema snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap` — updated schema snapshot
