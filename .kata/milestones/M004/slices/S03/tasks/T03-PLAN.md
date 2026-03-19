---
estimated_steps: 8
estimated_files: 2
---

# T03: Implement run_gossip() with coordinator thread

**Slice:** S03 — Gossip Mode
**Milestone:** M004

## Description

Replace the `run_gossip()` stub with a full implementation. The implementation follows the `mesh.rs` blueprint closely: `thread::scope` + bounded-concurrency semaphore + panic isolation for workers, but replaces the routing thread with an `mpsc`-based coordinator thread that synthesizes session completions into an atomically-written `knowledge.json`. A `PromptLayer` carrying the manifest path is injected into each session clone before launch. `GossipStatus` is persisted to `state.json` after each completion.

## Steps

1. Add imports to `gossip.rs`: `std::sync::{mpsc, Arc, Condvar, Mutex}`, `std::sync::atomic::{AtomicUsize, Ordering}`, `std::panic::{self, AssertUnwindSafe}`, `std::path::{Path, PathBuf}`, `chrono::Utc`, `ulid::Ulid`, `tempfile::NamedTempFile`, `assay_types::orchestrate::{GossipStatus, KnowledgeEntry, KnowledgeManifest, OrchestratorPhase, OrchestratorStatus, SessionRunState, SessionStatus}`, `assay_types::{PromptLayer, PromptLayerKind}`, `crate::orchestrate::executor::{OrchestratorConfig, OrchestratorResult, SessionOutcome, persist_state}`.

2. Define crate-local `struct GossipCompletion { session_name: String, spec: String, gate_pass_count: u32, gate_fail_count: u32, changed_files: Vec<String>, completed_at: chrono::DateTime<chrono::Utc> }`.

3. Add private `fn persist_knowledge_manifest(gossip_dir: &Path, manifest: &KnowledgeManifest) -> Result<(), AssayError>`:
   - Same tempfile+rename+sync_all pattern as `persist_state()` in executor.rs
   - Write to `gossip_dir.join("knowledge.json")`

4. Implement `run_gossip()` — setup phase (before `thread::scope`):
   - Generate `run_id = Ulid::new().to_string()`, `started_at = Utc::now()`, `wall_start = Instant::now()`
   - Create `run_dir = assay_dir.join("orchestrator").join(&run_id)`; create `gossip_dir = run_dir.join("gossip")`; `fs::create_dir_all(&gossip_dir)`
   - Set `knowledge_manifest_path = run_dir.join("gossip").join("knowledge.json")`
   - Emit `tracing::warn!` per session with non-empty `depends_on`
   - Build `cloned_sessions: Vec<(String, ManifestSession)>` — for each session: resolve name, inject `PromptLayer { kind: PromptLayerKind::System, name: "gossip-knowledge-manifest".to_string(), priority: -5, content: format!("# Gossip Mode — Knowledge Manifest\nKnowledge manifest: {path}\nRead this file at any point during your session to discover what other sessions have already completed.\nThe manifest is updated atomically as sessions finish.", path = knowledge_manifest_path.display()) }` into `session_clone.prompt_layers`
   - Initialize `KnowledgeManifest { run_id: run_id.clone(), entries: vec![], last_updated_at: Utc::now() }` and write it via `persist_knowledge_manifest(&gossip_dir, &initial_manifest)`
   - Initialize `GossipStatus { sessions_synthesized: 0, knowledge_manifest_path: knowledge_manifest_path.clone(), coordinator_rounds: 0 }`
   - Build `initial_session_statuses: Vec<SessionStatus>` (all Pending)
   - Persist initial `OrchestratorStatus { run_id, phase: Running, failure_policy, sessions: initial_session_statuses.clone(), started_at, completed_at: None, mesh_status: None, gossip_status: Some(gossip_status.clone()) }` to `state.json`
   - Create `active_count = Arc::new(AtomicUsize::new(session_count))`, `gossip_status_arc = Arc::new(Mutex::new(gossip_status))`, `session_statuses_arc = Arc::new(Mutex::new(initial_session_statuses))`, semaphore
   - Determine `coordinator_interval`: `Duration::from_secs(manifest.gossip_config.as_ref().map(|gc| gc.coordinator_interval_secs).unwrap_or(GossipConfig::default().coordinator_interval_secs))` (default is 5s per GossipConfig::default)
   - Create `(tx, rx) = mpsc::channel::<GossipCompletion>()`

