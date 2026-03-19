---
estimated_steps: 8
estimated_files: 1
---

# T03: Implement run_mesh() full body

**Slice:** S02 — Mesh Mode
**Milestone:** M004

## Description

Replace the `run_mesh()` stub body with a complete implementation. The implementation:
1. Creates `inbox/` and `outbox/` directories per session
2. Builds a roster `PromptLayer` listing each peer's name and inbox path
3. Clones sessions and appends the roster layer to each clone
4. Spawns a routing thread and session workers inside `thread::scope`
5. Routing thread polls outbox subdirectories and moves files to target inboxes
6. Session workers call the runner, write a `completed` sentinel, update membership state
7. Persists `OrchestratorStatus` (including `mesh_status`) to `state.json` after each session completes

The signature is fixed per D052: `run_mesh<F>(manifest, config, pipeline_config, session_runner)` — do not change it.

## Steps

1. **Imports and module-level setup** in `mesh.rs`: replace all imports with the full set needed:
   - `std::collections::HashMap`, `std::io::Write`, `std::panic::{self, AssertUnwindSafe}`, `std::path::PathBuf`, `std::sync::{Arc, Mutex}`, `std::sync::atomic::{AtomicUsize, Ordering}`, `std::time::{Duration, Instant}`
   - `chrono::Utc`, `tempfile::NamedTempFile`, `ulid::Ulid`
   - `assay_types::{ManifestSession, PromptLayer, PromptLayerKind}`, `assay_types::orchestrate::{FailurePolicy, MeshMemberState, MeshMemberStatus, MeshStatus, OrchestratorPhase, OrchestratorStatus, SessionRunState, SessionStatus}`
   - `crate::orchestrate::executor::{OrchestratorConfig, OrchestratorResult, persist_state}`
   - `crate::pipeline::{PipelineConfig, PipelineError, PipelineResult}`
   - `crate::error::AssayError`

2. **Pre-scope setup** (before `std::thread::scope`):
   - `run_id = Ulid::new().to_string(); started_at = Utc::now(); wall_start = Instant::now()`
   - `run_dir = pipeline_config.assay_dir.join("orchestrator").join(&run_id)` — create with `fs::create_dir_all`
   - For each session, compute `name = session.name.as_deref().unwrap_or(&session.spec).to_string()`; emit `tracing::warn!` if `!session.depends_on.is_empty()`
   - Create `mesh_dir = run_dir.join("mesh").join(&name)`; create `inbox_path = mesh_dir.join("inbox")` and `outbox_path = mesh_dir.join("outbox")` via `fs::create_dir_all`
   - Build `name_to_inbox: HashMap<String, PathBuf>` mapping each session name to its inbox path
   - Build roster content string for each session: header + list of all peers (excluding self) with `Peer: <name>  Inbox: <path>`; also include self's own outbox path as `Outbox: <own_outbox_path>` for runners to discover
   - Build pre-cloned sessions `Vec<(String, ManifestSession)>`: for each (name, session), clone ManifestSession, push `PromptLayer { kind: PromptLayerKind::System, name: "mesh-roster".to_string(), content: <roster_for_this_session>, priority: -5 }` to clone's `prompt_layers`
   - Initialize `session_statuses: Vec<SessionStatus>` — all Pending; initial `MeshStatus { members: all Alive, messages_routed: 0 }`
   - Persist initial `OrchestratorStatus` (with `mesh_status: Some(initial_mesh_status)`)
   - `active_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(sessions.len()))`
   - `mesh_status_arc: Arc<Mutex<MeshStatus>>` and `session_statuses_arc: Arc<Mutex<Vec<SessionStatus>>>`

