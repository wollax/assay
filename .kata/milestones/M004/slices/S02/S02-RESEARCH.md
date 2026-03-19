# S02: Mesh Mode — Research

**Date:** 2026-03-17

## Summary

S02 replaces the `run_mesh()` stub in `assay-core::orchestrate::mesh` with a full implementation that: (1) launches all sessions in parallel via `std::thread::scope`, (2) injects a roster `PromptLayer` into each `ManifestSession` before invoking the runner, (3) creates `inbox/` and `outbox/` directories under `.assay/orchestrator/<run_id>/mesh/<name>/`, (4) runs a routing thread within the same `thread::scope` that polls outboxes and moves files to target inboxes, and (5) tracks SWIM-inspired membership (`Alive` / `Suspect` / `Dead`) by polling heartbeat files.

The core risk is **routing thread composition inside `thread::scope`**: the DAG executor's dispatch loop pattern (Mutex + Condvar) is the reference, but Mesh's routing thread needs to co-exist with session worker threads within the same scope lifetime. The solution is straightforward because there is no condvar dispatch loop in Mesh — all sessions launch immediately (no DAG ordering). The routing thread runs a simple `sleep-and-poll` loop until all workers complete, using a shared `Arc<AtomicBool>` for the "done" signal.

Three new types go in `assay-types::orchestrate`: `MeshMemberState`, `MeshMemberStatus`, and `MeshStatus`. `OrchestratorStatus` gains `mesh_status: Option<MeshStatus>` (additive, `serde(default, skip_serializing_if)`). Schema snapshots must be locked for all three new types and the updated `OrchestratorStatus`.

The integration test (`test_mesh_mode_message_routing`) uses mock session runners that write a JSON outbox file targeting a peer, then sleep briefly, allowing the routing thread to route the message. The test asserts `mesh_status.messages_routed == 1` and verifies the file exists in the peer inbox.

## Recommendation

**Implement the routing thread with `Arc<AtomicBool>` done-signal, not a Condvar.** The simplification over the DAG executor's condvar loop is intentional: in Mesh mode, all sessions start immediately and there is no dispatch loop — the routing thread just needs to run until "all done." `AtomicBool` is sufficient, cleaner, and avoids a complex multi-Condvar composition.

**Roster injection strategy:** build the roster `PromptLayer` in a pre-launch step (before spawning the `thread::scope`), then clone it into each `ManifestSession`'s `prompt_layers`. The roster is a static snapshot — it does not update mid-run. Use `PromptLayerKind::System` at priority `-5` (just above scope prompt convention of `-10` from D037).

**Heartbeat vs completion:** a session that completes normally writes a `completed` file to its mesh directory. The membership tracker checks: if `<name>/completed` exists → `Completed` (not Dead). Only sessions that are running (not completed) and have exceeded `suspect_timeout_secs` without a heartbeat update are classified as `Suspect`, and those past `dead_timeout_secs` as `Dead`. This avoids the tombstone problem (D problem statement in roadmap).

**State persistence:** persist a `mesh_state.json` alongside `state.json` under `.assay/orchestrator/<run_id>/`. The `orchestrate_status` MCP tool already reads `state.json` and `merge_report.json` — S04 will wire up `mesh_status` in the response wrapper. S02's job is just to write the data correctly. The full `OrchestratorStatus` (with `mesh_status` field) should be persisted to `state.json` on each routing poll, not a separate file, to keep `orchestrate_status` simple.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Atomic tempfile-then-rename for state persistence | `persist_state()` in `executor.rs` — uses `tempfile::NamedTempFile` | Already battle-tested; copy the pattern verbatim (or extract to a shared helper later) |
| Effective session naming (`name` field or `spec` fallback) | `DependencyGraph::name_of()` in `dag.rs` | Mesh doesn't use the DAG graph but needs the same name resolution — replicate the `session.name.as_deref().unwrap_or(&session.spec)` pattern directly, no graph needed |
| PromptLayer construction | `PromptLayer { kind, name, content, priority }` in `assay-types::harness` | Already established type; no new abstraction needed |
| Parallel dispatch skeleton | `run_orchestrated()` in `executor.rs` — `std::thread::scope` + worker closure | The scope pattern and `panic::catch_unwind(AssertUnwindSafe(...))` wrapper are the reference — use the same structure for session workers |
| Ulid run_id generation | `ulid::Ulid::new().to_string()` — used throughout orchestrator | Consistent with all existing executors |

