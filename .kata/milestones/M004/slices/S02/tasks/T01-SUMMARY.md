---
id: T01
parent: S02
milestone: M004
provides:
  - MeshMemberState, MeshMemberStatus, MeshStatus types in assay-types::orchestrate
  - OrchestratorStatus.mesh_status optional field (backward-compatible)
  - Schema snapshots locked for all three new types and updated OrchestratorStatus
  - persist_state pub(crate) in executor.rs for reuse by mesh.rs
key_files:
  - crates/assay-types/src/orchestrate.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__mesh-member-state-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__mesh-member-status-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__mesh-status-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap
  - crates/assay-core/src/orchestrate/executor.rs
key_decisions:
  - MeshStatus uses deny_unknown_fields on both structs; MeshMemberState enum uses snake_case without deny_unknown_fields (enums don't support it)
  - mesh_status field on OrchestratorStatus uses serde(default, skip_serializing_if = "Option::is_none") for backward compatibility with existing state.json files
patterns_established:
  - New optional fields on OrchestratorStatus use serde(default, skip_serializing_if) — safe to add without breaking existing deserialization
  - All existing OrchestratorStatus construction sites in executor.rs get mesh_status: None explicitly — required because deny_unknown_fields prevents struct update syntax
observability_surfaces:
  - "cat .assay/orchestrator/<run_id>/state.json | jq .mesh_status — readable after T03 writes it"
  - MeshMemberState::Dead vs Completed distinguishes crash from normal exit in persisted state
duration: ~20min
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Add mesh status types and extend OrchestratorStatus

**Added `MeshMemberState`, `MeshMemberStatus`, `MeshStatus` types and extended `OrchestratorStatus` with an optional `mesh_status` field; all snapshot-locked and backward-compatible.**

## What Happened

Added three new types to `assay-types::orchestrate` following the exact patterns established by existing types in the module:

- `MeshMemberState` enum (`Alive`, `Suspect`, `Dead`, `Completed`) with `snake_case` serde and full derives
- `MeshMemberStatus` struct with `name`, `state`, `last_heartbeat_at` fields and `deny_unknown_fields`
- `MeshStatus` struct with `members: Vec<MeshMemberStatus>` and `messages_routed: u64` and `deny_unknown_fields`

Extended `OrchestratorStatus` with `pub mesh_status: Option<MeshStatus>` using `serde(default, skip_serializing_if = "Option::is_none")` — backward-compatible with existing persisted state files.

Fixed all three `OrchestratorStatus` construction sites in `executor.rs` to include `mesh_status: None` (required because `deny_unknown_fields` prevents struct update syntax from filling missing fields).

Added `inventory::submit!` entries for all three new types and re-exported them from `lib.rs` under `#[cfg(feature = "orchestrate")]`.

Made `persist_state` `pub(crate)` so `mesh.rs` (T02/T03) can reuse the atomic tempfile-rename pattern.

## Verification

- `cargo test -p assay-types --features orchestrate` — 254 tests pass (123 unit + 26 context + 44 roundtrip + 61 snapshots)
- `cargo test -p assay-core --features orchestrate` — 773 tests pass (768 unit + 5 integration)
- `ls crates/assay-types/tests/snapshots/ | grep mesh` — 3 new snap files confirmed present
- `grep "pub(crate) fn persist_state" crates/assay-core/src/orchestrate/executor.rs` — matches

## Diagnostics

After T03 writes mesh state: `cat .assay/orchestrator/<run_id>/state.json | jq .mesh_status` shows `members` list with per-session `state` and `last_heartbeat_at`, plus `messages_routed` counter. `MeshMemberState::Dead` vs `Completed` distinguishes crash from normal exit.

## Deviations

None. Plan followed exactly. One minor fix required beyond the plan: existing test `orchestrator_status_full_roundtrip` in `orchestrate.rs` constructed `OrchestratorStatus` by value and required adding `mesh_status: None` — this is expected mechanical work, not a deviation.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-types/src/orchestrate.rs` — Added MeshMemberState, MeshMemberStatus, MeshStatus types; extended OrchestratorStatus with mesh_status; 3 inventory entries; unit tests; fixed existing roundtrip test
- `crates/assay-types/src/lib.rs` — Added MeshMemberState, MeshMemberStatus, MeshStatus to pub use block
- `crates/assay-types/tests/schema_snapshots.rs` — Added 3 new snapshot test functions
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-member-state-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-member-status-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-status-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap` — Updated with mesh_status field, locked
- `crates/assay-core/src/orchestrate/executor.rs` — persist_state now pub(crate); 3 OrchestratorStatus construction sites updated with mesh_status: None
