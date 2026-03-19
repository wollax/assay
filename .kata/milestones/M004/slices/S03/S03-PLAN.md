# S03: Gossip Mode

**Goal:** Replace the `run_gossip()` stub with a full implementation: parallel session dispatch, knowledge manifest path injection at launch, a coordinator thread that synthesizes completed sessions into `knowledge.json`, and `gossip_status` surfaced in `state.json`. Three new types (`KnowledgeEntry`, `KnowledgeManifest`, `GossipStatus`) are added to `assay-types` with locked schema snapshots, and `OrchestratorStatus` gains a backward-compatible `gossip_status: Option<GossipStatus>` field.

**Demo:** `crates/assay-core/tests/gossip_integration.rs` passes with 2 tests (both compiled against the stub, then passing after T03): (1) 3 mock sessions → `knowledge.json` contains 3 entries, `gossip_status.sessions_synthesized == 3`; (2) 2 mock sessions → each session's prompt layers contain a `"gossip-knowledge-manifest"` layer with a `"Knowledge manifest: "` line pointing at the run directory. `just ready` passes with 0 warnings.

## Must-Haves

- `KnowledgeEntry`, `KnowledgeManifest`, `GossipStatus` types in `assay-types::orchestrate` with `deny_unknown_fields`, `inventory::submit!` schema registry entries, and locked snapshot files
- `OrchestratorStatus` extended with `gossip_status: Option<GossipStatus>` (additive, backward-compatible: `serde(default, skip_serializing_if = "Option::is_none")`)
- All 9 `OrchestratorStatus { ... }` construction sites updated to include `gossip_status: None`
- `orchestrator-status-schema.snap` regenerated (additive-only diff — new nullable property, not in `required`)
- `run_gossip()` full implementation in `gossip.rs`: parallel dispatch via `thread::scope` + bounded-concurrency semaphore, `PromptLayer` injection at launch with knowledge manifest path, coordinator thread using `std::sync::mpsc`, atomic `knowledge.json` writes via tempfile+rename, `gossip_status` persisted to `state.json` on each completion
- `GossipConfig.coordinator_interval_secs` honored as `recv_timeout` duration in coordinator loop
- `crates/assay-core/tests/gossip_integration.rs` with `test_gossip_mode_knowledge_manifest` and `test_gossip_mode_manifest_path_in_prompt_layer` — both passing against the full implementation
- `just ready` passes: fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓

## Proof Level

- This slice proves: integration (real filesystem operations, real `thread::scope` parallelism, real coordinator thread, real `knowledge.json` atomic writes)
- Real runtime required: no (mock session runners — no Claude, no git)
- Human/UAT required: no (integration tests cover R037 and R038 fully at the filesystem contract level)

## Verification

- `cargo test -p assay-types --features orchestrate` — all schema snapshot tests pass including `knowledge_entry_schema_snapshot`, `knowledge_manifest_schema_snapshot`, `gossip_status_schema_snapshot`; `orchestrator_status_schema_snapshot` passes with updated snap
- `cargo test -p assay-core --features orchestrate --test gossip_integration` — both `test_gossip_mode_knowledge_manifest` and `test_gossip_mode_manifest_path_in_prompt_layer` pass
- `cargo test -p assay-mcp` — all existing MCP tests pass (construction sites updated correctly)
- `just ready` → fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓
- Schema diff check: `git diff crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap` shows only additive change (new `gossip_status` property in `properties`, not in `required`)

## Observability / Diagnostics

- Runtime signals: `tracing::info!` per session start in `run_gossip()`; `tracing::debug!` per coordinator cycle with `sessions_synthesized` count; `tracing::warn!` per session with non-empty `depends_on`
- Inspection surfaces: `cat .assay/orchestrator/<run_id>/state.json | jq .gossip_status` → `{ sessions_synthesized, knowledge_manifest_path, coordinator_rounds }`; `cat .assay/orchestrator/<run_id>/gossip/knowledge.json` → `KnowledgeManifest` with entries
- Failure visibility: `state.json` persisted after each completion (best-effort) — partial results visible even if some sessions fail; coordinator loop exits cleanly on `Disconnected` error after draining remaining completions with `while let Ok(c) = rx.try_recv()`
- Redaction constraints: none (no secrets in knowledge manifest or gossip status)

## Integration Closure

- Upstream surfaces consumed: `run_gossip()` stub in `gossip.rs` (replaced); `OrchestratorStatus` in `assay-types::orchestrate`; `persist_state()` (pub(crate)) in `executor.rs`; `thread::scope` + semaphore + panic isolation pattern from `mesh.rs`
- New wiring introduced in this slice: `gossip_status` field on `OrchestratorStatus`; `KnowledgeEntry`/`KnowledgeManifest`/`GossipStatus` types exported from `assay-types`; `run_gossip()` full implementation calling `session_runner` closure and coordinator mpsc channel; `gossip_integration.rs` integration tests
- What remains before the milestone is truly usable end-to-end: S04 (observability — `orchestrate_status` MCP tool surfaces `gossip_status` for real callers; CLI shows mode in output; end-to-end tests covering all three modes together)

