---
id: T01
parent: S06
milestone: M001
provides:
  - RunManifest and ManifestSession types in assay-types
  - Schema snapshot tests locking the manifest contract
key_files:
  - crates/assay-types/src/manifest.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-types/tests/schema_snapshots.rs
key_decisions: []
patterns_established:
  - ManifestSession uses inline optional overrides (settings, hooks, prompt_layers) rather than embedding HarnessProfile
observability_surfaces:
  - Schema snapshot .snap files detect future type drift via cargo insta test
duration: 10m
verification_result: passed
completed_at: 2026-03-16
blocker_discovered: false
---

# T01: Define RunManifest and ManifestSession types with schema snapshots

**Created `RunManifest` and `ManifestSession` types with full derives, schema registry, and snapshot tests.**

## What Happened

Created `crates/assay-types/src/manifest.rs` with two types:
- `RunManifest`: top-level struct with `sessions: Vec<ManifestSession>` supporting `[[sessions]]` TOML array syntax
- `ManifestSession`: `spec: String` (required), `name: Option<String>`, `settings: Option<SettingsOverride>`, `hooks: Vec<HookContract>`, `prompt_layers: Vec<PromptLayer>`

Both types have full derives (`Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema`), `deny_unknown_fields`, `inventory::submit!` with kebab-case names, and doc comments on every type and field. Types are re-exported from `lib.rs` and have schema snapshot tests.

## Verification

- `cargo test -p assay-types -- run_manifest_schema_snapshot` — passed
- `cargo test -p assay-types -- manifest_session_schema_snapshot` — passed
- `cargo build -p assay-core` — compiles without errors
- Slice-level checks: schema snapshot tests pass (2/2). Remaining slice checks (`cargo test -p assay-core -- manifest`, `just ready`) depend on T02 work.

## Diagnostics

Schema snapshot `.snap` files at `crates/assay-types/tests/snapshots/` detect future type drift. Run `cargo insta test -p assay-types` to see diffs.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-types/src/manifest.rs` — new file with `RunManifest` and `ManifestSession` types
- `crates/assay-types/src/lib.rs` — added `pub mod manifest` and re-exports
- `crates/assay-types/tests/schema_snapshots.rs` — added 2 snapshot tests
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap` — locked schema
- `crates/assay-types/tests/snapshots/schema_snapshots__manifest-session-schema.snap` — locked schema
