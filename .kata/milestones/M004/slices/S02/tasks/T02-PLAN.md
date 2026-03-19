---
estimated_steps: 4
estimated_files: 1
---

# T02: Write failing integration test for mesh mode

**Slice:** S02 — Mesh Mode
**Milestone:** M004

## Description

Create `crates/assay-core/tests/mesh_integration.rs` with two integration tests that define the observable behavior of Mesh mode at the filesystem level. The tests must compile but fail — the stub `run_mesh()` doesn't route messages or produce `mesh_status` data. Failing tests prove the tests are correct and the stub is the blocker, not a test bug.

Both tests use only a temp directory (no git repo — Mesh mode has no git operations) and mock session runners that interact with the mesh directory structure.

## Steps

1. Create `crates/assay-core/tests/mesh_integration.rs` with the required imports and helper functions:
   ```rust
   use std::sync::Arc;
   use std::sync::atomic::{AtomicBool, Ordering};
   use assay_core::orchestrate::executor::{OrchestratorConfig, OrchestratorResult};
   use assay_core::orchestrate::mesh::run_mesh;
   use assay_types::{ManifestSession, OrchestratorMode, RunManifest};
   use assay_types::orchestrate::{OrchestratorStatus, MeshMemberState};
   use assay_core::pipeline::{PipelineConfig, PipelineError, PipelineResult};
   ```
   Helper `make_pipeline_config(tmp: &Path) -> PipelineConfig` — sets project_root and assay_dir to tmp (same pattern as orchestrate_integration.rs). Helper `make_mesh_manifest(names: &[&str]) -> RunManifest` — builds sessions with `mode: OrchestratorMode::Mesh`, no depends_on.

2. Write `test_mesh_mode_message_routing`:
   - Create manifest with 2 sessions: name "writer" (spec "writer"), name "reader" (spec "reader")
   - Mock runner captures `run_dir` via `Arc<Mutex<Option<PathBuf>>>` (share `run_id` is unknown at test setup time — use an `AtomicBool` written flag instead):
     - For "writer": look up its mesh outbox dir via `pipeline_config.assay_dir.join("orchestrator")`, glob for `run_id`, create `outbox/reader/msg.txt` in the first run dir found; sleep 200ms
     - **Simpler approach**: runners receive `pipeline_config`; the executor creates the mesh dirs before calling the runner (per implementation plan); so runner can write to `pipeline_config.assay_dir.join("orchestrator").join(&run_id).join("mesh/writer/outbox/reader/msg.txt")` — but it doesn't know run_id.
     - **Correct approach**: runner receives `ManifestSession` and `PipelineConfig`; the mesh inbox/outbox paths under `run_id` are created by the executor before runner invocation; the runner can discover them by reading the session's prompt_layers (the roster layer contains the inbox path). Parse `session.prompt_layers` for the "mesh-roster" layer and extract the outbox path from its content, or — more practically — trust the executor to have created `assay_dir/orchestrator/<run_id>/mesh/writer/outbox/` and find the run_id by listing `assay_dir/orchestrator/`.
     - **Simplest approach**: since the executor creates dirs and injects the roster (which includes the session's own outbox path), the runner reads the roster content from `session.prompt_layers` to find its outbox dir. Look for a prompt layer named "mesh-roster" and parse the outbox path from its content (the roster content will include a line like `Your outbox: <path>`).
   - After `run_mesh()` returns: read `state.json`, deserialize `OrchestratorStatus`, assert `mesh_status.is_some()`, assert `mesh_status.unwrap().messages_routed >= 1`, assert `reader`'s inbox dir contains exactly one file
   
   Concretely: implement writer runner to parse the roster layer and write `outbox/reader/msg.txt` from the outbox path; reader runner just sleeps 300ms. The roster layer content will include the session's outbox path — parse it by looking for `Outbox: <path>` line (implementation must produce this format).

3. Write `test_mesh_mode_completed_not_dead`:
   - 2 sessions ("alpha", "beta"), both mock runners return `Ok(success_result)` immediately
   - After `run_mesh()` returns: read and deserialize `state.json`; assert `mesh_status.is_some()`; for each member in `mesh_status.unwrap().members` assert `member.state == MeshMemberState::Completed`
   - No file routing involved — purely tests that the membership classifier treats normally-exited sessions as Completed

4. Compile-check: `cargo test -p assay-core --features orchestrate -- mesh 2>&1 | head -50` — expect compilation success and test failures (assertion errors), NOT compile errors. Fix any compile errors before moving on.

## Must-Haves

- [ ] Both tests compile without errors
- [ ] Both tests fail (assertion failures, not panics or compile errors) when run against the stub
- [ ] `test_mesh_mode_message_routing` asserts both: (a) inbox file exists AND (b) `messages_routed >= 1`
- [ ] `test_mesh_mode_completed_not_dead` asserts `MeshMemberState::Completed` for all members
- [ ] No git repo setup needed — tests work with a bare temp dir

## Verification

- `cargo build -p assay-core --features orchestrate --tests` — must succeed (compile check)
- `cargo test -p assay-core --features orchestrate -- mesh 2>&1` — must show FAILED (not compile error), with failure output showing assertion mismatch

## Observability Impact

- Signals added/changed: None — tests are passive consumers of `state.json` and filesystem state
- How a future agent inspects this: Run `cargo test -p assay-core --features orchestrate -- mesh -- --nocapture` to see full test output including which assertions failed; state.json inspection via `cat .assay/orchestrator/<run_id>/state.json | jq .mesh_status`
- Failure state exposed: Test output clearly identifies whether routing failed (messages_routed == 0) or membership classification failed (member.state != completed) or directory creation failed (inbox dir doesn't exist)

## Inputs

- `crates/assay-core/tests/orchestrate_integration.rs` — reference for make_pipeline_config pattern, mock runner closure signature, ManifestSession construction
- `crates/assay-core/src/orchestrate/mesh.rs` — `run_mesh()` signature (must match exactly: `(&RunManifest, &OrchestratorConfig, &PipelineConfig, &F)`)
- `crates/assay-types/src/orchestrate.rs` (T01 output) — `MeshMemberState`, `MeshStatus`, `OrchestratorStatus` with `mesh_status` field
- S02-RESEARCH.md — integration test structure guidance, roster layer format, outbox directory layout (`outbox/<target_name>/<filename>`)

## Expected Output

- `crates/assay-core/tests/mesh_integration.rs` — new file with 2 failing integration tests that compile cleanly
