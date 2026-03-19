---
id: S03
parent: M004
milestone: M004
provides:
  - KnowledgeEntry, KnowledgeManifest, GossipStatus types in assay-types with locked schema snapshots
  - OrchestratorStatus.gossip_status Option<GossipStatus> backward-compatible extension
  - run_gossip() full implementation in gossip.rs: parallel dispatch, PromptLayer injection, coordinator mpsc thread, atomic knowledge.json writes, gossip_status persistence
  - gossip_integration.rs with 2 passing integration tests (knowledge manifest population + prompt layer injection)
requires:
  - slice: S01
    provides: OrchestratorMode::Gossip dispatch, GossipConfig type, run_gossip() stub
affects:
  - S04 (consumes GossipStatus type and gossip_status field for orchestrate_status MCP surface)
key_files:
  - crates/assay-types/src/orchestrate.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__gossip-status-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__knowledge-entry-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__knowledge-manifest-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap
  - crates/assay-core/src/orchestrate/gossip.rs
  - crates/assay-core/tests/gossip_integration.rs
  - crates/assay-core/src/orchestrate/executor.rs
  - crates/assay-core/src/orchestrate/mesh.rs
  - crates/assay-mcp/src/server.rs
  - crates/assay-mcp/tests/mcp_handlers.rs
key_decisions:
  - D059: Gossip PromptLayer uses "Knowledge manifest: <path>" as machine-parseable line (mirrors D058 for mesh roster)
  - D060: Coordinator thread uses mpsc channel for session completion events; drain loop prevents last-message loss
patterns_established:
  - Gossip executor follows mesh.rs blueprint: thread::scope + bounded semaphore + panic isolation; coordinator thread replaces routing thread
  - persist_knowledge_manifest() uses identical tempfile+rename+sync_all atomic pattern as persist_state()
  - Coordinator drain loop (while let Ok(c) = rx.try_recv()) after Disconnected ensures no completions are lost on clean shutdown
  - drop(tx) in parent scope immediately after spawning all workers ensures coordinator exits cleanly
observability_surfaces:
  - "cat .assay/orchestrator/<run_id>/state.json | jq .gossip_status → { sessions_synthesized, knowledge_manifest_path, coordinator_rounds }"
  - "cat .assay/orchestrator/<run_id>/gossip/knowledge.json | jq '.entries | length' → synthesis count"
  - "cat .assay/orchestrator/<run_id>/gossip/knowledge.json | jq '[.entries[].session_name]' → which sessions synthesized"
  - gossip_status absent from state.json when mode != gossip (field omitted via skip_serializing_if)
drill_down_paths:
  - .kata/milestones/M004/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M004/slices/S03/tasks/T02-SUMMARY.md
  - .kata/milestones/M004/slices/S03/tasks/T03-SUMMARY.md
  - .kata/milestones/M004/slices/S03/tasks/T04-SUMMARY.md
duration: ~1.5h total (T01: 15min, T02: 10min, T03: 25min, T04: 5min)
verification_result: passed
completed_at: 2026-03-18
---

# S03: Gossip Mode

**Full `run_gossip()` implementation with coordinator mpsc thread, atomic `knowledge.json` writes, `"gossip-knowledge-manifest"` PromptLayer injection, and `gossip_status` persistence — both integration tests green, `just ready` exits 0 with 1264 tests passing.**

## What Happened

**T01** added three new types to `assay-types::orchestrate`: `KnowledgeEntry` (per-session synthesis record), `KnowledgeManifest` (cumulative manifest with run_id + entries), and `GossipStatus` (coordinator snapshot with sessions_synthesized, manifest path, coordinator_rounds). All three follow the exact derive/serde/doc-comment/`inventory::submit!` pattern established by `MeshStatus`. `OrchestratorStatus` was extended with `gossip_status: Option<GossipStatus>` using `serde(default, skip_serializing_if = "Option::is_none")` for full backward compatibility. All 9 `OrchestratorStatus { ... }` construction sites across `executor.rs`, `mesh.rs`, the orchestrate unit test, and `server.rs` were patched with `gossip_status: None`. Three schema snapshots were locked; the `orchestrator-status-schema.snap` diff was additive-only (new nullable property, not in `required`).

