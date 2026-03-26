---
estimated_steps: 5
estimated_files: 8
---

# T03: Wire Arc<dyn StateBackend> into OrchestratorConfig and replace all persist_state callsites

**Slice:** S02 — LocalFsBackend implementation and orchestrator wiring
**Milestone:** M010

## Description

The central wiring task. Adds `backend: Arc<dyn StateBackend>` to `OrchestratorConfig`, replaces all 15 `persist_state()` callsites in executor.rs/mesh.rs/gossip.rs with `config.backend.push_session_event()`, updates all test files that construct `OrchestratorConfig` to pass a `LocalFsBackend` instance, and verifies all existing integration tests still pass.

## Steps

1. Modify `OrchestratorConfig` in `executor.rs`:
   - Add `pub backend: std::sync::Arc<dyn crate::state_backend::StateBackend>` field.
   - Remove `#[derive(Clone)]` and add a manual `Clone` impl that clones the `Arc`.
   - Update `Default` impl: use `Arc::new(LocalFsBackend::new(PathBuf::from(".assay")))` as default backend (or remove `Default` and fix all callsites — audit which is less disruptive). Given ~20 `::default()` usages, keeping Default with a temporary path is likely better.
   - Add `use std::sync::Arc;` and `use crate::state_backend::LocalFsBackend;` imports.

2. Replace all `persist_state()` callsites in `executor.rs` (5 sites):
   - Each `persist_state(&run_dir, &status)` becomes `config.backend.push_session_event(&run_dir, &status)` (or `let _ = config.backend.push_session_event(...)` where the error was already ignored).
   - For callsites inside `thread::scope` workers, ensure `config.backend` (the `Arc`) is accessible — it should be since `config` is passed by reference and `Arc` is `Clone + Send + Sync`.

3. Replace all `persist_state()` callsites in `mesh.rs` (4 sites):
   - Remove `use crate::orchestrate::executor::persist_state` from mesh.rs imports.
   - Replace each callsite with `config.backend.push_session_event()`.
   - For the mesh routing thread, clone the `Arc` from `config.backend` before entering the thread closure.

4. Replace all `persist_state()` callsites in `gossip.rs` (6 sites):
   - Remove `use crate::orchestrate::executor::persist_state` from gossip.rs imports (keep other imports).
   - Replace each callsite with `config.backend.push_session_event()`.
   - For the gossip coordinator thread, clone the `Arc<dyn StateBackend>` into the thread closure.

5. Update all test files that construct `OrchestratorConfig`:
   - `crates/assay-core/tests/orchestrate_integration.rs` — 3 struct-literal sites
   - `crates/assay-core/tests/mesh_integration.rs` — 2 `::default()` sites
   - `crates/assay-core/tests/gossip_integration.rs` — 2 `::default()` sites
   - `crates/assay-core/tests/orchestrate_spans.rs` — 4 `::default()` sites
   - `crates/assay-core/tests/integration_modes.rs` — 3 `::default()` sites
   - Embedded tests in `executor.rs` (~12 sites), `mesh.rs` (~4 sites), `gossip.rs` (~1 site)
   - For `::default()` sites: if Default is kept, no change needed. For struct-literal sites: add `backend: Arc::new(LocalFsBackend::new(assay_dir.clone()))` where `assay_dir` is already available, or `..Default::default()` for the backend field.

## Must-Haves

- [ ] `OrchestratorConfig.backend: Arc<dyn StateBackend>` exists
- [ ] `OrchestratorConfig` is still `Clone` (via manual impl or `Arc`)
- [ ] All 15 `persist_state()` callsites replaced with `config.backend.push_session_event()`
- [ ] `persist_state()` no longer `pub(crate)` — either removed or made private
- [ ] `Arc<dyn StateBackend>` properly cloned into `thread::scope` workers in mesh/gossip
- [ ] All integration tests pass: `orchestrate_integration`, `mesh_integration`, `gossip_integration`, `orchestrate_spans`, `integration_modes`
- [ ] All embedded tests in executor.rs, mesh.rs, gossip.rs pass

## Verification

- `cargo test -p assay-core --features orchestrate --test orchestrate_integration` — passes
- `cargo test -p assay-core --features orchestrate --test mesh_integration` — passes
- `cargo test -p assay-core --features orchestrate --test gossip_integration` — passes
- `cargo test -p assay-core --features orchestrate --test orchestrate_spans` — passes
- `cargo test -p assay-core --features orchestrate --test integration_modes` — passes
- `cargo test -p assay-core --features orchestrate` — all crate tests pass (including embedded)
- `grep -rn "persist_state" crates/assay-core/src/orchestrate/` — only appears in `executor.rs` as a private fn or not at all

## Observability Impact

- Signals added/changed: `persist_state` replaced by `backend.push_session_event` — same atomic write behavior, now routed through trait object
- How a future agent inspects this: `grep persist_state` confirms no direct filesystem writes remain outside the backend; backend errors surface as `AssayError` in orchestrator logs
- Failure state exposed: Backend write failures propagate via `Result` at each callsite (same error handling as before)

## Inputs

- T02 output: `LocalFsBackend` with real method bodies
- `crates/assay-core/src/orchestrate/executor.rs` — `OrchestratorConfig`, `persist_state`, `run_orchestrated`
- `crates/assay-core/src/orchestrate/mesh.rs` — `run_mesh`, 4 `persist_state` callsites
- `crates/assay-core/src/orchestrate/gossip.rs` — `run_gossip`, 6 `persist_state` callsites

## Expected Output

- `crates/assay-core/src/orchestrate/executor.rs` — `OrchestratorConfig` with `backend: Arc<dyn StateBackend>`, `persist_state` removed or private
- `crates/assay-core/src/orchestrate/mesh.rs` — all callsites use `config.backend`
- `crates/assay-core/src/orchestrate/gossip.rs` — all callsites use `config.backend`
- All integration test files updated with backend field
- Zero regressions in existing test suite
