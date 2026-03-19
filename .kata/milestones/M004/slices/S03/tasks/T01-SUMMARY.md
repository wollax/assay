---
id: T01
parent: S03
milestone: M004
provides:
  - KnowledgeEntry, KnowledgeManifest, GossipStatus types in assay-types
  - OrchestratorStatus.gossip_status Option<GossipStatus> field
  - Three new locked schema snapshots
  - All 9 OrchestratorStatus construction sites updated with gossip_status: None
key_files:
  - crates/assay-types/src/orchestrate.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__gossip-status-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__knowledge-entry-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__knowledge-manifest-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap
  - crates/assay-core/src/orchestrate/executor.rs
  - crates/assay-core/src/orchestrate/mesh.rs
  - crates/assay-mcp/src/server.rs
key_decisions:
  - Placed KnowledgeEntry, KnowledgeManifest, GossipStatus immediately after MeshStatus block (same file section) to keep gossip types co-located with mesh types for discoverability
patterns_established:
  - New gossip types follow exact same pattern as MeshStatus: derive block, deny_unknown_fields, doc comments, inventory::submit! schema registry entry
  - gossip_status on OrchestratorStatus uses same backward-compatible Option + serde(default, skip_serializing_if) pattern as mesh_status
observability_surfaces:
  - "cat .assay/orchestrator/<run_id>/state.json | jq .gossip_status → {sessions_synthesized, knowledge_manifest_path, coordinator_rounds}"
  - gossip_status absent from state.json when mode != gossip (field omitted via skip_serializing_if)
duration: ~15min
verification_result: passed
completed_at: 2026-03-18
blocker_discovered: false
---

# T01: Add GossipStatus types and extend OrchestratorStatus

**Added three new types (`KnowledgeEntry`, `KnowledgeManifest`, `GossipStatus`) to `assay-types::orchestrate`, extended `OrchestratorStatus` with `gossip_status: Option<GossipStatus>`, locked three schema snapshots, and patched all 9 construction sites.**

## What Happened

Added three new structs to `crates/assay-types/src/orchestrate.rs` immediately after the `MeshStatus` block, following the same derive/serde/doc-comment/inventory pattern:

- `KnowledgeEntry`: per-session knowledge record with `session_name`, `spec`, `gate_pass_count`, `gate_fail_count`, `changed_files` (sparse), `completed_at`
- `KnowledgeManifest`: cumulative manifest with `run_id`, `entries`, `last_updated_at`
- `GossipStatus`: coordination snapshot with `sessions_synthesized`, `knowledge_manifest_path`, `coordinator_rounds`

Added `gossip_status: Option<GossipStatus>` to `OrchestratorStatus` after `mesh_status`, with `#[serde(default, skip_serializing_if = "Option::is_none")]` for full backward compatibility. Exported all three types from `lib.rs` in the existing `orchestrate` feature-gated pub use block.

Added three new `inventory::submit!` schema registry entries and three snapshot test functions. Ran `INSTA_UPDATE=always cargo test -p assay-types --features orchestrate` to generate snapshots.

Fixed all 9 `OrchestratorStatus { ... }` construction sites:
- `executor.rs` lines ~188, ~446, ~478 → `gossip_status: None`
- `mesh.rs` lines ~172, ~344, ~387 → `gossip_status: None`
- `orchestrate.rs` unit test line ~826 → `gossip_status: None`
- `server.rs` lines ~7261, ~7309 → `gossip_status: None`

## Verification

- `cargo test -p assay-types --features orchestrate` → 64 passed, 0 failed (includes 3 new snapshot tests)
- `git diff crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap` → purely additive: `GossipStatus` definition added to `$defs`, `gossip_status` nullable anyOf property added to `properties`, NOT in `required` array
- `cargo build --workspace --features orchestrate` → `Finished dev profile [unoptimized + debuginfo]` — 0 errors

## Diagnostics

After T03 ships, inspect gossip coordination progress:
```bash
cat .assay/orchestrator/<run_id>/state.json | jq .gossip_status
# → { "sessions_synthesized": N, "knowledge_manifest_path": "...", "coordinator_rounds": N }
# → null if run is not in gossip mode
```

## Deviations

None — followed the plan exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-types/src/orchestrate.rs` — Added KnowledgeEntry, KnowledgeManifest, GossipStatus structs + 3 inventory::submit! entries; gossip_status field on OrchestratorStatus; gossip_status: None in unit test
- `crates/assay-types/src/lib.rs` — Exported GossipStatus, KnowledgeEntry, KnowledgeManifest
- `crates/assay-types/tests/schema_snapshots.rs` — 3 new snapshot test functions
- `crates/assay-types/tests/snapshots/schema_snapshots__gossip-status-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__knowledge-entry-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__knowledge-manifest-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap` — regenerated with additive-only diff
- `crates/assay-core/src/orchestrate/executor.rs` — gossip_status: None at 3 sites
- `crates/assay-core/src/orchestrate/mesh.rs` — gossip_status: None at 3 sites
- `crates/assay-mcp/src/server.rs` — gossip_status: None at 2 sites