**T02** created `crates/assay-core/tests/gossip_integration.rs` with two tests defining the observable contract. `test_gossip_mode_knowledge_manifest` asserts that `gossip/knowledge.json` exists, deserializes as `KnowledgeManifest` with 3 entries, and `state.json.gossip_status.sessions_synthesized == 3`. `test_gossip_mode_manifest_path_in_prompt_layer` asserts that each session's prompt layers include a `"gossip-knowledge-manifest"` layer with a `"Knowledge manifest: "` line pointing under the assay dir. A key deviation was needed: since the stub never calls runners, runner-internal assertions pass vacuously — an `Arc<Mutex<usize>>` call counter was added as the primary failure gate. Both tests compiled cleanly and failed against the stub with assertion errors.

**T03** replaced the `run_gossip()` stub with a full implementation following the mesh.rs blueprint. A `GossipCompletion` struct carries per-session results from workers to the coordinator via an unbounded `mpsc::channel`. `persist_knowledge_manifest()` uses the same tempfile+rename+sync_all atomic pattern as `persist_state()`. Setup injects a `"gossip-knowledge-manifest"` PromptLayer into each session clone with the manifest path and writes an initial empty `knowledge.json`. The `thread::scope` runs a coordinator thread (owns `rx`, loops on `recv_timeout`, breaks on `Disconnected`, drains remaining completions) alongside N worker threads. `drop(tx)` is called in the parent scope immediately after spawning all workers so the coordinator exits when the last worker tx clone drops. A missed `gossip_status: None` construction site in `crates/assay-mcp/tests/mcp_handlers.rs` was also patched during this task.

**T04** ran `just ready` end-to-end: fmt ✓, lint ✓ (0 warnings), test ✓ (1264 tests, both gossip_integration tests included), deny ✓.

## Verification

- `cargo test -p assay-types --features orchestrate` — 64 tests passed including `knowledge_entry_schema_snapshot`, `knowledge_manifest_schema_snapshot`, `gossip_status_schema_snapshot`, and updated `orchestrator_status_schema_snapshot`
- `cargo test -p assay-core --features orchestrate --test gossip_integration` — both `test_gossip_mode_knowledge_manifest` and `test_gossip_mode_manifest_path_in_prompt_layer` pass with real filesystem operations
- `cargo test -p assay-mcp` — 141 tests pass (construction site patched)
- `cargo clippy --workspace --all-targets --features orchestrate -- -D warnings` — 0 warnings, exit 0
- `just ready` — exit 0, 1264 total tests (exceeds 1222+ threshold)
- Schema diff for `orchestrator-status-schema.snap`: additive-only (`gossip_status` nullable anyOf property added to `properties`, not in `required`)

## Requirements Advanced

- R037 (Gossip mode execution) — `run_gossip()` implements parallel dispatch with no dependency ordering; coordinator thread synthesizes completed sessions into `knowledge.json`; `gossip_status` persisted to `state.json` with `sessions_synthesized`, `coordinator_rounds`, and manifest path
- R038 (Gossip knowledge manifest injection) — `"gossip-knowledge-manifest"` PromptLayer injected at session launch time; coordinator atomically updates `knowledge.json` as sessions complete; running sessions can read it at any point during execution

## Requirements Validated

- R037 — Proved by `test_gossip_mode_knowledge_manifest`: 3 mock sessions → `knowledge.json` with 3 entries, `gossip_status.sessions_synthesized == 3`, real filesystem operations
- R038 — Proved by `test_gossip_mode_manifest_path_in_prompt_layer`: 2 mock sessions → each receives a PromptLayer named `"gossip-knowledge-manifest"` with a `"Knowledge manifest: "` line under the assay dir; real filesystem operations

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- **T02 — test_gossip_mode_manifest_path_in_prompt_layer failure gate**: The plan called for runner-internal `layer_errors` collection. Since the stub never calls runners, those assertions pass vacuously. Added `Arc<Mutex<usize>>` call counter + post-run `assert_eq!(calls, 2)` as the primary failure gate. More robust: fails against any stub or buggy impl that drops sessions.
- **T03 — changed_files field**: The plan referenced `result.merge_check.changed_files` but `MergeCheck` has `files: Vec<FileChange>` (each with `.path: String`). Used `mc.files.iter().map(|f| f.path.clone()).collect()` instead.
- **T03 — missed MCP test construction site**: One `OrchestratorStatus` construction in `crates/assay-mcp/tests/mcp_handlers.rs` was missed by T01. Patched during T03 (not a scope expansion — same T01 fix category).