## Tasks

- [x] **T01: Add GossipStatus types and extend OrchestratorStatus** `est:45m`
  - Why: New types must exist with locked snapshots before integration tests can be written or the executor can be implemented; the cascade of 9 `OrchestratorStatus` construction sites must be fixed before anything compiles
  - Files: `crates/assay-types/src/orchestrate.rs`, `crates/assay-types/src/lib.rs`, `crates/assay-types/tests/schema_snapshots.rs`, `crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap`, `crates/assay-core/src/orchestrate/executor.rs`, `crates/assay-core/src/orchestrate/mesh.rs`, `crates/assay-types/src/orchestrate.rs` (unit test), `crates/assay-mcp/src/server.rs`
  - Do: (1) Add `KnowledgeEntry`, `KnowledgeManifest`, `GossipStatus` structs to `assay-types::orchestrate` after `MeshStatus` — use exact derives/serde/deny_unknown_fields pattern from `MeshStatus`; add `inventory::submit!` entries for each. (2) Add `gossip_status: Option<GossipStatus>` to `OrchestratorStatus` with `#[serde(default, skip_serializing_if = "Option::is_none")]`. (3) Export `GossipStatus`, `KnowledgeEntry`, `KnowledgeManifest` in `assay-types/src/lib.rs` orchestrate pub use block. (4) Add 3 snapshot tests to `schema_snapshots.rs`. (5) Run `INSTA_UPDATE=always cargo test -p assay-types --features orchestrate` to regenerate snaps — verify orchestrator-status snap diff is additive only. (6) Add `gossip_status: None` to all 9 `OrchestratorStatus { ... }` construction sites: executor.rs lines ~180/437/469, mesh.rs lines ~163/336/379, orchestrate.rs unit test ~line 707, server.rs lines ~7252/7300. (7) Run `cargo build --workspace --features orchestrate` to confirm zero compilation errors.
  - Verify: `cargo test -p assay-types --features orchestrate` passes; `cargo build -p assay-mcp` compiles; `git diff crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap` is additive-only
  - Done when: All schema snapshot tests pass; workspace compiles with 0 errors; orchestrator-status snapshot updated and committed

- [x] **T02: Write failing gossip integration tests** `est:30m`
  - Why: Tests define the observable contract for `run_gossip()` and must fail against the current stub — this proves the tests are real and the stub is correctly excluded. Writing tests first makes T03 implementation unambiguous.
  - Files: `crates/assay-core/tests/gossip_integration.rs`
  - Do: (1) Create `gossip_integration.rs` with `#![cfg(feature = "orchestrate")]` gate. (2) Copy helper structure from `mesh_integration.rs` (setup_temp_dir, make_pipeline_config). (3) Add `make_gossip_manifest(names: &[(&str, &str)]) -> RunManifest` helper (mode = Gossip, no depends_on). (4) Add `success_result(session)` helper returning `Ok(PipelineResult { ... })`. (5) Write `test_gossip_mode_knowledge_manifest`: 3 mock sessions (all succeed); after `run_gossip()` returns, assert (a) `<assay_dir>/orchestrator/<run_id>/gossip/knowledge.json` exists, (b) deserializes as `KnowledgeManifest`, (c) `manifest.entries.len() == 3`, (d) `state.json` has `gossip_status.sessions_synthesized == 3`, (e) each session name appears in entries. (6) Write `test_gossip_mode_manifest_path_in_prompt_layer`: 2 mock sessions; runner for each session (via name) checks session has a `PromptLayer` named `"gossip-knowledge-manifest"`, layer content contains a line starting with `"Knowledge manifest: "`, that path is under `<assay_dir>/orchestrator/<run_id>/`; runner returns `Ok(success_result(session))`. (7) Run `cargo test -p assay-core --features orchestrate --test gossip_integration` — both tests must compile but fail (stub returns empty results, no knowledge.json written).
  - Verify: `cargo test -p assay-core --features orchestrate --test gossip_integration 2>&1 | grep -E "FAILED|error"` shows test failures (not compilation errors)
  - Done when: Both tests compile cleanly and fail against the stub with assertion errors (not panics or compile errors)