## Existing Code and Patterns

- `crates/assay-core/src/orchestrate/mesh.rs` — Current stub to replace; signature `run_mesh<F>(manifest, config, pipeline_config, session_runner)` is fixed (D052). Do not change the signature.
- `crates/assay-core/src/orchestrate/executor.rs` — Reference implementation for: (1) `persist_state()` tempfile-rename pattern, (2) `std::thread::scope` with worker closures, (3) `panic::catch_unwind(AssertUnwindSafe(...))` wrapping, (4) constructing `OrchestratorResult`. Mesh is simpler — no condvar dispatch loop, no DAG graph.
- `crates/assay-types/src/orchestrate.rs` — Where `MeshMemberState`, `MeshMemberStatus`, `MeshStatus` go. Pattern: full `derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)` + `deny_unknown_fields` on structs + `inventory::submit!` for schema registry. Enum variants use `serde(rename_all = "snake_case")`. All new orchestrate types live in this file.
- `crates/assay-types/src/lib.rs` — Re-exports under `#[cfg(feature = "orchestrate")]`; add `MeshMemberState`, `MeshMemberStatus`, `MeshStatus` here.
- `crates/assay-types/tests/schema_snapshots.rs` — Add snapshot tests for `MeshMemberState`, `MeshMemberStatus`, `MeshStatus`, and updated `OrchestratorStatus` (if its schema changes due to new optional field). Run `cargo test -p assay-types --features orchestrate -- --nocapture` to regenerate, then commit snapshots.
- `crates/assay-types/src/harness.rs` — `PromptLayer` and `PromptLayerKind` for roster injection. Pattern from D037: `PromptLayerKind::System` at priority `-10` for scope prompt. Roster layer can use `-5` (higher priority, renders before scope in assembly).
- `crates/assay-core/tests/orchestrate_integration.rs` — Reference for integration test structure: `setup_git_repo()`, mock session runner closures, `make_pipeline_config()`. The mesh integration test follows this pattern but does NOT need a real git repo — it only needs a temp dir with `.assay/` structure (no git ops in Mesh mode).
- `crates/assay-mcp/src/server.rs` — The `orchestrate_status` handler reads `state.json` and wraps it in `OrchestrateStatusResponse { status, merge_report }`. S04 extends this to also read `mesh_status`. S02 just needs to persist `OrchestratorStatus` (with `mesh_status` populated) to `state.json`.

## Constraints

