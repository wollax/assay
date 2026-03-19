# S03: Gossip Mode — Research

**Date:** 2026-03-18

## Summary

S03 replaces the `run_gossip()` stub from S01 with a full implementation: parallel session dispatch, knowledge manifest path injection at launch, a coordinator thread that synthesizes completed sessions into `knowledge.json`, and `gossip_status` surfaced in `state.json`. Three new types must be added to `assay-types::orchestrate` (`KnowledgeEntry`, `KnowledgeManifest`, `GossipStatus`) with locked schema snapshots, and `OrchestratorStatus` must gain a `gossip_status: Option<GossipStatus>` field (same backward-compatible pattern as S02's `mesh_status`).

The mesh executor (`mesh.rs`) is the direct model for gossip. The key difference: instead of a routing thread polling filesystem for inter-session messages, gossip has a coordinator thread that receives session completions via `mpsc` channel and atomically updates `knowledge.json`. The `std::sync::mpsc` channel provides clean backpressure and natural termination semantics (all senders drop → `recv_timeout` returns `Disconnected` → coordinator exits).

The implementation risk is low: mesh already proved the `thread::scope` + semaphore + `persist_state` pattern. The `mpsc` channel coordinator is simpler than the mesh routing thread (no filesystem polling loop). The main collateral work is updating all `OrchestratorStatus` struct literal construction sites (9 identified) to add `gossip_status: None`.

## Recommendation

Follow the mesh implementation blueprint closely. Use `std::sync::mpsc::channel` for coordinator-worker communication (cleaner than a shared `Mutex<Vec<>>` queue since channel disconnection provides natural termination). Write integration tests first (failing against the stub), then implement.

**Recommended task breakdown:**
- **T01**: Types + schema snapshots + OrchestratorStatus cascade fixes
- **T02**: Write failing integration tests (`gossip_integration.rs`)
- **T03**: Full `run_gossip()` implementation
- **T04**: `just ready` verification pass

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Atomic JSON persistence | `persist_state()` in `executor.rs` (pub(crate)) | Battle-tested tempfile+rename+fsync; already used by `mesh.rs` — call it directly |
| Parallel session dispatch with bounded concurrency | `(Mutex<usize>, Condvar)` semaphore + `thread::scope` — see `mesh.rs` lines 220–380 | Proven in S02; no external deps; copy the semaphore acquire/release pattern verbatim |
| Panic isolation per worker | `std::panic::catch_unwind(AssertUnwindSafe(...))` — see `mesh.rs` | Prevents one panicking session from aborting the whole run |
| Coordinator exit signal | `std::sync::mpsc` channel disconnect | All worker `Sender` clones drop when workers finish → `recv_timeout` returns `Disconnected` → coordinator loop breaks |

## Existing Code and Patterns

- `crates/assay-core/src/orchestrate/mesh.rs` — **Primary blueprint**. Copy the thread::scope structure, semaphore pattern, `active_count` (AtomicUsize), `session_statuses_arc` (Arc<Mutex<Vec<SessionStatus>>>), `persist_state` calls, and `SessionOutcome` construction. Replace the routing thread with the coordinator thread; replace `Arc<Mutex<MeshStatus>>` with `Arc<Mutex<GossipStatus>>`.

- `crates/assay-core/src/orchestrate/executor.rs` — `persist_state` (pub(crate)) at line ~107; `OrchestratorConfig`, `OrchestratorResult`, `SessionOutcome` types. `mesh.rs` already imports `persist_state` via `use crate::orchestrate::executor::persist_state` — do the same in `gossip.rs`.

- `crates/assay-core/tests/mesh_integration.rs` — Integration test structure to mirror. `gossip_integration.rs` follows the same setup: bare tempdir (no git), `PipelineConfig` pointing at `.assay/orchestrator/`, mock runners returning `Ok(PipelineResult)`.

- `crates/assay-types/src/orchestrate.rs` — All new types (`KnowledgeEntry`, `KnowledgeManifest`, `GossipStatus`) go here alongside `MeshStatus`, `MeshMemberState`, etc. Follow the exact same derives, `deny_unknown_fields`, serde defaults, doc comments, and `inventory::submit!` schema registry pattern established for `MeshStatus`.

- `crates/assay-types/tests/schema_snapshots.rs` — Add three new snapshot tests (`knowledge_entry_schema_snapshot`, `knowledge_manifest_schema_snapshot`, `gossip_status_schema_snapshot`). Regenerate with `INSTA_UPDATE=always cargo test -p assay-types --features orchestrate`.

- `crates/assay-types/src/lib.rs` lines 64–69 — Add `GossipStatus`, `KnowledgeEntry`, `KnowledgeManifest` to the `pub use orchestrate::{ ... }` block.

## New Types Required

### In `assay-types::orchestrate`

```rust
/// Single entry in the gossip knowledge manifest for a completed session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeEntry {
    pub session_name: String,
    pub spec: String,
    pub gate_pass_count: u32,
    pub gate_fail_count: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_files: Vec<String>,
    pub completed_at: DateTime<Utc>,
}

/// Gossip coordinator's knowledge manifest — persisted to
/// `.assay/orchestrator/<run_id>/gossip/knowledge.json`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeManifest {
    pub run_id: String,
    pub entries: Vec<KnowledgeEntry>,
    pub last_updated_at: DateTime<Utc>,
}

/// Aggregate gossip coordination status snapshot.
/// Written into OrchestratorStatus::gossip_status.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GossipStatus {
    pub sessions_synthesized: u32,
    /// Absolute path to knowledge.json.
    pub knowledge_manifest_path: std::path::PathBuf,
    pub coordinator_rounds: u32,
}
```

### On `OrchestratorStatus` (additive, backward-compatible)

```rust
/// Gossip coordination status (present only when `mode = "gossip"`).
#[serde(default, skip_serializing_if = "Option::is_none")]
pub gossip_status: Option<GossipStatus>,
```

Same pattern as `mesh_status`. The `orchestrator-status-schema.snap` must be regenerated.

## Construction Site Cascade

Adding `gossip_status` to `OrchestratorStatus` breaks all struct literal constructions — they must each add `gossip_status: None`. **9 sites identified:**

| File | Lines (approx) | Notes |
|------|---------------|-------|
| `assay-core/src/orchestrate/executor.rs` | 180, 437, 469 | DAG executor — add `gossip_status: None` |
| `assay-core/src/orchestrate/mesh.rs` | 163, 336, 379 | Mesh executor — add `gossip_status: None` |
| `assay-types/src/orchestrate.rs` | 707 | Unit test in orchestrate.rs |
| `assay-mcp/src/server.rs` | 7252, 7300 | Two test constructions (already needed `mesh_status: None` in S02) |

No `Default` impl for `OrchestratorStatus` is warranted (it has required fields with no defaults). Update each site individually.

## Coordinator Thread Design

Use `std::sync::mpsc::channel::<GossipCompletion>()` where `GossipCompletion` is a crate-local struct:

```rust
struct GossipCompletion {
    session_name: String,
    spec: String,
    gate_pass_count: u32,
    gate_fail_count: u32,
    changed_files: Vec<String>,   // empty in S03 — PipelineResult has no changed_files
    completed_at: DateTime<Utc>,
}
```

**Thread::scope layout:**
```
scope:
  ├── coordinator thread (owns rx)
  │     loop {
  │         match rx.recv_timeout(coordinator_interval) {
  │             Ok(completion)              => push KnowledgeEntry, write knowledge.json, increment sessions_synthesized, coordinator_rounds += 1
  │             Err(RecvTimeoutError::Timeout)    => coordinator_rounds += 1 (interval tick)
  │             Err(RecvTimeoutError::Disconnected) => break
  │         }
  │     }
  │     // final knowledge.json write after loop exits
  └── N worker threads (each owns a tx clone)
        acquire semaphore
        mark Running
        catch_unwind(runner)
        compute gate counts from PipelineResult.gate_summary
        tx.send(GossipCompletion { ... })
        drop(tx)  // implicit when worker scope ends
        update session_statuses_arc
        persist_state (best-effort)
        decrement active_count
        release semaphore
```

`drop(tx)` in the parent scope after spawning all workers ensures the coordinator exits when all workers finish (all sender clones drop).

**Atomically writing `knowledge.json`:** Add a new private `persist_knowledge_manifest` function in `gossip.rs` — same `NamedTempFile` + `rename` + `sync_all` pattern as `persist_state` in `executor.rs`.

## PromptLayer Injection

At session launch (before cloning sessions), inject a `PromptLayer` with the knowledge manifest path:

```rust
PromptLayer {
    kind: PromptLayerKind::System,
    name: "gossip-knowledge-manifest".to_string(),
    priority: -5,
    content: format!(
        "# Gossip Mode — Knowledge Manifest\nKnowledge manifest: {path}\nRead this file at any point during your session to discover what other sessions have already completed.\nThe manifest is updated atomically as sessions finish.",
        path = knowledge_manifest_path.display()
    ),
}
```

The `Knowledge manifest: <path>` line (starts with `"Knowledge manifest: "`) is machine-parseable by integration tests and agents, mirroring the `"Outbox: "` convention from D058.

## `changed_files` in KnowledgeEntry

`PipelineResult` does not carry `changed_files`. In S03:
- `changed_files: vec![]` for all entries from `PipelineResult` results
- `MergeCheck.files` (in `PipelineResult.merge_check`) has `Vec<FileChange>` with paths — but `merge_check` may be `None` for mock runners
- For mock runner integration tests, empty `changed_files` is correct and acceptable

To populate `changed_files` from real pipeline results, use `result.merge_check.as_ref().map(|mc| mc.files.iter().map(|f| f.path.clone()).collect()).unwrap_or_default()` — but only if `merge_check` is present. This gives best-effort file list without breaking tests.

## Integration Test Plan

File: `crates/assay-core/tests/gossip_integration.rs`

### `test_gossip_mode_knowledge_manifest`

3 mock sessions, all succeed. Verify:
1. `knowledge.json` exists at `<assay_dir>/orchestrator/<run_id>/gossip/knowledge.json`
2. Deserializes as `KnowledgeManifest`
3. `manifest.entries.len() == 3`
4. `gossip_status.sessions_synthesized == 3` in `state.json`
5. `gossip_status.knowledge_manifest_path` in `state.json` matches the actual path
6. Each entry's `session_name` appears in the manifest entries

### `test_gossip_mode_manifest_path_in_prompt_layer`

2 mock sessions. Runner verifies:
- Session has a `PromptLayer` named `"gossip-knowledge-manifest"`
- Layer content contains `"Knowledge manifest: "` on a parseable line
- The path from the layer points to a location under the run directory

Both tests use mock runners returning `Ok(PipelineResult)` with no real git or agent operations — identical pattern to `mesh_integration.rs`.

## Constraints

- **Zero traits** (D001): `run_gossip()` is a free function, `GossipCompletion` is a crate-local struct — no traits introduced.
- **Sync-only** (D017): `std::sync::mpsc` is used, not `tokio`. `thread::scope` as in mesh.
- **`deny_unknown_fields`** on all new persisted types (`KnowledgeEntry`, `KnowledgeManifest`, `GossipStatus`).
- **Schema snapshots must be locked** — not just generated. Use `INSTA_UPDATE=always` then commit.
- **MCP additive-only** (D005): `gossip_status` is added to `OrchestratorStatus` with `serde(default, skip_serializing_if = "Option::is_none")` — existing readers see `None` and work unchanged.
- **CLI/MCP gossip call sites** (`execute_gossip` in run.rs, `spawn_blocking` gossip arm in server.rs) use `unreachable!()` session runner and are **not changed in S03** — that's S04's job. The real implementation is exercised only by integration tests in S03.
- **`GossipConfig.coordinator_interval_secs`** (already exists from S01) must be honored as the `recv_timeout` duration in the coordinator loop.

## Common Pitfalls

- **`tx` not dropped before scope exits** — If the parent holds a live `Sender` clone when `thread::scope` waits for the coordinator thread, `recv_timeout` will never return `Disconnected` and the coordinator will spin until timeout. Always `drop(tx)` in the parent after spawning all workers (before the scope's implicit join).

- **Snapshot test cascade** — Adding `gossip_status` to `OrchestratorStatus` changes the `orchestrator-status-schema.snap`. Run `INSTA_UPDATE=always cargo test -p assay-types --features orchestrate` and review the diff — it must be purely additive (new optional `gossip_status` property in JSON schema `properties`, not in `required`).

- **`PathBuf` in `GossipStatus`** — `schemars` renders `PathBuf` as `"type": "string"` which is correct. Serde serializes it as a JSON string with the OS-native path. No special handling needed.

- **Coordinator rounds timing** — The coordinator may exit before processing the last batch of completions if `recv_timeout` duration is longer than the window between last completion and disconnect. Use `recv_timeout` with a reasonably short interval (e.g., 100ms even if `coordinator_interval_secs` is larger — the interval controls manifest flush frequency, not receive polling). Actually: process every `recv()` immediately, use `recv_timeout` only to implement interval-based flushing. Or simply call `try_recv` in a drain loop after a sleep.

- **Integration test file path** — `crates/assay-core/tests/gossip_integration.rs` must be feature-gated: `#![cfg(feature = "orchestrate")]`. The integration test file won't be compiled without the feature.

## Open Risks

- **Coordinator loop exits before last completion processed** — if all workers finish and drop their senders before the coordinator drains the last message, the `Disconnected` error fires before the last `KnowledgeEntry` is written. Mitigation: after the `Disconnected` break, drain any remaining messages from `rx` with `while let Ok(c) = rx.try_recv()` before writing the final `knowledge.json`. This is a post-scope cleanup pattern.

- **`OrchestratorStatus` construction cascade** — 9 sites to update. Any missed site fails to compile with "missing field `gossip_status`". This is caught immediately by `cargo build` — no silent failures.

- **`INSTA_UPDATE=always` snapshot regeneration** — The `orchestrator-status-schema.snap` changed in S02 (added `mesh_status`). Adding `gossip_status` changes it again. The diff must be reviewed carefully to confirm the `orchestrator-status-schema.snap` change is additive only (new nullable property, not in `required`, not breaking `additionalProperties: false`).

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / std::sync::mpsc | — | built-in; no skill needed |
| schemars PathBuf | — | no special handling needed |

## Sources

- `mesh.rs` full implementation — thread::scope, semaphore, persist_state, PromptLayer injection pattern (local codebase)
- `mesh_integration.rs` — integration test structure for mock runners (local codebase)
- S02-SUMMARY.md — Forward intelligence: OrchestratorStatus construction cascade mitigation, confirmed `persist_state` is reusable via `pub(crate)` visibility (local codebase)
- D054: OrchestratorStatus extended with optional mode-specific fields (DECISIONS.md)
- D058: Mesh roster uses machine-parseable "Outbox: <path>" line — apply same convention as "Knowledge manifest: <path>" for gossip (DECISIONS.md)
