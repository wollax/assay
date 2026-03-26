---
estimated_steps: 5
estimated_files: 3
---

# T01: Define StateBackendConfig enum in assay-types with schema snapshot

**Slice:** S01 — StateBackend trait and CapabilitySet
**Milestone:** M010

## Description

Add `StateBackendConfig` to `assay-types` as a new serializable enum with `LocalFs` and `Custom` variants. Register it in the schema registry and lock its schema snapshot. This is a self-contained type-only change that does not touch `assay-core` or any orchestration code. It establishes the config-side contract that S02 will add to `RunManifest.state_backend`.

## Steps

1. Create `crates/assay-types/src/state_backend.rs` with:
   ```rust
   use schemars::JsonSchema;
   use serde::{Deserialize, Serialize};
   use crate::schema_registry;

   /// Backend configuration for state persistence.
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
   #[serde(rename_all = "snake_case")]
   pub enum StateBackendConfig {
       /// Local filesystem backend (default). No additional config needed.
       LocalFs,
       /// Custom third-party backend identified by name.
       Custom {
           /// Identifier for the backend implementation.
           name: String,
           /// Backend-specific configuration payload.
           config: serde_json::Value,
       },
   }

   inventory::submit! {
       schema_registry::SchemaEntry {
           name: "state-backend-config",
           generate: || schemars::schema_for!(StateBackendConfig),
       }
   }
   ```
2. In `crates/assay-types/src/lib.rs`, add `pub mod state_backend;` near the other module declarations, and add `pub use state_backend::StateBackendConfig;` to the pub use block at the top.
3. In `crates/assay-types/tests/schema_snapshots.rs`, add:
   ```rust
   #[test]
   fn state_backend_config_schema_snapshot() {
       let schema = schemars::schema_for!(assay_types::StateBackendConfig);
       assert_json_snapshot!("state-backend-config-schema", schema.to_value());
   }
   ```
4. Run `cargo test -p assay-types state_backend_config_schema_snapshot -- --force-update-snapshots` to generate the snapshot file. Then run `cargo insta review` (or simply verify the snapshot file was written to `crates/assay-types/src/snapshots/`).
5. Run `cargo test -p assay-types` to confirm all existing tests plus the new snapshot test pass.

## Must-Haves

- [ ] `crates/assay-types/src/state_backend.rs` exists with `StateBackendConfig` enum, two variants (`LocalFs`, `Custom { name, config }`), all required derives, `serde(rename_all = "snake_case")`, and `inventory::submit!` schema registration
- [ ] `StateBackendConfig` is re-exported from `assay_types` lib root (accessible as `assay_types::StateBackendConfig`)
- [ ] `state_backend_config_schema_snapshot` test exists in schema_snapshots.rs and passes
- [ ] Snapshot file `state-backend-config-schema.snap` present in the snapshots directory
- [ ] `cargo test -p assay-types` passes with no regressions

## Verification

- `cargo test -p assay-types` — all tests pass including `state_backend_config_schema_snapshot`
- `grep -r "StateBackendConfig" crates/assay-types/src/lib.rs` — confirms re-export present
- `ls crates/assay-types/src/snapshots/` — confirms snapshot file exists

## Observability Impact

- Signals added/changed: None — this is a pure type definition with no runtime signals
- How a future agent inspects this: `StateBackendConfig` schema snapshot is the canonical reference; schema registry makes it discoverable via `assay_types::schema_registry::all_entries()`
- Failure state exposed: None at this layer; `AssayError::Json` is the error path when deserializing a `StateBackendConfig` field in S02

## Inputs

- `crates/assay-types/src/schema_registry.rs` — pattern for `inventory::submit!` registration
- `crates/assay-types/tests/schema_snapshots.rs` — pattern for snapshot test function structure
- `crates/assay-types/src/orchestrate.rs` — reference for serde enum pattern with `rename_all`

## Expected Output

- `crates/assay-types/src/state_backend.rs` — new file with `StateBackendConfig` enum
- `crates/assay-types/src/lib.rs` — updated with `pub mod state_backend` and re-export
- `crates/assay-types/tests/schema_snapshots.rs` — updated with new snapshot test
- `crates/assay-types/src/snapshots/state-backend-config-schema.snap` — locked snapshot