- **Signature is fixed**: `run_mesh<F>(manifest: &RunManifest, config: &OrchestratorConfig, pipeline_config: &PipelineConfig, session_runner: &F)` — no changes allowed (D052).
- **`session_runner` generic bound**: `F: Fn(&ManifestSession, &PipelineConfig) -> Result<PipelineResult, PipelineError> + Sync`. The `Sync` bound is required for sharing across threads. Roster layer must be injected by cloning the `ManifestSession` and adding to `prompt_layers` before calling the runner — do not mutate the manifest.
- **OrchestratorStatus has `deny_unknown_fields`**: adding `mesh_status: Option<MeshStatus>` with `serde(default, skip_serializing_if = "Option::is_none")` is backward-compatible; existing `state.json` files without this field will deserialize correctly (the field defaults to `None`). The schema snapshot will change — must be regenerated and re-locked.
- **No async**: sync threading only (D017). The routing thread uses `std::thread::sleep` for polling, not tokio. All file I/O is `std::fs`.
- **`deny_unknown_fields` on all new persisted types**: `MeshMemberStatus`, `MeshStatus`, `MeshConfig` already has it. `MeshMemberState` is an enum — no `deny_unknown_fields` on enums in serde.
- **Heartbeat file format**: Mesh sessions write a heartbeat file at `.assay/orchestrator/<run_id>/mesh/<name>/heartbeat` containing an ISO 8601 timestamp (or just the mtime is checked). Routing thread checks mtime, not content, to avoid parse errors from partially-written files.
- **Message format**: unstructured — the routing layer passes files through without interpretation. Message files in outbox have filenames of the form `<target_session_name>/<message_filename>` — i.e., the first path component is the target. Or simpler: files in `outbox/<target_name>/` subdirectory. The roadmap says "routes outbox messages to target inboxes" but doesn't prescribe the file naming — pick the simplest scheme and document it.
- **`max_concurrency` applies**: `OrchestratorConfig.max_concurrency` caps parallel sessions even in Mesh mode (S01 forward intelligence).
- **`completed` file sentinel**: session writes `.assay/orchestrator/<run_id>/mesh/<name>/completed` on exit (after `session_runner` returns). The executor, not the session_runner, writes this — the runner is a black box. The executor writes the sentinel after the `scope.spawn` closure returns.
- **`assay-core` does not depend on `assay-harness`**: roster injection must be done by cloning `ManifestSession` and pushing to `prompt_layers` in `mesh.rs` directly — do not import `assay-harness`. This is pure type construction using `assay-types`.

## Common Pitfalls

- **`thread::scope` closure captures**: the routing thread and session worker threads both need references to the `run_dir`, `run_id`, and shared state. Put all shared state in `Arc<>` or borrow it with explicit lifetimes from variables declared before `thread::scope`. The `done: Arc<AtomicBool>` should be constructed before entering the scope.
- **Modifying `ManifestSession` for roster**: `ManifestSession` doesn't implement `Copy`, and the manifest is borrowed. Build a `Vec<ManifestSession>` of clones with the roster layer appended before entering the scope — iterate over these clones in the session workers rather than `&manifest.sessions[idx]`. Worker closure captures `&session_clone` by reference from the pre-built vec.
- **Message routing loop target resolution**: the simplest scheme is `outbox/<target_name>/<filename>` — the routing thread reads subdirectory names as target session names and looks them up in a pre-built `name → inbox_path` map. If target name doesn't exist, log a warning and leave the file in place.
- **Heartbeat mtime race**: checking `metadata().modified()` has second-level precision on some platforms. Use `SystemTime::elapsed()` as a rough bound; do not assume sub-second accuracy. Default `heartbeat_interval_secs = 5` and `suspect_timeout_secs = 10` from `MeshConfig` means the window is 2 missed heartbeats — coarse but sufficient.
- **Schema snapshot `OrchestratorStatus` change**: `OrchestratorStatus` has `deny_unknown_fields` and a locked snapshot. Adding `mesh_status: Option<MeshStatus>` changes the schema. The snapshot test will fail until re-run with `INSTA_UPDATE=always` or `cargo insta review`. This is expected and must be done as part of S02.
- **Persisting `OrchestratorStatus` with `mesh_status`**: the `persist_state()` function in `executor.rs` takes `&OrchestratorStatus`. In `mesh.rs`, replicate or call the same pattern. Keeping `persist_state` as a module-private function in `executor.rs` means mesh.rs must have its own copy or the function must be made `pub(crate)`. Making it `pub(crate)` in `executor.rs` is the cleaner choice.
- **Routing thread deadlock**: the routing thread must exit when all sessions complete. Use `Arc<AtomicBool>` set to `true` after the scope's session workers all return. The routing thread checks this flag at the start of each poll loop. Since `thread::scope` only returns after all spawned threads complete, the "done" signal can actually be set outside the scope — but the routing thread is also inside the scope, so it must self-terminate. Set the flag inside a `drop(guard)` call after the last session outcome is recorded, or simply check a counter.

## Open Risks

