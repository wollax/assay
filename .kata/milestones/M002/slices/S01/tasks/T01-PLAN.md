---
estimated_steps: 5
estimated_files: 6
---

# T01: Add depends_on field and establish feature gate

**Slice:** S01 — Manifest Dependencies & DAG Validation
**Milestone:** M002

## Description

Add the `depends_on: Vec<String>` field to `ManifestSession` and establish the `cfg(feature = "orchestrate")` pattern on `assay-core`. This is the riskiest task in the slice — type evolution must preserve backward compatibility with existing manifests, and the feature gate pattern must compile correctly with and without the feature. Every subsequent M002 task depends on both of these being right.

## Steps

1. Add `depends_on: Vec<String>` to `ManifestSession` in `crates/assay-types/src/manifest.rs` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. Add doc comment explaining this references effective session names.
2. Run `cargo insta test -p assay-types --review` to update `manifest-session-schema` and `run-manifest-schema` snapshots. Accept the changes. Verify existing round-trip and parsing tests in `assay-core::manifest` still pass.
3. Add `[features]\norchestrate = []` to `crates/assay-core/Cargo.toml`. Create `crates/assay-core/src/orchestrate/mod.rs` with `pub mod dag;`. Create `crates/assay-core/src/orchestrate/dag.rs` with a placeholder struct and one test.
4. Add `#[cfg(feature = "orchestrate")]\npub mod orchestrate;` to `crates/assay-core/src/lib.rs`.
5. Verify compilation: `cargo check -p assay-core` (without feature — orchestrate module absent), `cargo check -p assay-core --features orchestrate` (with feature — module present), `cargo test -p assay-types`, `cargo test -p assay-core`.

## Must-Haves

- [ ] `ManifestSession.depends_on` field exists with correct serde attributes
- [ ] Existing TOML manifests without `depends_on` parse unchanged (backward compat)
- [ ] TOML manifests with `depends_on = ["session-a"]` parse correctly
- [ ] Schema snapshots updated and accepted for `manifest-session-schema` and `run-manifest-schema`
- [ ] `assay-core` Cargo.toml has `[features] orchestrate = []`
- [ ] `orchestrate/mod.rs` and `orchestrate/dag.rs` exist and compile behind feature gate
- [ ] `cargo check -p assay-core` succeeds (feature off)
- [ ] `cargo check -p assay-core --features orchestrate` succeeds (feature on)

## Verification

- `cargo test -p assay-types` — all existing tests pass, schema snapshots accepted
- `cargo test -p assay-core` — all existing tests pass (feature off, orchestrate absent)
- `cargo test -p assay-core --features orchestrate` — placeholder test passes
- `cargo check -p assay-core --no-default-features` — compiles without feature

## Observability Impact

- Signals added/changed: None — this is a type-level change with no runtime behavior
- How a future agent inspects this: schema snapshots in `crates/assay-types/tests/snapshots/` show the new field
- Failure state exposed: None

## Inputs

- `crates/assay-types/src/manifest.rs` — existing ManifestSession struct (adding field)
- `crates/assay-core/Cargo.toml` — existing Cargo.toml (adding features section)
- `crates/assay-core/src/lib.rs` — existing module registry (adding conditional module)
- S01-RESEARCH.md — serde attribute pattern, feature gate pattern

## Expected Output

- `crates/assay-types/src/manifest.rs` — ManifestSession with `depends_on: Vec<String>`
- `crates/assay-core/Cargo.toml` — `[features] orchestrate = []` section
- `crates/assay-core/src/lib.rs` — `#[cfg(feature = "orchestrate")] pub mod orchestrate;`
- `crates/assay-core/src/orchestrate/mod.rs` — module root
- `crates/assay-core/src/orchestrate/dag.rs` — placeholder with one passing test
- Updated schema snapshots in `crates/assay-types/tests/snapshots/`