3. **Bounded concurrency dispatch** — instead of DAG, use semaphore-style `AtomicUsize` or simply spawn all workers at once (let OS schedule) up to `effective_concurrency = config.max_concurrency.min(session_count)`. Since there's no DAG dispatch loop, the simplest correct implementation is: chunk the cloned sessions into batches of `effective_concurrency` and spawn each batch inside the scope. Or equivalently: use a work-queue Mutex with workers self-dispatching. **Simpler**: for the S02 scope, spawn all sessions into the scope immediately (rely on OS scheduling for parallelism). If `max_concurrency < session_count`, use a `Mutex<()>` slot semaphore approach: each worker acquires a semaphore slot (via `Arc<Mutex<Vec<()>>>` or simpler `Arc<AtomicUsize>` slot counter decrement pattern). For correctness per the spec, implement: maintain `in_flight: Arc<AtomicUsize>` initialized to 0, and use a condvar to wake the dispatch loop — OR accept that for mesh mode (no ordering needed) spawning all at once within `thread::scope` and letting them run concurrently is correct behavior; `max_concurrency` is advisory for the purpose of this slice. **Decision**: spawn all sessions at once, respect `max_concurrency` via a counting semaphore pattern with `Arc<(Mutex<usize>, Condvar)>` (same pattern as executor.rs but without DAG). Keep it simple: just spawn all into scope; if `effective_concurrency < session_count`, at the beginning of each worker acquire a semaphore.

4. **Spawn routing thread** inside `std::thread::scope`:
   ```rust
   let active_count_ref = &active_count;
   let name_to_inbox_ref = &name_to_inbox;
   let mesh_status_arc_ref = &mesh_status_arc;
   
   scope.spawn(move || {
       while active_count_ref.load(Ordering::Acquire) > 0 {
           // Poll each session's outbox subdirs
           for (source_name, session_dir) in &session_dirs {
               let outbox = session_dir.join("outbox");
               if let Ok(targets) = std::fs::read_dir(&outbox) {
                   for target_entry in targets.flatten() {
                       let target_name = target_entry.file_name().to_string_lossy().to_string();
                       if let Some(inbox) = name_to_inbox_ref.get(&target_name) {
                           if let Ok(msgs) = std::fs::read_dir(target_entry.path()) {
                               for msg in msgs.flatten() {
                                   let dst = inbox.join(msg.file_name());
                                   if std::fs::rename(msg.path(), &dst).is_ok() {
                                       let mut ms = mesh_status_arc_ref.lock().unwrap();
                                       ms.messages_routed += 1;
                                       tracing::debug!(from=%source_name, to=%target_name, "routed message");
                                   }
                               }
                           }
                       } else {
                           tracing::warn!(target=%target_name, "unknown outbox target — leaving file in place");
                       }
                   }
               }
           }
           std::thread::sleep(Duration::from_millis(50));
       }
   });
   ```

5. **Spawn session workers** inside `thread::scope`: for each `(name, session_clone)`, capture shared Arcs by reference:
   ```rust
   scope.spawn(move || {
       // Optional: bounded concurrency semaphore acquire
       let session_start = Instant::now();
       let result = panic::catch_unwind(AssertUnwindSafe(|| {
           session_runner(&session_clone, pipeline_config)
       }));
       let session_duration = session_start.elapsed();
       
       // Write completed sentinel
       let _ = std::fs::write(session_dir.join("completed"), b"");
       
       // Decrement active count (signals routing thread to exit when 0)
       active_count.fetch_sub(1, Ordering::Release);
       
       // Update session status and member state
       let member_state = match &result {
           Ok(Ok(_)) => MeshMemberState::Completed,
           _ => MeshMemberState::Dead,
       };
       // Update session_statuses_arc and mesh_status_arc
       // Persist OrchestratorStatus snapshot (best-effort)
   });
   ```

6. **After scope exits**, build `OrchestratorResult`:
   - Collect outcomes from the locked `session_statuses`
   - Determine final phase (Completed / PartialFailure)
   - Persist final `OrchestratorStatus` with final `mesh_status`
   - Return `Ok(OrchestratorResult { run_id, outcomes, duration: wall_start.elapsed(), failure_policy: config.failure_policy })`

