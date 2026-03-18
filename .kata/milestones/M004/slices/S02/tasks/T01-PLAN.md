---
estimated_steps: 6
estimated_files: 8
---

# T01: Add mesh status types and extend OrchestratorStatus

**Slice:** S02 — Mesh Mode
**Milestone:** M004

## Description

Add three new types to `assay-types::orchestrate` — `MeshMemberState`, `MeshMemberStatus`, and `MeshStatus` — that represent SWIM-inspired membership tracking state for Mesh mode. Extend `OrchestratorStatus` with an optional `mesh_status` field. Lock schema snapshots for all three new types and the updated `OrchestratorStatus`. Make `persist_state` in `executor.rs` `pub(crate)` so `mesh.rs` can reuse the atomic tempfile-rename pattern.

This task establishes the complete type contract before implementation begins — T02 and T03 depend on these types being stable and snapshot-locked.

## Steps

1. In `crates/assay-types/src/orchestrate.rs`, add after the existing coordination mode types section:
   - `MeshMemberState` enum: variants `Alive`, `Suspect`, `Dead`, `Completed`; derives `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema`; `#[serde(rename_all = "snake_case")]`; no `deny_unknown_fields` (enums don't support it)
   - `MeshMemberStatus` struct: fields `name: String`, `state: MeshMemberState`, `last_heartbeat_at: Option<DateTime<Utc>>` (with `serde(default, skip_serializing_if = "Option::is_none")`); derives `Debug, Clone, Serialize, Deserialize, JsonSchema`; `#[serde(deny_unknown_fields)]`
   - `MeshStatus` struct: fields `members: Vec<MeshMemberStatus>`, `messages_routed: u64`; derives `Debug, Clone, Serialize, Deserialize, JsonSchema`; `#[serde(deny_unknown_fields)]`
   - Add `inventory::submit!` entries for all three new types: names `"mesh-member-state"`, `"mesh-member-status"`, `"mesh-status"`
   - Add unit tests at the bottom of the module: serde round-trip for `MeshMemberState` (all variants), round-trip for `MeshMemberStatus`, round-trip for `MeshStatus`, `deny_unknown_fields` rejection for both structs

2. In `crates/assay-types/src/orchestrate.rs`, add `mesh_status: Option<MeshStatus>` to `OrchestratorStatus`:
   ```rust
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub mesh_status: Option<MeshStatus>,
   ```
   The field goes after `completed_at`. Confirm the existing `orchestrator_status_deny_unknown_fields` test still passes (it should because `serde(default)` means missing field is accepted).

3. In `crates/assay-types/src/lib.rs`, add `MeshMemberState`, `MeshMemberStatus`, `MeshStatus` to the `pub use orchestrate::{ ... }` block under `#[cfg(feature = "orchestrate")]`.

4. In `crates/assay-types/tests/schema_snapshots.rs`, add three new snapshot tests under the `#[cfg(feature = "orchestrate")]` section:
   ```rust
   #[cfg(feature = "orchestrate")]
   #[test]
   fn mesh_member_state_schema_snapshot() {
       let schema = schemars::schema_for!(assay_types::MeshMemberState);
       assert_json_snapshot!("mesh-member-state-schema", schema.to_value());
   }

   #[cfg(feature = "orchestrate")]
   #[test]
   fn mesh_member_status_schema_snapshot() {
       let schema = schemars::schema_for!(assay_types::MeshMemberStatus);
       assert_json_snapshot!("mesh-member-status-schema", schema.to_value());
   }

   #[cfg(feature = "orchestrate")]
   #[test]
   fn mesh_status_schema_snapshot() {
       let schema = schemars::schema_for!(assay_types::MeshStatus);
       assert_json_snapshot!("mesh-status-schema", schema.to_value());
   }
   ```

5. Generate and accept new snapshots:
   ```sh
   INSTA_UPDATE=always cargo test -p assay-types --features orchestrate -- schema_snapshots
   ```
   This creates `mesh-member-state-schema.snap`, `mesh-member-status-schema.snap`, `mesh-status-schema.snap` and updates `orchestrator-status-schema.snap`. Accept all.

6. Make `persist_state` `pub(crate)` in `crates/assay-core/src/orchestrate/executor.rs`:
   ```rust
   // Change: fn persist_state(...)
   // To:     pub(crate) fn persist_state(...)
   ```

## Must-Haves

- [ ] `MeshMemberState`, `MeshMemberStatus`, `MeshStatus` compile and have full derives
- [ ] `MeshMemberState` has `Completed` variant (not just Alive/Suspect/Dead) — distinguishes normal exit from crash
- [ ] `OrchestratorStatus.mesh_status` uses `serde(default, skip_serializing_if = "Option::is_none")` — backward-compatible with existing state.json files
- [ ] 3 new snapshot files committed (`mesh-member-state-schema.snap`, `mesh-member-status-schema.snap`, `mesh-status-schema.snap`)
- [ ] `orchestrator-status-schema.snap` updated and accepted
- [ ] `persist_state` is `pub(crate)` in executor.rs
- [ ] `cargo test -p assay-types --features orchestrate` passes (all existing + new tests)
- [ ] `cargo test -p assay-core --features orchestrate` passes (backward-compat verification)

## Verification

- `cargo test -p assay-types --features orchestrate` — must pass with no failures
- `ls crates/assay-types/tests/snapshots/` — confirm 3 new `.snap` files exist
- `cargo test -p assay-core --features orchestrate` — must pass (confirms OrchestratorStatus change doesn't break deserialization of existing test state files)
- `grep "pub(crate) fn persist_state" crates/assay-core/src/orchestrate/executor.rs` — must match

## Observability Impact

- Signals added/changed: `MeshStatus` struct is the new persistence contract for mesh membership — `members[*].state` and `messages_routed` will be written to `state.json` by T03
- How a future agent inspects this: `cat .assay/orchestrator/<run_id>/state.json | jq .mesh_status` — readable without code changes after T03 writes it
- Failure state exposed: `MeshMemberState::Dead` visible per-member when heartbeat timeout exceeded; `MeshMemberState::Completed` distinguishes normal exit

## Inputs

- `crates/assay-types/src/orchestrate.rs` — existing pattern for derives, inventory submissions, deny_unknown_fields, serde defaults (follow exactly)
- `crates/assay-types/src/lib.rs` — existing `pub use orchestrate::{ ... }` block to extend
- `crates/assay-types/tests/schema_snapshots.rs` — existing pattern for `#[cfg(feature = "orchestrate")]` snapshot tests
- `crates/assay-core/src/orchestrate/executor.rs` — `persist_state` function to make `pub(crate)`

## Expected Output

- `crates/assay-types/src/orchestrate.rs` — 3 new types, 3 inventory entries, OrchestratorStatus extended, unit tests for new types
- `crates/assay-types/src/lib.rs` — 3 new re-exports
- `crates/assay-types/tests/schema_snapshots.rs` — 3 new snapshot test functions
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-member-state-schema.snap` — new, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-member-status-schema.snap` — new, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-status-schema.snap` — new, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap` — updated, locked
- `crates/assay-core/src/orchestrate/executor.rs` — `persist_state` now `pub(crate)`
