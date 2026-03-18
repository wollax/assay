---
estimated_steps: 6
estimated_files: 5
---

# T03: Add run_mesh/run_gossip stubs, wire mod.rs, dispatch in CLI and MCP

**Slice:** S01 — Mode Infrastructure
**Milestone:** M004

## Description

Create the `mesh.rs` and `gossip.rs` stub executor modules in `assay-core::orchestrate`, declare them in `mod.rs`, and wire mode dispatch at both call sites (CLI `execute()` and MCP `orchestrate_run`). This closes the slice by making `mode = "mesh"` and `mode = "gossip"` actually compile and route — completing R034.

The stubs accept the same `(manifest, config, pipeline_config, session_runner: &F)` signature as `run_orchestrated()` per D052. They emit `tracing::warn!` for sessions with non-empty `depends_on` (D053) and return a valid `OrchestratorResult` with zero outcomes. The MCP multi-session guard must be conditioned on `mode == Dag` so single-session Mesh/Gossip manifests are not rejected.

## Steps

1. Create `crates/assay-core/src/orchestrate/mesh.rs`:
   ```rust
   use std::time::Duration;
   use assay_types::{ManifestSession, OrchestratorMode};
   use assay_types::orchestrate::FailurePolicy;
   use crate::error::AssayError;
   use crate::orchestrate::executor::{OrchestratorConfig, OrchestratorResult, SessionOutcome};
   use crate::pipeline::{PipelineConfig, PipelineError, PipelineResult};
   use ulid::Ulid;

   pub fn run_mesh<F>(
       manifest: &assay_types::RunManifest,
       config: &OrchestratorConfig,
       _pipeline_config: &PipelineConfig,
       _session_runner: &F,
   ) -> Result<OrchestratorResult, AssayError>
   where
       F: Fn(&ManifestSession, &PipelineConfig) -> Result<PipelineResult, PipelineError> + Sync,
   {
       for session in &manifest.sessions {
           if !session.depends_on.is_empty() {
               tracing::warn!(
                   session = session.name.as_deref().unwrap_or(&session.spec),
                   "depends_on is ignored in Mesh mode"
               );
           }
       }
       Ok(OrchestratorResult {
           run_id: Ulid::new().to_string(),
           outcomes: vec![],
           duration: Duration::ZERO,
           failure_policy: config.failure_policy,
       })
   }
   ```
   Create `gossip.rs` with the identical structure, swapping `Mesh` for `Gossip` in the warning message.

2. In `crates/assay-core/src/orchestrate/mod.rs`, add `pub mod mesh;` and `pub mod gossip;`.

3. In `crates/assay-cli/src/commands/run.rs`:
   - Add `use assay_types::OrchestratorMode;` import.
   - In `execute()`, before the `needs_orchestration()` check, add:
     ```rust
     match manifest.mode {
         OrchestratorMode::Mesh => return execute_mesh(cmd, &manifest, &pipeline_config),
         OrchestratorMode::Gossip => return execute_gossip(cmd, &manifest, &pipeline_config),
         OrchestratorMode::Dag => {} // fall through to existing logic
     }
     ```
   - Add `execute_mesh()` and `execute_gossip()` private functions that call the stubs via `spawn_blocking` (same pattern as `execute_orchestrated`), format a minimal `OrchestrationResponse` (empty sessions, zero summary), and return `Ok(0)`.
   - Add a unit test: `mode_mesh_bypasses_needs_orchestration` — construct a `RunManifest` with `mode: Mesh` and a single session with no `depends_on`, verify `needs_orchestration(&manifest)` returns false but the mode match would route to `execute_mesh` (test the dispatch logic, not the execution).

4. In `crates/assay-mcp/src/server.rs`:
   - Add `use assay_types::OrchestratorMode;` import (it's already available via `assay_types` feature).
   - Change the multi-session guard from:
     ```rust
     if manifest.sessions.len() < 2 && !has_deps {
     ```
     to:
     ```rust
     if manifest.mode == OrchestratorMode::Dag && manifest.sessions.len() < 2 && !has_deps {
     ```
   - After the existing DAG routing, add `Mesh` and `Gossip` branches that call `run_mesh()` / `run_gossip()` stubs (via `spawn_blocking`), serialize a minimal response, and return success.
   - Add a unit test: `orchestrate_run_mesh_skips_session_count_guard` — a manifest with `mode = "mesh"` and a single session should NOT return the "must contain multiple sessions" error.

5. Run `just build` — verify compilation succeeds with zero errors.

6. Run `just ready` — fix any fmt/lint issues. Confirm 0 warnings.

## Must-Haves

- [ ] `mesh.rs` and `gossip.rs` exist in `crates/assay-core/src/orchestrate/` and compile
- [ ] Both stubs emit `tracing::warn!` per session with non-empty `depends_on`
- [ ] Both stubs return valid `OrchestratorResult` with zero outcomes
- [ ] `pub mod mesh; pub mod gossip;` declared in `mod.rs`
- [ ] CLI `execute()` has `match manifest.mode { Mesh => ..., Gossip => ..., Dag => {} }` before `needs_orchestration()` check
- [ ] MCP multi-session guard conditioned on `mode == OrchestratorMode::Dag`
- [ ] At least one unit test per call site (CLI mode dispatch, MCP guard bypass) proves the routing works
- [ ] `just ready` green with 0 warnings

## Verification

- `just build` — compiles cleanly
- `cargo test -p assay-core --features orchestrate` — mesh/gossip module tests pass (or no test failures if stubs have no dedicated tests — at minimum they compile)
- `cargo test -p assay-cli` — mode dispatch unit test passes + all existing CLI tests pass
- `cargo test -p assay-mcp` — guard-bypass unit test passes + all existing MCP tests pass (1222+ total)
- `just ready` — fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓

## Observability Impact

- Signals added/changed: `tracing::warn!` emitted per session with `depends_on` when mode is Mesh/Gossip — surfaced in runtime logs and test output
- How a future agent inspects this: `RUST_LOG=warn cargo test` will show the warn message if a test exercises a Mesh/Gossip session with `depends_on`; the stub returns `OrchestratorResult` with `run_id` that can be passed to `orchestrate_status`
- Failure state exposed: If stub panics (shouldn't), the `spawn_blocking` boundary will surface it as an `Err` at the call site

## Inputs

- T01 output: `OrchestratorMode`, `MeshConfig`, `GossipConfig` in `assay-types`
- T02 output: `RunManifest.mode` field available
- `crates/assay-core/src/orchestrate/executor.rs` — `OrchestratorConfig`, `OrchestratorResult` types and signature pattern to follow
- `crates/assay-cli/src/commands/run.rs` — existing `execute()`, `execute_orchestrated()` for dispatch pattern
- `crates/assay-mcp/src/server.rs` — existing `orchestrate_run` handler for multi-session guard location

## Expected Output

- `crates/assay-core/src/orchestrate/mesh.rs` (new) — `run_mesh()` stub
- `crates/assay-core/src/orchestrate/gossip.rs` (new) — `run_gossip()` stub
- `crates/assay-core/src/orchestrate/mod.rs` — `pub mod mesh; pub mod gossip;` added
- `crates/assay-cli/src/commands/run.rs` — mode match dispatch + `execute_mesh()`/`execute_gossip()` stubs + unit test
- `crates/assay-mcp/src/server.rs` — guard conditioned on `Dag`, Mesh/Gossip routing + unit test