- [x] **T03: Implement run_gossip() with coordinator thread** `est:1.5h`
  - Why: This is the core implementation task — replaces the stub with real parallel dispatch, PromptLayer injection, coordinator mpsc channel, atomic knowledge.json writes, and gossip_status persistence
  - Files: `crates/assay-core/src/orchestrate/gossip.rs`
  - Do: (1) Add imports: `std::sync::{mpsc, Arc, Condvar, Mutex}`, `std::sync::atomic::{AtomicUsize, Ordering}`, `std::panic::{self, AssertUnwindSafe}`, `chrono::Utc`, `ulid::Ulid`, `tempfile::NamedTempFile`, `assay_types::orchestrate::{GossipStatus, KnowledgeEntry, KnowledgeManifest, OrchestratorPhase, OrchestratorStatus, SessionRunState, SessionStatus}`, `assay_types::{PromptLayer, PromptLayerKind}`, `crate::orchestrate::executor::{OrchestratorConfig, OrchestratorResult, SessionOutcome, persist_state}`. (2) Define crate-local `GossipCompletion { session_name, spec, gate_pass_count, gate_fail_count, changed_files, completed_at }`. (3) Add private `persist_knowledge_manifest(gossip_dir: &Path, manifest: &KnowledgeManifest) -> Result<(), AssayError>` using NamedTempFile+rename+sync_all (same pattern as `persist_state`). (4) In `run_gossip()`: (a) generate run_id/started_at/wall_start; (b) create `<assay_dir>/orchestrator/<run_id>/gossip/` dir; (c) set `knowledge_manifest_path = run_dir.join("gossip/knowledge.json")`; (d) warn per session with non-empty depends_on; (e) build cloned sessions with `PromptLayer { kind: System, name: "gossip-knowledge-manifest", priority: -5, content: format!("# Gossip Mode — Knowledge Manifest\nKnowledge manifest: {path}\n...", path = knowledge_manifest_path.display()) }`; (f) initialize `KnowledgeManifest { run_id, entries: vec![], last_updated_at }` and write initial knowledge.json; (g) initialize `GossipStatus { sessions_synthesized: 0, knowledge_manifest_path: knowledge_manifest_path.clone(), coordinator_rounds: 0 }`; (h) persist initial `OrchestratorStatus` with `gossip_status: Some(...)` to state.json; (i) set up `active_count = Arc::new(AtomicUsize::new(session_count))`, `gossip_status_arc = Arc::new(Mutex::new(gossip_status))`, `session_statuses_arc = Arc::new(Mutex::new(initial_session_statuses))`, semaphore (same condvar pattern as mesh.rs); (j) create `mpsc::channel::<GossipCompletion>()`; (k) use `thread::scope` with coordinator thread (owns `rx`) and N worker threads (each owns a `tx` clone, drops it when done); (l) coordinator loop: `match rx.recv_timeout(coordinator_interval) { Ok(c) => push entry, write knowledge.json, increment sessions_synthesized + coordinator_rounds; Timeout => coordinator_rounds += 1; Disconnected => break }`; after break, drain with `while let Ok(c) = rx.try_recv()` then do final knowledge.json write; (m) `drop(tx)` in parent (before scope waits) so coordinator exits; (n) workers: acquire semaphore, mark Running, `catch_unwind`, compute gate counts from `result.gate_summary`, send `GossipCompletion` via tx clone, update session_statuses_arc, persist_state (best-effort), decrement active_count, release semaphore; (o) build final OrchestratorStatus and outcomes vec after scope. (5) Remove old stub tests and add one unit test: `run_gossip_stub_warns_for_depends_on` replaced by `run_gossip_calls_runner` checking runner is actually called.
  - Verify: `cargo test -p assay-core --features orchestrate --test gossip_integration` — both tests pass; `cargo test -p assay-core --features orchestrate` passes all unit tests
  - Done when: Both integration tests pass; `cargo clippy -p assay-core --features orchestrate` reports 0 warnings

- [x] **T04: just ready verification pass** `est:20m`
  - Why: Confirms the full workspace is green — fmt, lint, all tests, deny — before the slice is marked complete
  - Files: `crates/assay-core/src/orchestrate/gossip.rs` (lint fixes only if needed), any other file flagged by clippy
  - Do: (1) Run `just ready` and collect output. (2) Fix any fmt issues (`cargo fmt`). (3) Fix any clippy warnings (0 tolerance). (4) Confirm `cargo deny check` passes. (5) Confirm total test count ≥ previous (1222+) to verify no regressions. (6) Confirm `gossip_integration` test results are in the output.
  - Verify: `just ready` exits 0 with `fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓`
  - Done when: `just ready` passes end-to-end with 0 warnings and all tests green

## Files Likely Touched

- `crates/assay-types/src/orchestrate.rs`
- `crates/assay-types/src/lib.rs`
- `crates/assay-types/tests/schema_snapshots.rs`
- `crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap`
- `crates/assay-types/tests/snapshots/schema_snapshots__knowledge-entry-schema.snap` (new)
- `crates/assay-types/tests/snapshots/schema_snapshots__knowledge-manifest-schema.snap` (new)
- `crates/assay-types/tests/snapshots/schema_snapshots__gossip-status-schema.snap` (new)
- `crates/assay-core/src/orchestrate/executor.rs`
- `crates/assay-core/src/orchestrate/mesh.rs`
- `crates/assay-core/src/orchestrate/gossip.rs`
- `crates/assay-core/tests/gossip_integration.rs` (new)
- `crates/assay-mcp/src/server.rs`