7. **Wire `session_dirs` map** — build it before the scope as `Vec<(String, PathBuf)>` mapping session name to its mesh dir (`.assay/orchestrator/<run_id>/mesh/<name>`). This is captured by reference in the routing thread.

8. **Update existing stub unit tests** in mesh.rs to work with the new implementation (they assert `outcomes.is_empty()` — this will no longer be true for the `run_mesh_returns_empty_result` test since the real impl calls the runner). Replace stub unit tests with implementation-accurate unit tests or remove the `run_mesh_emits_warn_for_depends_on` test and keep only the integration tests in `mesh_integration.rs` for behavior verification.

## Must-Haves

- [ ] `run_mesh()` signature unchanged: `run_mesh<F>(manifest: &RunManifest, config: &OrchestratorConfig, pipeline_config: &PipelineConfig, session_runner: &F) -> Result<OrchestratorResult, AssayError>`
- [ ] Roster `PromptLayer` injected into each session clone before `session_runner` is called; manifest is not mutated
- [ ] `inbox/` and `outbox/` dirs created for every session before workers launch
- [ ] Routing thread runs inside `thread::scope` and exits when all session workers complete (`active_count == 0`)
- [ ] Routing thread routes files from `outbox/<target_name>/<filename>` to `target_name/inbox/<filename>`; unrecognized target names emit `tracing::warn`
- [ ] Session workers write `completed` sentinel file after runner returns (regardless of success/failure)
- [ ] `MeshMemberStatus.state` transitions to `Completed` on successful runner return, `Dead` on error/panic
- [ ] `OrchestratorStatus` with `mesh_status: Some(...)` persisted to `state.json` after each worker (best-effort, same as executor.rs)
- [ ] `max_concurrency` respected (sessions don't all run simultaneously if `max_concurrency < session_count`)
- [ ] Both integration tests from T02 pass

## Verification

- `cargo test -p assay-core --features orchestrate -- mesh` — both `test_mesh_mode_message_routing` and `test_mesh_mode_completed_not_dead` pass
- `cargo test -p assay-core --features orchestrate` — all existing executor/DAG/pipeline tests still pass
- `cargo test -p assay-types --features orchestrate` — unchanged from T01

## Observability Impact

- Signals added/changed: `tracing::info!` per session launch (name, inbox_path); `tracing::debug!` per routed message (from, to, filename); `tracing::warn!` per unknown outbox target; `tracing::warn!` per session with `depends_on` (preserved from stub)
- How a future agent inspects this: `RUST_LOG=assay_core=debug cargo test -- mesh --nocapture` shows routing events; `state.json` at `.assay/orchestrator/<run_id>/state.json` contains `mesh_status.members` and `messages_routed` for post-run inspection
- Failure state exposed: `MeshMemberState::Dead` per session in `state.json` when runner returns error; `messages_routed == 0` when routing thread failed to route any messages

## Inputs

- `crates/assay-core/src/orchestrate/executor.rs` — `persist_state` (now `pub(crate)` after T01), `OrchestratorResult`, `OrchestratorConfig`, `panic::catch_unwind(AssertUnwindSafe(...))` pattern, `std::thread::scope` worker pattern
- `crates/assay-types/src/orchestrate.rs` (T01 output) — `MeshMemberState`, `MeshMemberStatus`, `MeshStatus`, `OrchestratorStatus` with `mesh_status`
- `crates/assay-types/src/harness.rs` — `PromptLayer`, `PromptLayerKind`
- `crates/assay-core/tests/mesh_integration.rs` (T02 output) — failing tests that define the contract; roster content format must produce parseable output for writer runner to find its outbox path
- S02-RESEARCH.md — `active_count` termination approach, outbox layout `outbox/<target_name>/<filename>`, heartbeat mtime approach (use mtime not content), `completed` sentinel file spec, `PromptLayerKind::System` at priority `-5`

## Expected Output

- `crates/assay-core/src/orchestrate/mesh.rs` — full implementation replacing stub; passes both integration tests; lint-clean