## Known Limitations

- Gossip mode does not support mid-run manifest injection to already-running sessions — the manifest path is injected at launch time; running sessions must poll the file for updates. This is by design (no push mechanism), but means very short-lived sessions may complete before peers have synthesized anything useful.
- `GossipConfig.coordinator_interval_secs` defaults to 1 second; in high-throughput scenarios with many rapid completions, the coordinator may batch multiple completions between cycles (harmless, just means `coordinator_rounds` will be lower than `sessions_synthesized`).
- S04 is needed before `orchestrate_status` MCP tool surfaces `gossip_status` to real callers — the field exists in `OrchestratorStatus` but the MCP handler reads state.json directly and currently returns it as-is.

## Follow-ups

- S04 must wire `gossip_status` into the `orchestrate_status` MCP tool's response surface and surface mode in CLI run output
- S04 end-to-end tests should cover all three modes together (DAG, Mesh, Gossip) in a single integration test suite

## Files Created/Modified

- `crates/assay-types/src/orchestrate.rs` — Added `KnowledgeEntry`, `KnowledgeManifest`, `GossipStatus` structs; 3 `inventory::submit!` entries; `gossip_status` field on `OrchestratorStatus`; `gossip_status: None` in unit test
- `crates/assay-types/src/lib.rs` — Exported `GossipStatus`, `KnowledgeEntry`, `KnowledgeManifest`
- `crates/assay-types/tests/schema_snapshots.rs` — 3 new snapshot test functions
- `crates/assay-types/tests/snapshots/schema_snapshots__gossip-status-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__knowledge-entry-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__knowledge-manifest-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap` — regenerated with additive-only diff
- `crates/assay-core/src/orchestrate/gossip.rs` — full implementation: `GossipCompletion`, `persist_knowledge_manifest()`, `run_gossip()`, `run_gossip_calls_runner` unit test; stub tests removed
- `crates/assay-core/src/orchestrate/executor.rs` — `gossip_status: None` at 3 construction sites
- `crates/assay-core/src/orchestrate/mesh.rs` — `gossip_status: None` at 3 construction sites
- `crates/assay-core/tests/gossip_integration.rs` — new: 2 integration tests defining `run_gossip()` observable contract
- `crates/assay-mcp/src/server.rs` — `gossip_status: None` at 2 construction sites
- `crates/assay-mcp/tests/mcp_handlers.rs` — `gossip_status: None` at 1 missed construction site

## Forward Intelligence

### What the next slice should know
- `GossipStatus` is already in `OrchestratorStatus` and persisted to `state.json` — S04's `orchestrate_status` MCP handler just needs to deserialize and return it; no new types needed
- The `knowledge.json` file path is `<assay_dir>/orchestrator/<run_id>/gossip/knowledge.json` — always predictable from `run_id`
- Both gossip integration tests use `make_gossip_manifest` with `mode: OrchestratorMode::Gossip` and no `depends_on` — copy this pattern for S04 end-to-end tests

### What's fragile
- Coordinator drain loop — the `while let Ok(c) = rx.try_recv()` after `Disconnected` is critical to avoid losing completions when all workers finish simultaneously; removing it would cause `sessions_synthesized < actual` in fast-running test scenarios
- `drop(tx)` placement — must remain inside `thread::scope` but before the scope waits on threads; moving it outside breaks coordinator termination semantics

### Authoritative diagnostics
- `cat .assay/orchestrator/<run_id>/state.json | jq .gossip_status` — live coordination progress with session count and coordinator rounds
- `cat .assay/orchestrator/<run_id>/gossip/knowledge.json | jq '.entries | length'` — how many sessions have been synthesized

### What assumptions changed
- Original plan assumed `MergeCheck.changed_files` field — actually `MergeCheck.files: Vec<FileChange>` with `.path` on each entry; use `mc.files.iter().map(|f| f.path.clone())` to build the `changed_files: Vec<String>` for `KnowledgeEntry`