5. Implement `thread::scope` with coordinator thread + worker threads:
   - **Coordinator thread** (owns `rx`): loop `match rx.recv_timeout(coordinator_interval)`:
     - `Ok(completion)`: push `KnowledgeEntry { session_name, spec, gate_pass_count, gate_fail_count, changed_files, completed_at }` to local `knowledge_entries`, write `KnowledgeManifest { run_id, entries: knowledge_entries.clone(), last_updated_at: Utc::now() }` via `persist_knowledge_manifest`, update `gossip_status_arc` (`sessions_synthesized += 1, coordinator_rounds += 1`), best-effort `persist_state`
     - `Err(RecvTimeoutError::Timeout)`: `coordinator_rounds += 1`, update gossip_status_arc, best-effort `persist_state`
     - `Err(RecvTimeoutError::Disconnected)`: break
   - After break: `while let Ok(c) = rx.try_recv() { ... }` drain loop, then final `persist_knowledge_manifest`
   - **Worker threads**: acquire semaphore; mark Running in session_statuses_arc; `catch_unwind(AssertUnwindSafe(|| session_runner(session_clone, pipeline_config)))`; compute `gate_pass_count`/`gate_fail_count` from `result.gate_summary`; extract `changed_files` from `result.merge_check` if present; send `GossipCompletion { ... }` via `tx` clone (drop tx at end of worker); update session_statuses_arc; best-effort `persist_state` snapshot; `active_count.fetch_sub(1, Ordering::Release)`; release semaphore

6. After scope: `drop(tx)` must happen **before** `thread::scope` waits for the coordinator — ensure the `tx` created in step 4 is dropped in the parent scope after spawning all workers but before scope join. Simplest: call `drop(tx)` explicitly just before `thread::scope` returns.

7. Build final `OrchestratorStatus` with `gossip_status: Some(final_gossip)`, persist to state.json, build and return `OrchestratorResult` (same pattern as mesh.rs outcomes vec).

8. Remove the two stub unit tests (`run_gossip_returns_empty_result`, `run_gossip_emits_warn_for_depends_on`) and replace with one unit test `run_gossip_calls_runner` that verifies the runner closure is actually invoked (use an `AtomicBool` captured in the closure).

## Must-Haves

- [ ] `knowledge.json` created and updated atomically (tempfile+rename) in gossip_dir
- [ ] `PromptLayer` named `"gossip-knowledge-manifest"` injected with `"Knowledge manifest: <path>"` line
- [ ] Coordinator thread uses `mpsc` channel; exits cleanly on `Disconnected` after draining remaining messages
- [ ] `drop(tx)` in parent scope before coordinator waits — coordinator does not spin forever
- [ ] `gossip_status.sessions_synthesized` equals the number of completed sessions
- [ ] `GossipConfig.coordinator_interval_secs` honored as recv_timeout duration
- [ ] Both integration tests from T02 pass
- [ ] Existing stub unit tests removed; replaced with at least one real unit test
- [ ] `cargo clippy -p assay-core --features orchestrate` reports 0 warnings

## Verification

- `cargo test -p assay-core --features orchestrate --test gossip_integration` — both tests pass
- `cargo test -p assay-core --features orchestrate` — all unit tests pass (including new `run_gossip_calls_runner`)
- `cargo clippy -p assay-core --features orchestrate -- -D warnings` — exits 0

## Observability Impact

- Signals added/changed: `tracing::info!(session = %name, "gossip session starting")` per worker; `tracing::debug!(sessions_synthesized, coordinator_rounds, "gossip coordinator cycle")` per coordinator round; `tracing::warn!(session, "depends_on is ignored in Gossip mode")` per session with deps
- How a future agent inspects this: `cat .assay/orchestrator/<run_id>/state.json | jq .gossip_status` for live progress; `cat .assay/orchestrator/<run_id>/gossip/knowledge.json | jq '.entries | length'` for synthesis count
- Failure state exposed: `session.state = "failed"` + `session.error` in state.json visible per session; `gossip_status.sessions_synthesized` shows how far coordination got before failure

## Inputs

- `crates/assay-core/src/orchestrate/mesh.rs` — thread::scope structure, semaphore pattern, persist_state usage, panic isolation, OrchestratorStatus construction (primary blueprint)
- `crates/assay-core/tests/gossip_integration.rs` — exact assertions the implementation must satisfy (from T02)
- `crates/assay-core/src/orchestrate/executor.rs` — `persist_state` (pub(crate)), `SessionOutcome`, `OrchestratorConfig`
- `crates/assay-types/src/orchestrate.rs` — `GossipStatus`, `KnowledgeEntry`, `KnowledgeManifest` (from T01)
- S03-RESEARCH.md — `GossipCompletion` struct definition, coordinator thread layout, `drop(tx)` pitfall, drain loop pattern

## Expected Output

- `crates/assay-core/src/orchestrate/gossip.rs` — full implementation (replaces stub): `GossipCompletion` struct, `persist_knowledge_manifest()`, full `run_gossip()`, 1+ unit tests; no stub tests remaining