- **`deny_unknown_fields` on `OrchestratorStatus` + new field**: the `serde(default, skip_serializing_if)` pattern is established (confirmed for `completed_at`, `resolutions`). The concern is whether schemars generates the correct schema (optional field with no required constraint). Verify by running the snapshot test after adding the field — if the snapshot changes unexpectedly, audit the generated JSON Schema.
- **Routing thread inside `thread::scope` with no condvar**: the routing thread blocks on `thread::sleep(poll_interval)` rather than being woken by a condvar. This is simpler but means the thread can't be woken early when sessions all complete — it sleeps for one full poll interval after the last session finishes before checking `done`. Given `heartbeat_interval_secs = 5` default, this adds ≤5s to total wall clock. Accept this — it's a prototype-level implementation.
- **Mock runner in integration test writes outbox files**: the mock runner must create the inbox/outbox directories first (or the executor creates them before launching). Confirm that the executor creates directories before calling the session runner — otherwise the mock can't write to a non-existent outbox.
- **MeshStatus in `state.json` grows with member count**: for 10 sessions, this is negligible. No concern at this scale.
- **Mutex around session roster construction**: the roster `PromptLayer` content embeds all peer names and inbox paths. Building it requires knowing all session names up front — this is fine since `manifest.sessions` is fully known before launch. No concurrency concern here.

## New Types Required (S02)

### In `assay-types::orchestrate`

```rust
// Membership state for a single peer in the mesh
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MeshMemberState { Alive, Suspect, Dead, Completed }

// Per-member status snapshot
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MeshMemberStatus {
    pub name: String,
    pub state: MeshMemberState,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
}

// Mode-specific status for the mesh run
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MeshStatus {
    pub members: Vec<MeshMemberStatus>,
    pub messages_routed: u64,
}
```

### Extension to `OrchestratorStatus`

```rust
// New field on OrchestratorStatus (additive — won't break existing state.json):
#[serde(default, skip_serializing_if = "Option::is_none")]
pub mesh_status: Option<MeshStatus>,
```

Note: `OrchestratorStatus` has `deny_unknown_fields`. Adding a `serde(default)` field to a `deny_unknown_fields` struct is safe — serde accepts fields with `default` even if they're missing from the JSON (this is the whole point of `serde(default)`). Existing `state.json` files deserialize fine.

## Directory Structure

```
.assay/orchestrator/<run_id>/
  state.json                         ← OrchestratorStatus (extended with mesh_status)
  mesh/
    <session_name>/
      inbox/                         ← routed messages arrive here
      outbox/
        <target_name>/               ← session writes files here; target_name is peer name
      heartbeat                      ← updated by session runner (agent writes this; executor watches)
      completed                      ← written by executor after session_runner returns
```

## Integration Test Structure

```rust
// tests/mesh_integration.rs or inline in orchestrate_integration.rs
// Two sessions: "writer" writes an outbox file to "reader", then sleeps 200ms.
// Routing thread polls every 50ms and routes it.
// After both sessions complete, assert:
//   - .../mesh/reader/inbox/<filename> exists
//   - state.json mesh_status.messages_routed == 1
//   - state.json mesh_status.members[*].state is Alive or Completed for all
```

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust (std::thread::scope, AtomicBool, file I/O) | — | No skill needed; core std |

## Sources

- `executor.rs` — threading model, persist_state pattern, panic::catch_unwind usage (source: local codebase)
- `dag.rs` — session effective name resolution, ready_set pattern (source: local codebase)
- `orchestrate.rs` — existing type patterns (deny_unknown_fields, inventory::submit!, schema snapshots) (source: local codebase)
- S01 Summary — stub signature, Default::default() pattern, MeshConfig defaults (source: `.kata/milestones/M004/slices/S01/S01-SUMMARY.md`)
- M004 Roadmap — S02→S04 boundary map, exact type names and field shapes for MeshMemberState/MeshMemberStatus/MeshStatus (source: `.kata/milestones/M004/M004-ROADMAP.md`)
