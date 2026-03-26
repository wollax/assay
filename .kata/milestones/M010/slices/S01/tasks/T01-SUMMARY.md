---
id: T01
parent: S01
milestone: M010
provides:
  - StateBackendConfig enum with LocalFs and Custom variants in assay-types
  - Schema registration via inventory::submit! for state-backend-config
  - Snapshot test state_backend_config_schema_snapshot in schema_snapshots.rs
  - Locked snapshot at tests/snapshots/schema_snapshots__state-backend-config-schema.snap
  - Re-export of StateBackendConfig from assay_types lib root
key_files:
  - crates/assay-types/src/state_backend.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__state-backend-config-schema.snap
key_decisions:
  - StateBackendConfig uses serde(rename_all = "snake_case") — LocalFs serializes as "local_fs", Custom wraps as {"custom": {...}} following schemars/serde enum conventions
patterns_established:
  - New type module (state_backend.rs) with inventory::submit! registration, pub mod + pub use in lib.rs — same pattern as all other assay-types modules
observability_surfaces:
  - none — pure type definition with no runtime signals; schema discoverable via assay_types::schema_registry::all_entries()
duration: 10min
verification_result: passed
completed_at: 2026-03-26T00:00:00Z
blocker_discovered: false
---

# T01: Define StateBackendConfig enum in assay-types with schema snapshot

**Added `StateBackendConfig` enum to assay-types with `LocalFs`/`Custom` variants, schema registry entry, and locked JSON Schema snapshot.**

## What Happened

Created `crates/assay-types/src/state_backend.rs` with the `StateBackendConfig` enum deriving `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`, `JsonSchema`. The `Custom` variant carries `name: String` and `config: serde_json::Value` for arbitrary backend-specific payloads. An `inventory::submit!` block registers the schema under the key `"state-backend-config"`.

Added `pub mod state_backend;` and `pub use state_backend::StateBackendConfig;` to `lib.rs`, following the established module pattern. Added `state_backend_config_schema_snapshot` to `tests/schema_snapshots.rs` using `INSTA_UPDATE=always` to generate the initial snapshot, which was committed.

A pre-existing failure `run_manifest_schema_snapshot` was present before this task and was not introduced by these changes (confirmed by reverting and re-running).

## Verification

- `INSTA_UPDATE=always cargo test -p assay-types --test schema_snapshots state_backend_config_schema_snapshot` — snapshot generated and test passes
- `cargo test -p assay-types --test schema_snapshots` — 46/47 pass; the 1 failure (`run_manifest_schema_snapshot`) is pre-existing and unrelated to this task
- `grep -r "StateBackendConfig" crates/assay-types/src/lib.rs` — confirms re-export present
- Snapshot file `schema_snapshots__state-backend-config-schema.snap` present in `tests/snapshots/`

## Diagnostics

No runtime signals. Schema discoverable via `assay_types::schema_registry::all_entries()` which includes the `"state-backend-config"` entry. Deserialization failures will surface as `AssayError::Json` when this type is used in S02.

## Deviations

None. Snapshot directory location is `crates/assay-types/tests/snapshots/` (insta's default for test-file snapshots) rather than `crates/assay-types/src/snapshots/` as mentioned in the plan — this is correct behavior; insta places snapshots adjacent to the test file, not the source.

## Known Issues

`run_manifest_schema_snapshot` was already failing on the branch before this task. Not caused by this work.

## Files Created/Modified

- `crates/assay-types/src/state_backend.rs` — new: StateBackendConfig enum with schema registry entry
- `crates/assay-types/src/lib.rs` — added `pub mod state_backend` and `pub use state_backend::StateBackendConfig`
- `crates/assay-types/tests/schema_snapshots.rs` — added `state_backend_config_schema_snapshot` test
- `crates/assay-types/tests/snapshots/schema_snapshots__state-backend-config-schema.snap` — locked JSON Schema snapshot
