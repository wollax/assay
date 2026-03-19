---
estimated_steps: 7
estimated_files: 8
---

# T01: Add GossipStatus types and extend OrchestratorStatus

**Slice:** S03 — Gossip Mode
**Milestone:** M004

## Description

Add three new types (`KnowledgeEntry`, `KnowledgeManifest`, `GossipStatus`) to `assay-types::orchestrate`, extend `OrchestratorStatus` with a backward-compatible `gossip_status: Option<GossipStatus>` field, lock three new schema snapshots and regenerate the orchestrator-status snapshot, and fix the 9 construction sites that must now include `gossip_status: None`. This task must be completed before integration tests can compile (T02) and before the executor can be implemented (T03).

## Steps

1. Add `KnowledgeEntry`, `KnowledgeManifest`, `GossipStatus` structs to `crates/assay-types/src/orchestrate.rs` immediately after the `MeshStatus` block. Use exact same pattern: `#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]`, `#[serde(deny_unknown_fields)]`, doc comments, and `inventory::submit!` registry entries for each.
   - `KnowledgeEntry`: `session_name: String`, `spec: String`, `gate_pass_count: u32`, `gate_fail_count: u32`, `changed_files: Vec<String>` (with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`), `completed_at: DateTime<Utc>`
   - `KnowledgeManifest`: `run_id: String`, `entries: Vec<KnowledgeEntry>`, `last_updated_at: DateTime<Utc>`
   - `GossipStatus`: `sessions_synthesized: u32`, `knowledge_manifest_path: std::path::PathBuf`, `coordinator_rounds: u32`

2. Add `gossip_status: Option<GossipStatus>` field to `OrchestratorStatus` struct with `#[serde(default, skip_serializing_if = "Option::is_none")]` — place it after `mesh_status`.

3. Export `GossipStatus`, `KnowledgeEntry`, `KnowledgeManifest` from `crates/assay-types/src/lib.rs` orchestrate feature-gated pub use block (after `MeshStatus`).

4. Add three snapshot tests to `crates/assay-types/tests/schema_snapshots.rs`:
   - `knowledge_entry_schema_snapshot` → `"knowledge-entry-schema"`
   - `knowledge_manifest_schema_snapshot` → `"knowledge-manifest-schema"`
   - `gossip_status_schema_snapshot` → `"gossip-status-schema"`

5. Run `INSTA_UPDATE=always cargo test -p assay-types --features orchestrate` to generate new snapshots and update `orchestrator-status-schema.snap`. Inspect the `orchestrator-status-schema.snap` diff — it must be purely additive: new `gossip_status` property in `properties`, not in `required`.

6. Fix all 9 `OrchestratorStatus { ... }` construction sites by adding `gossip_status: None`:
   - `crates/assay-core/src/orchestrate/executor.rs`: ~lines 180, 437, 469
   - `crates/assay-core/src/orchestrate/mesh.rs`: ~lines 163, 336, 379
   - `crates/assay-types/src/orchestrate.rs`: unit test ~line 707
   - `crates/assay-mcp/src/server.rs`: ~lines 7252, 7300

7. Run `cargo build --workspace --features orchestrate` to confirm zero compilation errors across all crates.

## Must-Haves

- [ ] `KnowledgeEntry`, `KnowledgeManifest`, `GossipStatus` structs present with `deny_unknown_fields` and `inventory::submit!` entries
- [ ] `OrchestratorStatus.gossip_status: Option<GossipStatus>` with correct serde attributes
- [ ] Three new snapshot files committed: `knowledge-entry-schema.snap`, `knowledge-manifest-schema.snap`, `gossip-status-schema.snap`
- [ ] `orchestrator-status-schema.snap` regenerated with additive-only diff
- [ ] All 9 construction sites updated with `gossip_status: None`
- [ ] `cargo test -p assay-types --features orchestrate` passes with 0 failures
- [ ] Workspace compiles with 0 errors

## Verification

- `cargo test -p assay-types --features orchestrate` — all tests pass including 3 new snapshot tests
- `git diff crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap` — diff shows new nullable `gossip_status` property only, not in `required` array
- `cargo build --workspace --features orchestrate` — exits 0

## Observability Impact

- Signals added/changed: `GossipStatus` fields (`sessions_synthesized`, `coordinator_rounds`) will appear in `state.json` as observable gossip coordination progress
- How a future agent inspects this: `cat .assay/orchestrator/<run_id>/state.json | jq .gossip_status`
- Failure state exposed: `gossip_status: None` in state.json indicates the run is not in gossip mode (or the field was absent/not yet written)

## Inputs

- `crates/assay-types/src/orchestrate.rs` — existing `MeshStatus` and `OrchestratorStatus` as the pattern template
- `crates/assay-core/src/orchestrate/mesh.rs` — 3 construction sites needing `gossip_status: None`
- `crates/assay-core/src/orchestrate/executor.rs` — 3 construction sites needing `gossip_status: None`
- `crates/assay-mcp/src/server.rs` — 2 test construction sites needing `gossip_status: None`
- S03-RESEARCH.md — exact struct definitions and inventory entry names

## Expected Output

- `crates/assay-types/src/orchestrate.rs` — 3 new structs + 3 inventory entries + `gossip_status` field on `OrchestratorStatus`
- `crates/assay-types/src/lib.rs` — `GossipStatus`, `KnowledgeEntry`, `KnowledgeManifest` exported
- `crates/assay-types/tests/schema_snapshots.rs` — 3 new snapshot test functions
- `crates/assay-types/tests/snapshots/schema_snapshots__knowledge-entry-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__knowledge-manifest-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__gossip-status-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap` — updated (additive only)
- `crates/assay-core/src/orchestrate/executor.rs` — `gossip_status: None` at 3 sites
- `crates/assay-core/src/orchestrate/mesh.rs` — `gossip_status: None` at 3 sites
- `crates/assay-types/src/orchestrate.rs` — `gossip_status: None` in unit test
- `crates/assay-mcp/src/server.rs` — `gossip_status: None` at 2 sites
